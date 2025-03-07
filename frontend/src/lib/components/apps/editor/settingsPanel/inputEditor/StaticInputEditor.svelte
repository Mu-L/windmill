<script lang="ts">
	import type { InputType, StaticInput, StaticOptions } from '../../../inputType'
	import ArrayStaticInputEditor from '../ArrayStaticInputEditor.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import JsonEditor from './JsonEditor.svelte'
	import { getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '$lib/components/apps/types'
	import IconSelectInput from './IconSelectInput.svelte'
	import ColorInput from './ColorInput.svelte'
	import TabSelectInput from './TabSelectInput.svelte'
	import { DollarSign } from 'lucide-svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import SchemaEditor from '$lib/components/SchemaEditor.svelte'

	export let componentInput: StaticInput<any> | undefined
	export let fieldType: InputType | undefined = undefined
	export let subFieldType: InputType | undefined = undefined
	export let selectOptions: StaticOptions['selectOptions'] | undefined = undefined
	export let placeholder: string | undefined = undefined

	export let format: string | undefined = undefined
	export let noVariablePicker: boolean = false

	const { onchange } = getContext<AppViewerContext>('AppViewerContext')
	const { pickVariableCallback } = getContext<AppEditorContext>('AppEditorContext')

	$: componentInput && onchange?.()
</script>

{#if componentInput?.type === 'static'}
	{#if fieldType === 'number' || fieldType === 'integer'}
		<input on:keydown|stopPropagation type="number" bind:value={componentInput.value} />
	{:else if fieldType === 'textarea'}
		<textarea on:keydown|stopPropagation bind:value={componentInput.value} />
	{:else if fieldType === 'date'}
		<input on:keydown|stopPropagation type="date" bind:value={componentInput.value} />
	{:else if fieldType === 'boolean'}
		<Toggle bind:checked={componentInput.value} size="xs" />
	{:else if fieldType === 'select' && selectOptions}
		<select on:keydown|stopPropagation on:keydown|stopPropagation bind:value={componentInput.value}>
			{#each selectOptions ?? [] as option}
				{#if typeof option == 'string'}
					<option value={option}>
						{option}
					</option>
				{:else}
					<option value={option.value}>
						{option.label}
					</option>
				{/if}
			{/each}
		</select>
	{:else if fieldType === 'icon-select'}
		<IconSelectInput bind:componentInput />
	{:else if fieldType === 'tab-select'}
		<TabSelectInput bind:componentInput />
	{:else if fieldType === 'labeledresource'}
		{#if componentInput?.value && typeof componentInput?.value == 'object' && 'label' in componentInput?.value}
			<div class="flex flex-col gap-1 w-full">
				<input
					on:keydown|stopPropagation
					placeholder="Label"
					type="text"
					bind:value={componentInput.value['label']}
				/>
				<ResourcePicker
					initialValue={componentInput.value?.['value']?.split('$res:')[1] || ''}
					on:change={(e) => {
						let path = e.detail
						if (componentInput) {
							if (path) {
								componentInput.value['value'] = `$res:${path}`
							} else {
								componentInput.value['value'] = undefined
							}
						}
					}}
				/>
			</div>
		{:else}
			Inconsistent labeled resource object
		{/if}
	{:else if fieldType === 'color'}
		<ColorInput bind:value={componentInput.value} />
	{:else if fieldType === 'object' || fieldType == 'labeledselect'}
		{#if format?.startsWith('resource-')}
			<ResourcePicker
				initialValue={componentInput.value?.split('$res:')[1] || ''}
				on:change={(e) => {
					let path = e.detail
					if (componentInput) {
						if (path) {
							componentInput.value = `$res:${path}`
						} else {
							componentInput.value = undefined
						}
					}
				}}
				resourceType={format.split('-').length > 1
					? format.substring('resource-'.length)
					: undefined}
			/>
		{:else}
			<div class="flex w-full flex-col">
				<JsonEditor
					bind:value={componentInput.value}
					code={JSON.stringify(componentInput.value, null, 2)}
				/>
			</div>
		{/if}
	{:else if fieldType === 'array'}
		<ArrayStaticInputEditor {subFieldType} bind:componentInput on:deleteArrayItem />
	{:else if fieldType === 'schema'}
		<div class="w-full">
			<SchemaEditor bind:schema={componentInput.value} lightMode />
		</div>
	{:else}
		<div class="flex gap-1 relative w-full">
			<input
				on:keydown|stopPropagation
				type="text"
				placeholder={placeholder ?? 'Static value'}
				bind:value={componentInput.value}
				class="!pr-12"
			/>
			{#if !noVariablePicker}
				<!-- svelte-ignore a11y-click-events-have-key-events -->
				<button
					class="absolute right-1 top-1 bottom-1 min-w-min !px-2 items-center text-gray-800 bg-gray-100 border rounded center-center hover:bg-gray-300 transition-all cursor-pointer"
					on:click={() => {
						$pickVariableCallback = (variable) => {
							if (componentInput) {
								componentInput.value = `$var:${variable}`
							}
						}
					}}
					title="Use a Variable"
				>
					<DollarSign size={14} />
				</button>
			{/if}
		</div>
	{/if}
{/if}
