<script lang="ts">
	import { page } from '$app/stores'
	import { JobService, ScriptService, type Script } from '$lib/gen'
	import {
		truncateHash,
		displayDaysAgo,
		defaultIfEmptyString,
		copyToClipboard,
		emptyString,
		encodeState,
		canWrite
	} from '$lib/utils'
	import {
		faPlay,
		faEdit,
		faArchive,
		faList,
		faTrash,
		faCalendar,
		faShare,
		faGlobe,
		faCodeFork,
		faClipboard,
		faArrowLeft,
		faChevronUp,
		faChevronDown
	} from '@fortawesome/free-solid-svg-icons'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ShareModal from '$lib/components/ShareModal.svelte'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import SchemaViewer from '$lib/components/SchemaViewer.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { onDestroy } from 'svelte'
	import HighlightCode from '$lib/components/HighlightCode.svelte'
	import { Badge, Tabs, Tab, TabContent, Button, Alert } from '$lib/components/common'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import Icon from 'svelte-awesome'
	import RunForm from '$lib/components/RunForm.svelte'
	import { goto } from '$app/navigation'
	import Popover from '$lib/components/Popover.svelte'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import { Loader2 } from 'lucide-svelte'
	import { slide } from 'svelte/transition'
	import MoveDrawer from '$lib/components/MoveDrawer.svelte'
	import {
		DEFAULT_WEBHOOK_TYPE,
		SCRIPT_VIEW_SHOW_PUBLISH_TO_HUB,
		SCRIPT_VIEW_SHOW_SCHEDULE,
		SCRIPT_VIEW_SHOW_EXAMPLE_CURL,
		SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON,
		SCRIPT_VIEW_WEBHOOK_INFO_LINK,
		SCRIPT_VIEW_WEBHOOK_INFO_TIP
	} from '$lib/consts'
	import { sendUserToast } from '$lib/toast'
	import { scriptToHubUrl } from '$lib/hub'
	import Urlize from '$lib/components/Urlize.svelte'

	let userSettings: UserSettings
	let script: Script | undefined
	let topHash: string | undefined
	let can_write = false
	let deploymentInProgress = false
	let intervalId: NodeJS.Timer

	let shareModal: ShareModal

	$: loading = !script
	$: if ($workspaceStore) {
		loadScript($page.params.hash)
	}
	$: webhooks = {
		async: {
			hash: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run/h/${script?.hash}`,
			path: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run/p/${script?.path}`
		},
		sync: {
			hash: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run_wait_result/h/${script?.hash}`,
			path: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run_wait_result/p/${script?.path}`,
			get_path: `${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run_wait_result/p/${script?.path}`
		}
	}

	async function deleteScript(hash: string): Promise<void> {
		try {
			await ScriptService.deleteScriptByHash({ workspace: $workspaceStore!, hash })
			loadScript(hash)
		} catch (err) {
			console.error(err)
			sendUserToast(`Could not delete this script ${err.body}`, true)
		}
	}

	async function archiveScript(hash: string): Promise<void> {
		await ScriptService.archiveScriptByHash({ workspace: $workspaceStore!, hash })
		loadScript(hash)
	}

	async function unarchiveScript(hash: string): Promise<void> {
		const r = await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash })
		const ns = await ScriptService.createScript({
			workspace: $workspaceStore!,
			requestBody: {
				...r,
				parent_hash: hash,
				lock: r.lock?.split('\n')
			}
		})
		sendUserToast(`Unarchived script`)
		loadScript(ns)
		goto(`/scripts/get/${ns}`)
	}

	async function syncer(): Promise<void> {
		if (script?.hash) {
			const status = await ScriptService.getScriptDeploymentStatus({
				workspace: $workspaceStore!,
				hash: script?.hash!
			})
			if (status.lock != undefined || status.lock_error_logs != undefined) {
				deploymentInProgress = false
				script.lock = status.lock
				script.lock_error_logs = status.lock_error_logs
				clearInterval(intervalId)
			}
		}
	}

	async function loadScript(hash: string): Promise<void> {
		try {
			script = await ScriptService.getScriptByHash({ workspace: $workspaceStore!, hash })
		} catch {
			script = await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path: hash })
			hash = script.hash
		}
		can_write =
			script.workspace_id == $workspaceStore &&
			canWrite(script.path, script.extra_perms!, $userStore)
		if (script.path && script.archived) {
			const script_by_path = await ScriptService.getScriptByPath({
				workspace: $workspaceStore!,
				path: script.path
			}).catch((_) => console.error('this script has no non-archived version'))
			if (script_by_path?.hash != script.hash) {
				topHash = script_by_path?.hash
			}
		} else {
			topHash = undefined
		}
		intervalId && clearInterval(intervalId)
		deploymentInProgress = script.lock == undefined && script.lock_error_logs == undefined
		if (deploymentInProgress) {
			intervalId = setInterval(syncer, 500)
		}
	}

	onDestroy(() => {
		intervalId && clearInterval(intervalId)
	})

	let isValid = true

	let runLoading = false
	async function runScript(
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisibleToOwner?: boolean
	) {
		try {
			runLoading = true
			const scheduledFor = scheduledForStr ? new Date(scheduledForStr).toISOString() : undefined
			let run = await JobService.runScriptByHash({
				workspace: $workspaceStore!,
				hash: script?.hash ?? '',
				requestBody: args,
				scheduledFor,
				invisibleToOwner
			})
			await goto('/run/' + run + '?workspace=' + $workspaceStore)
		} catch (err) {
			runLoading = false
			sendUserToast(`Could not create job: ${err.body}`, true)
		}
	}
	let scheduleEditor: ScheduleEditor
	let webhookElem: HTMLHeadingElement

	let viewWebhookCommand = false

	let args = undefined
	function curlCommand(async: boolean) {
		return `curl -H 'Content-Type: application/json' -H "Authorization: Bearer $TOKEN" -X POST -d '${JSON.stringify(
			args
		)}' ${$page.url.protocol}//${$page.url.hostname}/api/w/${$workspaceStore}/jobs/run${
			async ? '' : '_wait_result'
		}/p/${script?.path}`
	}
	let moveDrawer: MoveDrawer
</script>

<MoveDrawer
	bind:this={moveDrawer}
	on:update={async (e) => {
		await goto('/scripts/get/' + e.detail + `?workspace=${$workspaceStore}`)
		loadScript($page.params.hash)
	}}
/>

<ScheduleEditor bind:this={scheduleEditor} />

{#if script}
	<CenteredPage>
		<Skeleton {loading} layout={[[{ h: 1.5, w: 40 }], 1, [{ h: 1, w: 30 }]]} />

		<div class="prose-sm mx-auto mt-6">
			<div
				class="flex flex-row-reverse w-full flex-wrap md:flex-nowrap justify-between gap-x-2 gap-y-4"
			>
				<div class="flex flex-row-reverse gap-2 h-full">
					<Button
						href={`/scripts/run/${script.hash}`}
						color="blue"
						size="md"
						startIcon={{ icon: faPlay }}
					>
						Run
					</Button>
					{#if !$userStore?.operator}
						<Button
							href={`/scripts/edit/${script.path}?args=${encodeState(args)}${
								topHash ? `&hash=${script.hash}&topHash=` + topHash : ''
							}`}
							color="blue"
							size="md"
							startIcon={{ icon: faEdit }}
							disabled={!can_write}
						>
							Edit
						</Button>
						{#if !topHash}
							<Button
								href={`/scripts/add?template=${script.path}`}
								variant="border"
								size="md"
								startIcon={{ icon: faCodeFork }}
							>
								Fork
							</Button>
						{/if}
					{/if}
					<Button
						href={`/runs/${script.path}`}
						size="md"
						startIcon={{ icon: faList }}
						color="light"
						variant="border"
					>
						View Runs
					</Button>
				</div>
				<div class="grow truncate">
					<h1 class="mb-1 truncate">{defaultIfEmptyString(script.summary, script.path)}</h1>
				</div>
			</div>

			{#if !emptyString(script.summary)}
				<span class="text-lg font-semibold">{script.path}</span>
			{/if}

			<div class="flex flex-row gap-x-2 flex-wrap items-center mt-2">
				<span class="text-sm text-gray-600">
					Edited {displayDaysAgo(script.created_at || '')} by {script.created_by || 'unknown'}
				</span>
				<Badge color="dark-gray">
					{truncateHash(script?.hash ?? '')}
				</Badge>
				{#if script?.is_template}
					<Badge color="blue">Template</Badge>
				{/if}
				{#if script && script.kind !== 'script'}
					<Badge color="blue">
						{script?.kind}
					</Badge>
				{/if}
				{#if deploymentInProgress}
					<Badge color="yellow">
						<Loader2 size={12} class="inline animate-spin mr-1" />
						Deployment in progress
					</Badge>
				{/if}
				<SharedBadge canWrite={can_write} extraPerms={script?.extra_perms ?? {}} />
			</div>

			<div class="flex gap-2 flex-wrap mt-4">
				{#if SCRIPT_VIEW_SHOW_PUBLISH_TO_HUB}
					<Button
						disabled={deploymentInProgress}
						target="_blank"
						href={scriptToHubUrl(
							script.content,
							script.summary,
							script.description ?? '',
							script.kind,
							script.language,
							script.schema,
							script.language == 'deno' ? '' : script.lock
						).toString()}
						variant="border"
						color="light"
						size="xs"
						startIcon={{ icon: faGlobe }}
					>
						Publish to Hub
					</Button>
				{/if}
				<Button
					on:click={() => shareModal.openDrawer(script?.path ?? '', 'script')}
					variant="border"
					color="light"
					size="xs"
					startIcon={{ icon: faShare }}
					disabled={!can_write}
				>
					Share
				</Button>
				{#if SCRIPT_VIEW_SHOW_SCHEDULE}
					<Button
						on:click={() => scheduleEditor?.openNew(false, script?.path ?? '')}
						variant="border"
						color="light"
						size="xs"
						startIcon={{ icon: faCalendar }}
					>
						New Schedule
					</Button>
				{/if}
				<Button
					on:click={() => moveDrawer.openDrawer(script?.path ?? '', script?.summary, 'script')}
					variant="border"
					color="light"
					size="xs"
					startIcon={{ icon: faEdit }}
				>
					Move/Rename
				</Button>
				<Button
					color="dark"
					variant="border"
					size="xs"
					on:click={() => webhookElem.scrollIntoView()}>Webhooks</Button
				>
				{#if Array.isArray(script.parent_hashes) && script.parent_hashes.length > 0}
					<Button
						color="dark"
						variant="contained"
						size="xs"
						startIcon={{ icon: faArrowLeft }}
						href="/scripts/get/{script.parent_hashes[0]}?workspace={$workspaceStore}"
						dropdownItems={script.parent_hashes.map((hash) => ({
							href: `/scripts/get/${hash}?workspace=${$workspaceStore}`,
							label: hash
						}))}
					>
						Previous version ({script.parent_hashes.length})
					</Button>
				{/if}
			</div>

			{#if script.lock_error_logs || topHash || script.archived || script.deleted}
				<div class="flex flex-col gap-2 my-2">
					{#if script.lock_error_logs}
						<div class="bg-red-100 border-l-4 border-red-500 text-red-700 p-4" role="alert">
							<p class="font-bold">Error deploying this script</p>
							<p>This script has not been deployed successfully because of the following errors:</p>
							<pre class="w-full text-xs mt-2 whitespace-pre-wrap">{script.lock_error_logs}</pre>
						</div>
					{/if}
					{#if topHash}
						<div class="mt-2" />
						<Alert type="warning" title="Not HEAD">
							This hash is not HEAD (latest non-archived version at this path) :
							<a href="/scripts/get/{topHash}?workspace={$workspaceStore}"
								>Go to the HEAD of this path</a
							>
						</Alert>
					{/if}
					{#if script.archived && !topHash}
						<Alert type="error" title="Archived">This path was archived</Alert>
					{/if}
					{#if script.deleted}
						<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4" role="alert">
							<p class="font-bold">Deleted</p>
							<p>The content of this script was deleted (by an admin, no less)</p>
						</div>
					{/if}
				</div>
			{/if}

			<div class="mt-6 grid grid-cols-1 sm:grid-cols-3 gap-6">
				<div class="col-span-2 box">
					<RunForm
						loading={runLoading}
						autofocus
						detailed={false}
						bind:isValid
						runnable={script}
						runAction={runScript}
						bind:args
						viewCliRun
						isFlow={false}
					/>
				</div>
				{#if !emptyString(script.description)}
					<div class="box overflow-auto break-words whitespace-pre-wrap">
						<Urlize text={defaultIfEmptyString(script.description, 'No description')} />
					</div>
				{/if}
			</div>

			<div class="mt-8">
				<Skeleton {loading} layout={[[20]]} />

				<Tabs selected="code">
					<Tab value="code">Code</Tab>
					<Tab value="dependencies">Dependencies lock file</Tab>
					<Tab value="arguments"
						><span class="inline-flex items-center gap-1">
							Arguments JSON Schema
							<Tooltip>
								The jsonschema defines the constraints that the payload must respect to be
								compatible with the input parameters of this script. The UI form is generated
								automatically from the script jsonschema. See
								<a href="https://json-schema.org/" class="text-blue-500">
									jsonschema documentation
								</a>
							</Tooltip>
						</span>
					</Tab>
					<svelte:fragment slot="content">
						<TabContent value="code">
							<div class="border rounded-sm mt-2">
								<HighlightCode language={script.language} code={script.content} />
							</div>
						</TabContent>
						<TabContent value="dependencies">
							<div class="border rounded-sm mt-2">
								{#if script?.lock}
									<pre>{script.lock}</pre>
								{:else}
									<p>There is no lock file for this script</p>
								{/if}
							</div>
						</TabContent>
						<TabContent value="arguments">
							<div class="max-w-2xl">
								<SchemaViewer schema={script.schema} />
							</div>
						</TabContent>
					</svelte:fragment>
				</Tabs>
			</div>

			<div class="max-w-2xl mt-12">
				<h3 class="mb-4" bind:this={webhookElem} id="webhooks">
					Webhooks
					<Tooltip>
						{SCRIPT_VIEW_WEBHOOK_INFO_TIP}
						<a href={SCRIPT_VIEW_WEBHOOK_INFO_LINK} class="text-blue-500"> See docs </a>
					</Tooltip>
				</h3>
				<Skeleton {loading} layout={[[8.5]]} />
				<Tabs selected={DEFAULT_WEBHOOK_TYPE}>
					<Tab value="async">UUID/Async</Tab>
					<Tab value="sync">Result/Sync</Tab>
					<svelte:fragment slot="content">
						{#each Object.keys(webhooks) as key}
							<TabContent value={key}>
								<ul>
									{#each Object.keys(webhooks[key]) as type}
										{@const url = webhooks[key][type]}
										{@const href = $page.url.protocol + '//' + url}
										<li class="flex justify-between items-center mt-2">
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
											{#if type == 'get_path'}
												<div class="flex flex-row gap-1">
													<Badge>GET</Badge>
													<Tooltip
														>This webhook unlike the others which are all POST takes in a GET
														request. The payload must be passed as the query arg `payload` and
														encoded in JSON first, then in an URL safe base64. e.g:
														`encodeURIComponent(btoa(JSON.stringify({'{a: 2}'})))` `
													</Tooltip>
												</div>
											{:else}
												<div class="flex flex-row gap-1">
													<Badge>POST</Badge>

													<Badge color="dark-gray" capitalize>
														{type}
													</Badge>
												</div>
											{/if}
										</li>
									{/each}
								</ul>
								{#if SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON}
									<div class="flex flex-row-reverse mt-2">
										<Button size="xs" on:click={userSettings.openDrawer}
											>Create a Webhook-specific Token <Tooltip
												>The token will have a scope such that it can only be used to trigger this
												script. It is safe to share as it cannot be used to impersonate you.</Tooltip
											></Button
										>
									</div>
								{/if}
								{#if SCRIPT_VIEW_SHOW_EXAMPLE_CURL}
									<div class="flex">
										<Button
											color="light"
											size="lg"
											endIcon={{ icon: viewWebhookCommand ? faChevronUp : faChevronDown }}
											on:click={() => (viewWebhookCommand = !viewWebhookCommand)}
										>
											CURL
										</Button>
									</div>
								{/if}
								{#if viewWebhookCommand}
									{@const command = curlCommand(key == 'async')}
									<div transition:slide|local class="px-4">
										<!-- svelte-ignore a11y-click-events-have-key-events -->
										<pre class="bg-gray-700 text-gray-100 p-2 font-mono text-sm whitespace-pre-wrap"
											>{command} <span
												on:click={() => copyToClipboard(command)}
												class="cursor-pointer ml-2"><Icon data={faClipboard} /></span
											>{#if key == 'async'}<br /><br
												/>//^ returns an UUID. Fetch until completed == true<br
												/>curl -H "Authorization: Bearer $TOKEN" {$page.url.protocol}//{$page.url
													.hostname}/api/w/{$workspaceStore}/jobs_u/completed/get_result_maybe/$UUID{/if}</pre
										>
									</div>
								{/if}
							</TabContent>
						{/each}
					</svelte:fragment>
				</Tabs>
			</div>
			<div class="mt-32">
				{#if can_write}
					<h3>Danger zone</h3>
					<div class="flex gap-2">
						<Popover>
							<Button
								size="xs"
								on:click={() => {
									script?.hash && deleteScript(script.hash)
								}}
								color="red"
								variant="contained"
								startIcon={{ icon: faTrash }}
								disabled={!($superadmin || ($userStore?.is_admin ?? false))}
							>
								Delete
							</Button>
							<span slot="text">require to be admin</span>
						</Popover>
						{#if script.archived}
							<Button
								size="xs"
								on:click={() => {
									script?.hash && unarchiveScript(script.hash)
								}}
								color="red"
								variant="border"
								startIcon={{ icon: faArchive }}
							>
								Unarchive
							</Button>
						{:else}
							<Button
								size="xs"
								on:click={() => {
									script?.hash && archiveScript(script.hash)
								}}
								color="red"
								variant="border"
								startIcon={{ icon: faArchive }}
							>
								Archive
							</Button>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	</CenteredPage>
{/if}

<UserSettings bind:this={userSettings} scopes={[`run:script/${script?.path}`]} />

<ShareModal bind:this={shareModal} />
