<script lang="ts">
	import type { Folder } from '$lib/gen'
	import { FolderService } from '$lib/gen'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import FolderEditor from '$lib/components/FolderEditor.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { userStore, workspaceStore } from '$lib/stores'
	import { faEdit, faPlus, faTrash } from '@fortawesome/free-solid-svg-icons'
	import { Button, Drawer, DrawerContent, Skeleton } from '$lib/components/common'
	import FolderInfo from '$lib/components/FolderInfo.svelte'
	import FolderUsageInfo from '$lib/components/FolderUsageInfo.svelte'
	import { canWrite } from '$lib/utils'

	type FolderW = Folder & { canWrite: boolean }

	let newFolderName: string = ''
	let folders: FolderW[] | undefined = undefined
	let folderDrawer: Drawer

	async function loadFolders(): Promise<void> {
		folders = (await FolderService.listFolders({ workspace: $workspaceStore! })).map((x) => {
			return { canWrite: canWrite('f/' + x.name, x.extra_perms ?? {}, $userStore), ...x }
		})
	}

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key
		if (key === 'Enter') {
			event.preventDefault()
			addFolder()
		}
	}
	async function addFolder() {
		await FolderService.createFolder({
			workspace: $workspaceStore ?? '',
			requestBody: { name: newFolderName }
		})
		$userStore?.folders.push(newFolderName)
		loadFolders()
		editFolderName = newFolderName
		folderDrawer.openDrawer()
	}

	$: {
		if ($workspaceStore && $userStore) {
			loadFolders()
		}
	}

	let editFolderName: string = ''

	function computeMembers(owners: string[], extra_perms: Record<string, any>) {
		const members = new Set(owners)
		for (const [user, _] of Object.entries(extra_perms)) {
			members.add(user)
		}
		return Array.from(members)
	}
</script>

<Drawer bind:this={folderDrawer}>
	<DrawerContent title="Folder {editFolderName}" on:close={folderDrawer.closeDrawer}>
		<FolderEditor on:update={loadFolders} name={editFolderName} />
	</DrawerContent>
</Drawer>

<CenteredPage>
	<PageHeader
		title="Folders"
		tooltip="Folders allow to group items such as scripts/flows/resources/schedule together and to grant homogenous RBAC permissions to groups and individual users towards them."
		documentationLink="https://docs.windmill.dev/docs/core_concepts/groups_and_folders"
	>
		<div class="flex flex-row">
			<input
				class="mr-2"
				on:keyup={handleKeyUp}
				placeholder="New folder name"
				bind:value={newFolderName}
			/>
			<div>
				<Button
					size="md"
					startIcon={{ icon: faPlus }}
					disabled={!newFolderName}
					on:click={addFolder}
				>
					New&nbsp;folder
				</Button>
			</div>
		</div>
	</PageHeader>

	<div class="relative mb-20 pt-8">
		<TableCustom>
			<tr slot="header-row">
				<th class="!px-0" />
				<th>Name</th>
				<th class="!text-center">Scripts</th>
				<th class="!text-center">Flows</th>
				<th class="!text-center">Apps</th>
				<th class="!text-center">Schedules</th>
				<th class="!text-center">Variables</th>
				<th class="!text-center">Resources</th>
				<th>Participants</th>
			</tr>
			<tbody slot="body">
				{#if folders === undefined}
					{#each new Array(4) as _}
						<tr>
							<td colspan="9">
								<Skeleton layout={[[2]]} />
							</td>
						</tr>
					{/each}
				{:else}
					{#if folders.length === 0}
						<tr>
							<td colspan="4" class="text-gray-600 mt-2"> No folders yet, create one! </td>
						</tr>
					{/if}

					{#each folders as { name, extra_perms, owners, canWrite }}
						<tr>
							<td class="!px-0 text-center">
								<SharedBadge {canWrite} extraPerms={extra_perms} />
							</td>
							<td>
								<a
									href="#{name}"
									on:click={() => {
										editFolderName = name
										folderDrawer.openDrawer()
									}}
									>{name}
								</a>
							</td>
							<FolderUsageInfo {name} tabular />

							<td><FolderInfo members={computeMembers(owners, extra_perms)} /></td>
							<td>
								<Dropdown
									placement="bottom-end"
									dropdownItems={[
										{
											displayName: 'Manage folder',
											icon: faEdit,
											disabled: !canWrite,
											action: () => {
												editFolderName = name
												folderDrawer.openDrawer()
											}
										},
										{
											displayName: 'Delete',

											icon: faTrash,
											type: 'delete',
											disabled: !canWrite,
											action: async () => {
												await FolderService.deleteFolder({
													workspace: $workspaceStore ?? '',
													name
												})
												loadFolders()
											}
										}
									]}
								/>
							</td>
						</tr>
					{/each}
				{/if}
			</tbody>
		</TableCustom>
	</div>
</CenteredPage>
