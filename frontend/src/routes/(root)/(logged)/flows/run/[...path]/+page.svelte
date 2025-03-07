<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import SavedInputs from '$lib/components/SavedInputs.svelte'
	import RunForm from '$lib/components/RunForm.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import { Button, Kbd, Skeleton } from '$lib/components/common'
	import { FlowService, JobService, type Flow } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import {
		canWrite,
		defaultIfEmptyString,
		displayDaysAgo,
		emptyString,
		getModifierKey
	} from '$lib/utils'
	import { faEye, faPen, faPlay } from '@fortawesome/free-solid-svg-icons'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { tweened } from 'svelte/motion'
	import { cubicOut } from 'svelte/easing'
	import { ArrowLeftIcon, ArrowRightIcon } from 'lucide-svelte'
	import { sendUserToast } from '$lib/toast'

	const path = $page.params.path
	let flow: Flow | undefined
	let runForm: RunForm | undefined
	let isValid = true
	let can_write = false
	let args: object = {}

	async function loadFlow() {
		try {
			if (path) {
				flow = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path })
				can_write =
					flow.workspace_id == $workspaceStore && canWrite(flow.path, flow.extra_perms!, $userStore)
			} else {
				sendUserToast(`Failed to fetch flow path from URL`, true)
			}
		} catch (err) {
			console.error(err)
			sendUserToast(`Could not load flow: ${err}`, true)
		}
	}

	let loading = false

	async function runFlow(
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisibleToOwner?: boolean
	) {
		loading = true
		const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
		let run = await JobService.runFlowByPath({
			workspace: $workspaceStore!,
			path,
			invisibleToOwner,
			requestBody: args,
			scheduledFor
		})
		goto('/run/' + run + '?workspace=' + $workspaceStore)
	}

	$: {
		if ($workspaceStore) {
			loadFlow()
		}
	}
	function onKeyDown(event: KeyboardEvent) {
		switch (event.key) {
			case 'Enter':
				if (event.ctrlKey || event.metaKey) {
					if (isValid) {
						event.preventDefault()
						runForm?.run()
					} else {
						sendUserToast('Please fix errors before running', true)
					}
				}
				break
		}
	}

	let savedInputPaneSize = tweened(0, {
		duration: 200,
		easing: cubicOut
	})

	let reloadArgs = 0
</script>

<svelte:window on:keydown={onKeyDown} />

<SplitPanesWrapper class="h-screen">
	<Splitpanes class="overflow-hidden">
		<Pane class="px-4 flex justify-center" size={100 - $savedInputPaneSize} minSize={50}>
			<div class="w-full max-w-4xl flex flex-col">
				{#if flow}
					<div class="flex flex-row flex-wrap justify-between gap-4 mb-6">
						<div class="w-full">
							<div class="flex flex-col mt-6 mb-2 w-full">
								<div
									class="flex flex-row-reverse w-full flex-wrap md:flex-nowrap justify-between gap-x-1"
								>
									<div class="flex flex-row gap-4">
										<div class="flex flex-row items-center gap-2">
											{#if !$userStore?.operator && can_write}
												<div>
													<Button
														size="sm"
														startIcon={{ icon: faPen }}
														disabled={flow == undefined}
														variant="border"
														href="/flows/edit/{flow?.path}"
													>
														Edit
													</Button>
												</div>
											{/if}
											<Button
												size="sm"
												startIcon={{ icon: faEye }}
												disabled={flow == undefined}
												variant="border"
												href="/flows/get/{flow?.path}?workspace={$workspaceStore}"
											>
												Flow
											</Button>

											<Button
												startIcon={{ icon: faPlay }}
												disabled={runForm == undefined || !isValid}
												on:click={() => runForm?.run()}
												>Run &nbsp;<Kbd small>{getModifierKey()}</Kbd>
												<Kbd small><span class="text-lg font-bold">⏎</span></Kbd></Button
											>
										</div>
									</div>
									<div class="flex flex-col">
										<h1 class="break-words py-2 mr-2">
											{defaultIfEmptyString(flow.summary, flow.path)}
										</h1>
										{#if !emptyString(flow.summary)}
											<h2 class="font-bold pb-4">{flow.path}</h2>
										{/if}
									</div></div
								>
								<div class="flex items-center gap-2">
									<span class="text-sm text-gray-500">
										{#if flow}
											Edited {displayDaysAgo(flow.edited_at || '')} by {flow.edited_by || 'unknown'}
										{/if}
									</span>

									<SharedBadge canWrite={can_write} extraPerms={flow?.extra_perms ?? {}} />
								</div>
							</div>
						</div>
						{#if !emptyString(flow.description)}
							<div class="prose text-sm box max-w-6xl w-full mt-8">
								{defaultIfEmptyString(flow.description, 'No description')}
							</div>
						{/if}
					</div>
					<div class="flex justify-end">
						<Button
							size="xs"
							variant="border"
							disabled={flow == undefined}
							color="light"
							on:click={() => {
								//savedInputPaneSize = savedInputPaneSize == 0 ? 30 : 0
								savedInputPaneSize.set($savedInputPaneSize === 0 ? 30 : 0)
							}}
						>
							<div class="flex flex-row gap-2 items-center">
								{$savedInputPaneSize === 0 ? 'Open input library' : 'Close input library'}
								{#if $savedInputPaneSize === 0}
									<ArrowRightIcon class="w-4 h-4" />
								{:else}
									<ArrowLeftIcon class="w-4 h-4" />
								{/if}
							</div>
						</Button>
					</div>
					{#key reloadArgs}
						<RunForm
							{loading}
							autofocus
							bind:this={runForm}
							bind:isValid
							detailed={false}
							runnable={flow}
							runAction={runFlow}
							viewCliRun
							isFlow
							bind:args
						/>
					{/key}
				{:else}
					<Skeleton layout={[2, [3], 1, [2], 4, [4], 3, [8]]} />
				{/if}
			</div>
		</Pane>
		<Pane size={$savedInputPaneSize}>
			{#if $savedInputPaneSize > 0}
				<SavedInputs
					flowPath={path}
					{isValid}
					{args}
					on:selected_args={(e) => {
						args = JSON.parse(JSON.stringify(e.detail))
						reloadArgs += 1
					}}
				/>
			{/if}
		</Pane>
	</Splitpanes>
</SplitPanesWrapper>
