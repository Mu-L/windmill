<script lang="ts">
	import { Button } from '$lib/components/common'
	import { faUser } from '@fortawesome/free-solid-svg-icons'
	import { getContext } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import type RunnableComponent from '../helpers/RunnableComponent.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	import { concatCustomCss } from '../../utils'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import AlwaysMountedModal from '$lib/components/common/modal/AlwaysMountedModal.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let recomputeIds: string[] | undefined = undefined
	export let extraQueryParams: Record<string, any> = {}
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'formbuttoncomponent'> | undefined = undefined
	export let render: boolean
	export let errorHandledByComponent: boolean | undefined = false

	$: errorHandledByComponent = resolvedConfig?.onError?.selected !== 'errorOverlay'

	const { app, worldStore, componentControl } = getContext<AppViewerContext>('AppViewerContext')

	$componentControl[id] = {
		onDelete: () => {
			modal?.close()
		}
	}

	let outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let resolvedConfig = initConfig(
		components['formbuttoncomponent'].initialData.configuration,
		configuration
	)
	let runnableComponent: RunnableComponent

	let errors: Record<string, string> = {}

	$: errorsMessage = Object.values(errors)
		.filter((x) => x != '')
		.join('\n')

	$: noInputs =
		componentInput?.type != 'runnable' || Object.keys(componentInput?.fields ?? {}).length == 0

	$: if (outputs?.loading != undefined) {
		outputs.loading.set(false, true)
	}

	$: css = concatCustomCss($app?.css?.formbuttoncomponent, customCss)
	let runnableWrapper: RunnableWrapper
	let loading = false
	let modal: AlwaysMountedModal
</script>

{#each Object.keys(components['formbuttoncomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<AlwaysMountedModal title={resolvedConfig.label ?? ''} bind:this={modal}>
	<RunnableWrapper
		bind:this={runnableWrapper}
		{recomputeIds}
		{render}
		bind:runnableComponent
		{componentInput}
		{id}
		{extraQueryParams}
		autoRefresh={false}
		forceSchemaDisplay={true}
		runnableClass="!block"
		{outputs}
		doOnSuccess={resolvedConfig.onSuccess}
		doOnError={resolvedConfig.onError}
		{errorHandledByComponent}
	>
		<div class="flex flex-col gap-2 px-4 w-full">
			<div>
				{#if noInputs}
					<div class="text-gray-600 italic text-sm my-4">
						Run forms are associated with a runnable that has user inputs.
						<br />
						Once a script or flow is chosen, set some <strong>Runnable Inputs</strong> to
						<strong>
							User Input
							<Icon data={faUser} scale={1.3} class="rounded-sm bg-gray-200 p-1 ml-0.5" />
						</strong>
					</div>
				{/if}
			</div>
			<div class="flex justify-end">
				<Button
					{loading}
					btnClasses="my-1"
					on:pointerdown={(e) => {
						e?.stopPropagation()
					}}
					on:click={async () => {
						if (!runnableComponent) {
							runnableWrapper?.onSuccess()
						} else {
							await runnableComponent?.runComponent()
						}
						modal?.close()
					}}
					size="xs"
					color="dark"
				>
					Submit
				</Button>
			</div>
		</div>
	</RunnableWrapper>
</AlwaysMountedModal>

<AlignWrapper {horizontalAlignment} {verticalAlignment}>
	{#if errorsMessage}
		<div class="text-red-500 text-xs">{errorsMessage}</div>
	{/if}
	<Button
		disabled={resolvedConfig.disabled ?? false}
		size={resolvedConfig.size ?? 'md'}
		color={resolvedConfig.color}
		btnClasses={css?.button?.class ?? ''}
		style={css?.button?.style ?? ''}
		on:click={(e) => {
			modal?.open()
		}}
	>
		{resolvedConfig.label}
	</Button>
</AlignWrapper>
