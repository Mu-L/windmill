<script lang="ts">
	import { importStore } from '$lib/components/apps/store'

	import AppEditor from '$lib/components/apps/editor/AppEditor.svelte'
	import { AppService, Policy } from '$lib/gen'
	import { page } from '$app/stores'
	import { decodeState } from '$lib/utils'
	import { dirtyStore } from '$lib/components/common/confirmationModal/dirtyStore'
	import { userStore, workspaceStore } from '$lib/stores'
	import type { App } from '$lib/components/apps/types'
	import { goto } from '$app/navigation'
	import { sendUserToast } from '$lib/toast'

	let nodraft = $page.url.searchParams.get('nodraft')
	const hubId = $page.url.searchParams.get('hub')
	const templatePath = $page.url.searchParams.get('template')
	const templateId = $page.url.searchParams.get('template_id')

	const importJson = $importStore
	if ($importStore) {
		$importStore = undefined
	}

	const state = nodraft ? undefined : localStorage.getItem('app')

	let summary = ''
	let value: App = {
		grid: [],
		fullscreen: false,
		unusedInlineScripts: [],
		hiddenInlineScripts: [],
		css: {}
	}

	if (nodraft) {
		goto('?', { replaceState: true })
	}

	let policy: Policy = {
		on_behalf_of: $userStore?.username.includes('@')
			? $userStore?.username
			: `u/${$userStore?.username}`,
		on_behalf_of_email: $userStore?.email,
		execution_mode: Policy.execution_mode.PUBLISHER
	}

	loadApp()

	async function loadApp() {
		if (importJson) {
			sendUserToast('Loaded from JSON')
			if ('value' in importJson) {
				summary = importJson.summary
				value = importJson.value
				policy = importJson.policy
			} else {
				value = importJson
			}
		} else if (templatePath) {
			const template = await AppService.getAppByPath({
				workspace: $workspaceStore!,
				path: templatePath
			})
			value = template.value
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (templateId) {
			const template = await AppService.getAppByVersion({
				workspace: $workspaceStore!,
				id: parseInt(templateId)
			})
			value = template.value
			sendUserToast('App loaded from template')
			goto('?', { replaceState: true })
		} else if (hubId) {
			const hub = await AppService.getHubAppById({ id: Number(hubId) })
			value = {
				hiddenInlineScripts: [],
				unusedInlineScripts: [],
				fullscreen: false,
				...hub.app.value
			}
			summary = hub.app.summary
			sendUserToast('App loaded from Hub')
			goto('?', { replaceState: true })
		} else if (!templatePath && !hubId && state) {
			sendUserToast('App restored from draft', false, [
				{
					label: 'Start from blank',
					callback: () => {
						value = {
							grid: [],
							fullscreen: false,
							unusedInlineScripts: [],
							hiddenInlineScripts: [],
							css: {}
						}
					}
				}
			])
			value = decodeState(state)
		}
	}

	$dirtyStore = false
</script>

{#if value}
	<div class="h-screen">
		{#key value}
			<AppEditor versions={[]} {summary} app={value} path={''} {policy} fromHub={hubId != null} />
		{/key}
	</div>
{/if}
