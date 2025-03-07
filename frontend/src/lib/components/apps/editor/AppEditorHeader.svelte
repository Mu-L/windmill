<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { Alert, Badge, Drawer, DrawerContent, Kbd, UndoRedo } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import DisplayResult from '$lib/components/DisplayResult.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import FlowProgressBar from '$lib/components/flows/FlowProgressBar.svelte'
	import FlowStatusViewer from '$lib/components/FlowStatusViewer.svelte'
	import JobArgs from '$lib/components/JobArgs.svelte'
	import LogViewer from '$lib/components/LogViewer.svelte'
	import Path from '$lib/components/Path.svelte'
	import TestJobLoader from '$lib/components/TestJobLoader.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { AppService, DraftService, Job, Policy } from '$lib/gen'
	import { redo, undo } from '$lib/history'
	import { workspaceStore } from '$lib/stores'
	import {
		faClipboard,
		faFileExport,
		faHistory,
		faSave,
		faSlidersH
	} from '@fortawesome/free-solid-svg-icons'
	import {
		AlignHorizontalSpaceAround,
		Bug,
		Expand,
		Laptop2,
		Loader2,
		Smartphone
	} from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { classNames, copyToClipboard } from '../../../utils'
	import type {
		AppInput,
		ConnectedAppInput,
		RowAppInput,
		Runnable,
		StaticAppInput,
		UserAppInput
	} from '../inputType'
	import type { AppEditorContext, AppViewerContext } from '../types'
	import { BG_PREFIX, allItems, toStatic } from '../utils'
	import AppExportButton from './AppExportButton.svelte'
	import AppInputs from './AppInputs.svelte'
	import type { AppComponent } from './component/components'
	import PanelSection from './settingsPanel/common/PanelSection.svelte'
	import PreviewToggle from './PreviewToggle.svelte'

	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import UnsavedConfirmationModal from '$lib/components/common/confirmationModal/UnsavedConfirmationModal.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Sha256 } from '@aws-crypto/sha256-js'
	import { sendUserToast } from '$lib/toast'
	import DeploymentHistory from './DeploymentHistory.svelte'

	async function hash(message) {
		try {
			const msgUint8 = new TextEncoder().encode(message) // encode as (utf-8) Uint8Array
			const hashBuffer = await crypto.subtle.digest('SHA-256', msgUint8) // hash the message
			const hashArray = Array.from(new Uint8Array(hashBuffer)) // convert buffer to byte array
			const hashHex = hashArray.map((b) => b.toString(16).padStart(2, '0')).join('') // convert bytes to hex string
			return hashHex
		} catch {
			//subtle not available, trying pure js
			const hash = new Sha256()
			hash.update(message)
			const result = Array.from(await hash.digest())
			const hex = result.map((b) => b.toString(16).padStart(2, '0')).join('') // convert bytes to hex string
			return hex
		}
	}

	export let policy: Policy
	export let fromHub: boolean = false
	export let versions: number[]

	const {
		app,
		summary,
		breakpoint,
		appPath,
		jobs,
		staticExporter,
		errorByComponent,
		openDebugRun
	} = getContext<AppViewerContext>('AppViewerContext')

	const { history } = getContext<AppEditorContext>('AppEditorContext')

	const loading = {
		publish: false,
		save: false,
		saveDraft: false
	}

	$: if ($openDebugRun == undefined) {
		$openDebugRun = (componentId: string) => {
			jobsDrawerOpen = true

			const job = $jobs.find((job) => job.component === componentId)
			if (job) {
				selectedJobId = job.job
			}
		}
	}

	let newPath: string = ''
	let pathError: string | undefined = undefined
	let appExport: AppExportButton

	let draftDrawerOpen = false
	let saveDrawerOpen = false
	let jobsDrawerOpen = false
	let inputsDrawerOpen = fromHub
	let historyBrowserDrawerOpen = false

	function closeSaveDrawer() {
		saveDrawerOpen = false
	}

	function closeDraftDrawer() {
		draftDrawerOpen = false
	}

	function collectStaticFields(
		fields: Record<string, StaticAppInput | ConnectedAppInput | RowAppInput | UserAppInput>
	) {
		return Object.fromEntries(
			Object.entries(fields ?? {})
				.filter(([k, v]) => v.type == 'static')
				.map(([k, v]) => {
					return [k, v['value']]
				})
		)
	}
	async function computeTriggerables() {
		const allTriggers = (await Promise.all(
			allItems($app.grid, $app.subgrids)
				.flatMap((x) => {
					let c = x.data as AppComponent
					let r: { input: AppInput | undefined; id: string }[] = [
						{ input: c.componentInput, id: x.id }
					]
					if (c.type === 'tablecomponent') {
						r.push(...c.actionButtons.map((x) => ({ input: x.componentInput, id: x.id })))
					}
					return r
						.filter((x) => x.input)
						.map(async (o) => {
							if (o.input?.type == 'runnable') {
								return await processRunnable(o.id, o.input.runnable, o.input.fields)
							}
						})
				})
				.concat(
					Object.values($app.hiddenInlineScripts ?? {}).map(async (v, i) => {
						return await processRunnable(BG_PREFIX + i, v, v.fields)
					})
				)
		)) as ([string, Record<string, any>] | undefined)[]
		policy.triggerables = Object.fromEntries(
			allTriggers.filter(Boolean) as [string, Record<string, any>][]
		)
	}

	async function processRunnable(
		id: string,
		runnable: Runnable,
		fields: Record<string, any>
	): Promise<[string, Record<string, any>] | undefined> {
		const staticInputs = collectStaticFields(fields)
		if (runnable?.type == 'runnableByName') {
			let hex = await hash(runnable.inlineScript?.content)
			return [`${id}:rawscript/${hex}`, staticInputs]
		} else if (runnable?.type == 'runnableByPath') {
			let prefix = runnable.runType !== 'hubscript' ? runnable.runType : 'script'
			return [`${id}:${prefix}/${runnable.path}`, staticInputs]
		}
	}
	async function createApp(path: string) {
		await computeTriggerables()
		try {
			const appId = await AppService.createApp({
				workspace: $workspaceStore!,
				requestBody: {
					value: $app,
					path,
					summary: $summary,
					policy
				}
			})
			$dirtyStore = false
			closeSaveDrawer()
			sendUserToast('App deployed successfully')
			goto(`/apps/edit/${appId}`)
		} catch (e) {
			sendUserToast('Error creating app', e)
		}
	}

	async function updateApp(path: string) {
		await computeTriggerables()
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path: $page.params.path,
			requestBody: {
				value: $app!,
				summary: $summary,
				policy
			}
		})
		$dirtyStore = false
		closeSaveDrawer()
		sendUserToast('App deployed successfully')
	}

	let secretUrl: string | undefined = undefined

	$: secretUrl == undefined && policy.execution_mode == 'anonymous' && getSecretUrl()

	async function getSecretUrl() {
		secretUrl = await AppService.getPublicSecretOfApp({
			workspace: $workspaceStore!,
			path: appPath
		})
	}

	async function setPublishState() {
		await AppService.updateApp({
			workspace: $workspaceStore!,
			path: appPath,
			requestBody: { policy }
		})
		if (policy.execution_mode == 'anonymous') {
			sendUserToast('App made visible publicly at the secret URL.')
		} else {
			sendUserToast('App made unaccessible publicly')
		}
	}

	async function save() {
		$dirtyStore = false
		saveDrawerOpen = true
		return
	}

	async function saveInitialDraft() {
		await computeTriggerables()
		try {
			await AppService.createApp({
				workspace: $workspaceStore!,
				requestBody: {
					value: $app,
					path: newPath,
					summary: $summary,
					policy,
					draft_only: true
				}
			})
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: {
					path: newPath,
					typ: 'app',
					value: $app!
				}
			})
			draftDrawerOpen = false
			$dirtyStore = false
			goto(`/apps/edit/${newPath}`)
		} catch (e) {
			sendUserToast('Error saving initial draft', e)
		}
		draftDrawerOpen = false
	}

	async function saveDraft() {
		$dirtyStore = false
		if ($page.params.path == undefined) {
			draftDrawerOpen = true
			return
		}
		loading.saveDraft = true
		try {
			await computeTriggerables()
			$dirtyStore = false
			await DraftService.createDraft({
				workspace: $workspaceStore!,
				requestBody: {
					path: $page.params.path,
					typ: 'app',
					value: $app!
				}
			})
			sendUserToast('Draft saved')
			loading.saveDraft = false
		} catch (e) {
			loading.saveDraft = false
			throw e
		}
	}

	let selectedJobId: string | undefined = undefined
	let testJobLoader: TestJobLoader
	let job: Job | undefined = undefined
	let testIsLoading = false

	$: selectedJobId && !selectedJobId?.includes('Frontend') && testJobLoader?.watchJob(selectedJobId)

	$: if (selectedJobId?.includes('Frontend') && selectedJobId) {
		job = undefined
	}

	$: hasErrors = Object.keys($errorByComponent).length > 0

	let lock = false
	function onKeyDown(event: KeyboardEvent) {
		if (lock) return

		let classes = event.target?.['className']
		if (
			(typeof classes === 'string' && classes.includes('inputarea')) ||
			['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName!)
		) {
			return
		}

		lock = true

		switch (event.key) {
			case 'Z':
				if (event.ctrlKey || event.metaKey) {
					$app = redo(history)
					event.preventDefault()
				}
				break
			case 'z':
				if (event.ctrlKey || event.metaKey) {
					$app = undo(history, $app)

					event.preventDefault()
				}
				break
			case 's':
				if (event.ctrlKey || event.metaKey) {
					saveDraft()
					event.preventDefault()
				}
				break
			// case 'ArrowDown': {
			// 	let ids = generateIds()
			// 	let idx = ids.indexOf($selectedIdStore)
			// 	if (idx > -1 && idx < ids.length - 1) {
			// 		$selectedIdStore = ids[idx + 1]
			// 		event.preventDefault()
			// 	}
			// 	break
			// }
			// case 'ArrowUp': {
			// 	let ids = generateIds()
			// 	let idx = ids.indexOf($selectedIdStore)
			// 	if (idx > 0 && idx < ids.length) {
			// 		$selectedIdStore = ids[idx - 1]
			// 		event.preventDefault()
			// 	}
			// 	break
			// }
		}
		lock = false
	}
</script>

<svelte:window on:keydown={onKeyDown} />

<TestJobLoader bind:this={testJobLoader} bind:isLoading={testIsLoading} bind:job />

<UnsavedConfirmationModal />

{#if appPath == ''}
	<Drawer bind:open={draftDrawerOpen} size="800px">
		<DrawerContent title="Initial draft save" on:close={() => closeDraftDrawer()}>
			<Alert title="Require path" type="info">
				Choose a path to save the initial draft of the app.
			</Alert>
			<div class="py-2" />
			<Path
				bind:error={pathError}
				bind:path={newPath}
				initialPath=""
				namePlaceholder="app"
				kind="app"
			/>
			<div class="py-4" />

			<h3>Summary</h3>
			<div class="w-full pt-2">
				<input
					type="text"
					placeholder="App summary"
					class="text-sm w-full font-semibold"
					bind:value={$summary}
				/>
			</div>

			<div slot="actions">
				<Button
					startIcon={{ icon: faSave }}
					disabled={pathError != ''}
					on:click={() => saveInitialDraft()}
				>
					Save initial draft
				</Button>
			</div>
		</DrawerContent>
	</Drawer>
{/if}
<Drawer bind:open={saveDrawerOpen} size="800px">
	<DrawerContent title="Deploy" on:close={() => closeSaveDrawer()}>
		<Path
			bind:error={pathError}
			bind:path={newPath}
			initialPath={appPath}
			namePlaceholder="app"
			kind="app"
		/>

		<div class="py-4" />

		<h3>Summary</h3>
		<div class="w-full pt-2">
			<input
				type="text"
				placeholder="App summary"
				class="text-sm w-full font-semibold"
				bind:value={$summary}
			/>
		</div>

		<div slot="actions">
			<Button
				startIcon={{ icon: faSave }}
				disabled={pathError != ''}
				on:click={() => {
					if (appPath == '') {
						createApp(newPath)
					} else {
						updateApp(newPath)
					}
				}}
			>
				Deploy
			</Button>
		</div>
		<div class="py-2" />
		{#if appPath == ''}
			<Alert title="Require saving" type="error">
				Save this app once before you can publish it
			</Alert>
		{:else}
			<Alert title="App executed on behalf of you">
				A viewer of the app will execute the runnables of the app on behalf of the publisher (you).
				<Tooltip>
					This is to ensure that all resources/runnable available at time of creating the app would
					prevent the good execution of the app. To guarantee tight security, a policy is computed
					at time of deployment of the app which only allow the scripts/flows referred to in the app
					to be called on behalf of, and the resources are passed by reference so that their actual
					value is . Furthermore, static parameters are not overridable. Hence, users will only be
					able to use the app as intended by the publisher without risk for leaking resources not
					used in the app.
				</Tooltip>
			</Alert>
			<div class="mt-4" />
			<Toggle
				options={{
					left: `Require read-access`,
					right: `Publish publicly for anyone knowing the secret url`
				}}
				checked={policy.execution_mode == 'anonymous'}
				on:change={(e) => {
					policy.execution_mode = e.detail
						? Policy.execution_mode.ANONYMOUS
						: Policy.execution_mode.PUBLISHER
					setPublishState()
				}}
			/>

			{#if policy.execution_mode == 'anonymous' && secretUrl}
				{@const url = `${$page.url.hostname}/public/${$workspaceStore}/${secretUrl}`}
				{@const href = $page.url.protocol + '//' + url}
				<div class="my-6 box">
					Public url:
					<a
						on:click={(e) => {
							e.preventDefault()
							copyToClipboard(href)
						}}
						{href}
						class="whitespace-nowrap text-ellipsis overflow-hidden mr-1"
					>
						{url}
						<span class="text-gray-700 ml-2">
							<Icon data={faClipboard} />
						</span>
					</a>
				</div>

				<Alert type="info" title="Only latest saved app is publicly available">
					Once made public, you will still need to deploy the app to make visible the latest changes
				</Alert>
			{/if}
		{/if}
	</DrawerContent>
</Drawer>

<Drawer bind:open={inputsDrawerOpen} size="600px">
	<DrawerContent title="App inputs configuration" on:close={() => (inputsDrawerOpen = false)}>
		<AppInputs />
	</DrawerContent>
</Drawer>

<Drawer bind:open={historyBrowserDrawerOpen} size="1200px">
	<DrawerContent title="Deployment History" on:close={() => historyBrowserDrawerOpen}>
		<DeploymentHistory on:restore {versions} />
	</DrawerContent>
</Drawer>

<Drawer bind:open={jobsDrawerOpen} size="900px">
	<DrawerContent
		noPadding
		title="Debug Runs"
		on:close={() => (jobsDrawerOpen = false)}
		tooltip="Look at latests runs to spot potential bugs."
		documentationLink="https://docs.windmill.dev/docs/apps/app_toolbar#debug-runs"
	>
		<Splitpanes class="!overflow-visible">
			<Pane size={25}>
				<PanelSection title="Past Runs">
					<div class="flex flex-col gap-2 w-full">
						{#if $jobs.length > 0}
							<div class="flex gap-2 flex-col">
								{#each $jobs ?? [] as { job, component } (job)}
									<!-- svelte-ignore a11y-click-events-have-key-events -->
									<div
										class={classNames(
											'border flex gap-1 truncate justify-between flex-row w-full items-center p-2 rounded-md cursor-pointer hover:bg-blue-50 hover:text-blue-400',
											$errorByComponent[job] ? 'border border-red-500 bg-red-100' : '',
											selectedJobId == job && !$errorByComponent[component]
												? 'bg-blue-100 text-blue-600'
												: ''
										)}
										on:click={() => (selectedJobId = job)}
									>
										<span class="text-xs truncate">{job}</span>
										<Badge color="indigo">{component}</Badge>
									</div>
								{/each}
							</div>
						{:else}
							<div class="text-sm text-gray-500">No items</div>
						{/if}
					</div>
				</PanelSection>
			</Pane>
			<Pane size={75}>
				<div class="h-full w-full overflow-auto">
					{#if selectedJobId}
						{#if !job}
							{@const jobResult = $jobs.find((j) => j.job == selectedJobId)}

							{#if jobResult?.error !== undefined}
								<Splitpanes horizontal class="grow border w-full">
									<Pane size={50} minSize={10}>
										<LogViewer
											content={`Logs are avaiable in the browser console directly`}
											isLoading={false}
										/>
									</Pane>
									<Pane size={50} minSize={10} class="text-sm text-gray-600">
										<pre class="overflow-x-auto break-words relative h-full px-2">
											<DisplayResult result={{ error: { name: 'Frontend execution error', message: jobResult.error } }} />
										</pre>
									</Pane>
								</Splitpanes>
							{:else if jobResult?.result !== undefined}
								<Splitpanes horizontal class="grow border w-full">
									<Pane size={50} minSize={10}>
										<LogViewer
											content={`Logs are avaiable in the browser console directly`}
											isLoading={false}
										/>
									</Pane>
									<Pane size={50} minSize={10} class="text-sm text-gray-600">
										<pre class="overflow-x-auto break-words relative h-full px-2">
											<DisplayResult result={jobResult.result} />
										</pre>
									</Pane>
								</Splitpanes>
							{:else}
								<Skeleton layout={[[40]]} />
							{/if}
						{:else}
							<div class="flex flex-col h-full w-full gap-4 p-2 mb-4">
								{#if job?.['running']}
									<div class="flex flex-row-reverse w-full">
										<Button
											color="red"
											variant="border"
											on:click={() => testJobLoader?.cancelJob()}
										>
											<Loader2 size={14} class="animate-spin mr-2" />

											Cancel
										</Button>
									</div>
								{/if}
								<div class="p-2">
									<JobArgs args={job?.args} />
								</div>
								{#if job?.job_kind !== 'flow' && job?.job_kind !== 'flowpreview'}
									<Splitpanes horizontal class="grow border w-full">
										<Pane size={50} minSize={10}>
											<LogViewer content={job?.logs} isLoading={testIsLoading} />
										</Pane>
										<Pane size={50} minSize={10} class="text-sm text-gray-600">
											{#if job != undefined && 'result' in job && job.result != undefined}
												<pre class="overflow-x-auto break-words relative h-full px-2"
													><DisplayResult result={job.result} /></pre
												>
											{:else if testIsLoading}
												<div class="p-2"><Loader2 class="animate-spin" /> </div>
											{/if}
										</Pane>
									</Splitpanes>
								{:else}
									<div class="mt-10" />
									<FlowProgressBar {job} class="py-4" />
									<div class="w-full mt-10 mb-20">
										<FlowStatusViewer
											jobId={job.id}
											on:jobsLoaded={({ detail }) => {
												job = detail
											}}
										/>
									</div>
								{/if}
							</div>
						{/if}
					{:else}
						<div class="text-sm p-2 text-gray-500">Select a job to see its details</div>
					{/if}
				</div>
			</Pane>
		</Splitpanes>
	</DrawerContent>
</Drawer>

<div
	class="border-b flex flex-row justify-between py-1 gap-2 gap-y-2 px-2 items-center overflow-y-visible"
>
	<div class="min-w-64 w-64">
		<input
			type="text"
			placeholder="App summary"
			class="text-sm w-full font-semibold"
			bind:value={$summary}
		/>
	</div>
	<div class="flex gap-4 items-center justify-center">
		<UndoRedo
			undoProps={{ disabled: $history?.index === 0 }}
			redoProps={{ disabled: $history && $history?.index === $history.history.length - 1 }}
			on:undo={() => {
				$app = undo(history, $app)
			}}
			on:redo={() => {
				$app = redo(history)
			}}
		/>

		<div>
			<ToggleButtonGroup class="h-[30px]" bind:selected={$breakpoint}>
				<ToggleButton icon={Smartphone} value="sm" />
				<ToggleButton icon={Laptop2} value="lg" />
			</ToggleButtonGroup>
		</div>
		{#if $app}
			<ToggleButtonGroup class="h-[30px]" bind:selected={$app.fullscreen}>
				<ToggleButton
					icon={AlignHorizontalSpaceAround}
					value={false}
					tooltip="The max width is 1168px and the content stay centered instead of taking the full page width"
				/>
				<ToggleButton icon={Expand} value={true} />
			</ToggleButtonGroup>
		{/if}
	</div>

	<div class="flex flex-row gap-2 justify-end items-center overflow-visible">
		<Dropdown
			placement="bottom-end"
			btnClasses="!rounded-md"
			dropdownItems={[
				{
					displayName: 'Deployment History',
					icon: faHistory,
					action: () => {
						historyBrowserDrawerOpen = true
					}
				},
				{
					displayName: 'JSON',
					icon: faFileExport,
					action: () => {
						appExport.open($app)
					}
				},
				// {
				// 	displayName: 'Publish to Hub',
				// 	icon: faGlobe,
				// 	action: () => {
				// 		const url = appToHubUrl(toStatic($app, $staticExporter, $summary))
				// 		window.open(url.toString(), '_blank')
				// 	}
				// },
				{
					displayName: 'Hub compatible JSON',
					icon: faFileExport,
					action: () => {
						appExport.open(toStatic($app, $staticExporter, $summary).app)
					}
				},
				{
					displayName: 'App Inputs',
					icon: faSlidersH,
					action: () => {
						inputsDrawerOpen = true
					}
				}
			]}
		/>
		<div class="hidden md:inline relative overflow-visible">
			{#if hasErrors}
				<span
					class="animate-ping absolute inline-flex rounded-full bg-red-600 h-2 w-2 z-50 -right-0.5 -top-0.5"
				/>
				<span
					class=" absolute inline-flex rounded-full bg-red-600 h-2 w-2 z-50 -right-0.5 -top-0.5"
				/>
			{/if}
			<Button
				on:click={() => {
					if (selectedJobId == undefined && $jobs.length > 0) {
						selectedJobId = $jobs[0].job
					}
					jobsDrawerOpen = true
				}}
				color={hasErrors ? 'red' : 'light'}
				size="xs"
				variant="border"
				btnClasses="relative"
			>
				<div class="flex flex-row gap-1 items-center">
					<Bug size={14} />
					<div> Debug runs </div>
				</div>
			</Button>
		</div>
		<AppExportButton bind:this={appExport} />
		<PreviewToggle loading={loading.save} />
		<Button loading={loading.save} startIcon={{ icon: faSave }} on:click={saveDraft} size="xs">
			Save draft&nbsp;<Kbd small>Ctrl</Kbd><Kbd small>S</Kbd>
		</Button>
		<Button
			loading={loading.save}
			startIcon={{ icon: faSave }}
			on:click={save}
			size="xs"
			dropdownItems={appPath != ''
				? () => [
						{
							label: 'Fork',
							onClick: () => {
								window.open(`/apps/add?template=${appPath}`)
							}
						}
				  ]
				: undefined}
		>
			Deploy
		</Button>
	</div>
</div>
