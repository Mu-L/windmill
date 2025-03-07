<script lang="ts">
	import { getContext, onMount } from 'svelte'
	import type {
		AppViewerContext,
		BaseAppComponent,
		ComponentCustomCSS,
		RichConfigurations
	} from '../../../types'
	import type { AppInput } from '../../../inputType'
	import RunnableWrapper from '../../helpers/RunnableWrapper.svelte'
	import { writable } from 'svelte/store'
	import { createSvelteTable, flexRender, type TableOptions } from '@tanstack/svelte-table'
	import AppButton from '../../buttons/AppButton.svelte'
	import { classNames, isObject } from '$lib/utils'
	import DebouncedInput from '../../helpers/DebouncedInput.svelte'
	import AppTableFooter from './AppTableFooter.svelte'
	import { tableOptions } from './tableOptions'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { components, type ButtonComponent } from '../../../editor/component'
	import { concatCustomCss } from '../../../utils'
	import { twMerge } from 'tailwind-merge'
	import { initConfig, initOutput } from '$lib/components/apps/editor/appUtils'
	import ResolveConfig from '../../helpers/ResolveConfig.svelte'
	import AppCheckbox from '../../inputs/AppCheckbox.svelte'
	import AppSelect from '../../inputs/AppSelect.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let actionButtons: (BaseAppComponent & ButtonComponent)[]
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'tablecomponent'> | undefined = undefined
	export let render: boolean

	type T = Record<string, any>

	let result: Record<string, any>[] | undefined = undefined

	const { app, worldStore, componentControl, selectedComponent, hoverStore, mode } =
		getContext<AppViewerContext>('AppViewerContext')

	let searchValue = ''

	let outputs = initOutput($worldStore, id, {
		selectedRowIndex: 0,
		selectedRow: undefined,
		loading: false,
		result: [],
		search: '',
		page: 1
	})

	$: setSearch(searchValue)

	function setSearch(srch: string) {
		outputs?.search?.set(srch)
	}

	const options = writable<TableOptions<T>>({
		...tableOptions,
		data: [],
		columns: []
	})

	let table = createSvelteTable(options)

	let resolvedConfig = initConfig(
		components['tablecomponent'].initialData.configuration,
		configuration
	)

	let selectedRowIndex = -1

	function toggleRow(row: Record<string, any>, rowIndex: number, force: boolean = false) {
		if (
			selectedRowIndex !== rowIndex ||
			JSON.stringify(row.original) !== JSON.stringify(result?.[rowIndex]) ||
			force
		) {
			selectedRowIndex = rowIndex
			outputs?.selectedRow.set(row.original, force)
			outputs?.selectedRowIndex.set(rowIndex, force)
		}
	}

	let mounted = false
	onMount(() => {
		mounted = true
	})

	$: selectedRowIndex === -1 &&
		Array.isArray(result) &&
		result.length > 0 &&
		// We need to wait until the component is mounted so the world is created
		mounted &&
		outputs &&
		toggleRow({ original: result[0] }, 0)

	function searchInResult(result: Array<Record<string, any>>, searchValue: string) {
		if (searchValue === '') {
			return result
		}
		return result.filter((row) =>
			Object.values(row).some((value) => value?.toString()?.includes(searchValue))
		)
	}

	function renderCell(x: any, props: any): any {
		try {
			return flexRender(x, props)
		} catch (e) {
			return undefined
		}
	}

	function cellIsObject(x: (any) => any, props: any): boolean {
		return typeof x != 'string' && typeof x(props) == 'object'
	}

	let filteredResult: Array<Record<string, any>> = []

	function setFilteredResult() {
		if (resolvedConfig.search === 'By Runnable' || resolvedConfig.search === 'Disabled') {
			filteredResult = result ?? []
		} else {
			filteredResult = searchInResult(result ?? [], searchValue)
		}
	}
	$: (result || resolvedConfig.search || searchValue || resolvedConfig.pagination) &&
		setFilteredResult()

	$: outputs.page.set($table.getState().pagination.pageIndex)

	function rerender() {
		if (!Array.isArray(result)) {
			return
		}

		const headers = Array.from(
			new Set(result.flatMap((row) => (typeof row == 'object' ? Object.keys(row) : [])))
		)

		$options = {
			...tableOptions,
			...(resolvedConfig?.pagination?.selected != 'manual'
				? {
						initialState: {
							pagination: {
								pageSize: resolvedConfig?.pagination?.configuration?.auto?.pageSize ?? 20
							}
						}
				  }
				: {}),
			manualPagination: resolvedConfig?.pagination?.selected == 'manual',
			pageCount:
				resolvedConfig?.pagination?.selected == 'manual'
					? resolvedConfig?.pagination?.configuration.manual.pageCount ?? -1
					: undefined,
			data: filteredResult,
			columns: headers.map((header) => {
				return {
					accessorKey: header,
					cell: (info) => info.getValue()
				}
			})
		}
		table = createSvelteTable(options)

		if (result) {
			//console.log('rerendering table', result[0])
			toggleRow({ original: filteredResult[0] }, 0, true)
		}

		if (outputs.page.peak()) {
			$table.setPageIndex(outputs?.page.peak())
		}
	}

	$: filteredResult && rerender()

	$: css = concatCustomCss($app.css?.tablecomponent, customCss)

	$componentControl[id] = {
		right: (skipActions: boolean | undefined) => {
			if (skipActions) {
				return false
			}

			const hasActions = actionButtons.length >= 1

			if (hasActions) {
				$selectedComponent = [actionButtons[0].id]
				return true
			}
			return false
		}
	}
</script>

{#each Object.keys(components['tablecomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<RunnableWrapper {outputs} {render} {componentInput} {id} bind:initializing bind:result>
	{#if Array.isArray(result) && result.every(isObject)}
		<div
			class={twMerge(
				'border border-gray-300 shadow-sm divide-y divide-gray-300 h-full',
				css?.container?.class ?? '',
				'flex flex-col'
			)}
			style={css?.container?.style ?? ''}
		>
			{#if resolvedConfig.search !== 'Disabled'}
				<div class="px-2 py-1">
					<div class="flex items-center">
						<div class="grow max-w-[300px]">
							<DebouncedInput placeholder="Search..." bind:value={searchValue} />
						</div>
					</div>
				</div>
			{/if}

			<div class="overflow-x-auto flex-1 w-full">
				<table class="relative w-full border-b border-b-gray-200">
					<thead
						class={twMerge(
							'bg-gray-50 text-left',
							css?.tableHeader?.class ?? '',
							'sticky top-0 z-40'
						)}
						style={css?.tableHeader?.style ?? ''}
					>
						{#each $table.getHeaderGroups() as headerGroup}
							<tr class="divide-x">
								{#each headerGroup.headers as header}
									{#if header?.column?.columnDef?.header}
										{@const context = header?.getContext()}
										{#if context}
											{@const component = renderCell(header.column.columnDef.header, context)}
											<th class="!p-0">
												<span class="block px-4 py-4 text-sm font-semibold border-b">
													{#if !header.isPlaceholder && component}
														<svelte:component this={component} />
													{/if}
												</span>
											</th>
										{/if}
									{/if}
								{/each}
								{#if actionButtons.length > 0}
									<th class="!p-0">
										<span class="block px-4 py-4 text-sm font-semibold border-b"> Actions </span>
									</th>
								{/if}
							</tr>
						{/each}
					</thead>
					<tbody
						class={twMerge('divide-y divide-gray-200 bg-white', css?.tableBody?.class ?? '')}
						style={css?.tableBody?.style ?? ''}
					>
						{#each $table.getRowModel().rows as row, rowIndex (row.id)}
							<tr
								class={classNames(
									'last-of-type:!border-b-0',
									selectedRowIndex === rowIndex
										? 'bg-blue-100 hover:bg-blue-200'
										: 'hover:bg-blue-50',
									'divide-x w-full',
									selectedRowIndex === rowIndex
										? 'divide-blue-200 hover:divide-blue-300'
										: 'divide-gray-200'
								)}
							>
								{#each row.getVisibleCells() as cell, index (index)}
									{#if cell?.column?.columnDef?.cell}
										{@const context = cell?.getContext()}
										{#if context}
											{@const component = renderCell(cell.column.columnDef.cell, context)}
											<td
												on:keydown={() => toggleRow(row, rowIndex)}
												on:click={() => toggleRow(row, rowIndex)}
												class="p-4 whitespace-pre-wrap truncate text-xs text-gray-900"
											>
												{#if typeof cell.column.columnDef.cell != 'string' && cellIsObject(cell.column.columnDef.cell, context)}
													{JSON.stringify(cell.column.columnDef.cell(context), null, 4)}
												{:else if component != undefined}
													<svelte:component this={component} />
												{/if}
											</td>
										{/if}
									{/if}
								{/each}

								{#if actionButtons.length > 0}
									<td
										class="p-2"
										on:keypress={() => toggleRow(row, rowIndex)}
										on:click={() => toggleRow(row, rowIndex)}
									>
										<div class="center-center h-full w-full flex-wrap gap-1.5">
											{#each actionButtons as actionButton, actionIndex (actionButton?.id)}
												<!-- svelte-ignore a11y-mouse-events-have-key-events -->
												<div
													on:mouseover|stopPropagation={() => {
														if (actionButton.id !== $hoverStore) {
															$hoverStore = actionButton.id
														}
													}}
													on:mouseout|stopPropagation={() => {
														if ($hoverStore !== undefined) {
															$hoverStore = undefined
														}
													}}
													class={($selectedComponent?.includes(actionButton.id) ||
														$hoverStore === actionButton.id) &&
													$mode !== 'preview'
														? 'outline outline-indigo-500 outline-1 outline-offset-1 relative '
														: ''}
												>
													{#if $mode !== 'preview'}
														<!-- svelte-ignore a11y-click-events-have-key-events -->
														<span
															title={`Id: ${actionButton.id}`}
															class={classNames(
																'px-2 text-2xs font-bold w-fit absolute shadow  -top-2 -left-2 border z-50 rounded-sm',
																'bg-indigo-500/90 border-indigo-600 text-white',
																$selectedComponent?.includes(actionButton.id) ||
																	$hoverStore === actionButton.id
																	? 'opacity-100'
																	: 'opacity-0'
															)}
															on:click|stopPropagation={() => {
																$selectedComponent = [actionButton.id]
															}}
														>
															{actionButton.id}
														</span>
													{/if}
													{#if rowIndex == 0}
														{@const controls = {
															left: () => {
																if (actionIndex === 0) {
																	$selectedComponent = [id]
																	return true
																} else if (actionIndex > 0) {
																	$selectedComponent = [actionButtons[actionIndex - 1].id]
																	return true
																}
																return false
															},
															right: () => {
																if (actionIndex === actionButtons.length - 1) {
																	return id
																} else if (actionIndex < actionButtons.length - 1) {
																	$selectedComponent = [actionButtons[actionIndex + 1].id]
																	return true
																}
																return false
															}
														}}
														{#if actionButton.type == 'buttoncomponent'}
															<AppButton
																extraKey={'idx' + rowIndex}
																{render}
																noWFull
																preclickAction={async () => {
																	toggleRow(row, rowIndex)
																}}
																id={actionButton.id}
																customCss={actionButton.customCss}
																configuration={actionButton.configuration}
																recomputeIds={actionButton.recomputeIds}
																extraQueryParams={{ row: row.original }}
																componentInput={actionButton.componentInput}
																{controls}
															/>
														{:else if actionButton.type == 'checkboxcomponent'}
															<AppCheckbox
																extraKey={'idx' + rowIndex}
																{render}
																id={actionButton.id}
																customCss={actionButton.customCss}
																configuration={actionButton.configuration}
																recomputeIds={actionButton.recomputeIds}
																preclickAction={async () => {
																	toggleRow(row, rowIndex)
																}}
																{controls}
															/>
														{:else if actionButton.type == 'selectcomponent'}
															<AppSelect
																extraKey={'idx' + rowIndex}
																{render}
																id={actionButton.id}
																customCss={actionButton.customCss}
																configuration={actionButton.configuration}
																recomputeIds={actionButton.recomputeIds}
																preclickAction={async () => {
																	toggleRow(row, rowIndex)
																}}
																{controls}
															/>
														{/if}
													{:else if actionButton.type == 'buttoncomponent'}
														<AppButton
															extraKey={'idx' + rowIndex}
															{render}
															noWFull
															id={actionButton.id}
															customCss={actionButton.customCss}
															configuration={actionButton.configuration}
															recomputeIds={actionButton.recomputeIds}
															preclickAction={async () => {
																toggleRow(row, rowIndex)
															}}
															extraQueryParams={{ row: row.original }}
															componentInput={actionButton.componentInput}
														/>
													{:else if actionButton.type == 'checkboxcomponent'}
														<AppCheckbox
															extraKey={'idx' + rowIndex}
															{render}
															id={actionButton.id}
															customCss={actionButton.customCss}
															configuration={actionButton.configuration}
															recomputeIds={actionButton.recomputeIds}
															preclickAction={async () => {
																toggleRow(row, rowIndex)
															}}
														/>
													{:else if actionButton.type == 'selectcomponent'}
														<AppSelect
															extraKey={'idx' + rowIndex}
															{render}
															id={actionButton.id}
															customCss={actionButton.customCss}
															configuration={actionButton.configuration}
															recomputeIds={actionButton.recomputeIds}
															preclickAction={async () => {
																toggleRow(row, rowIndex)
															}}
														/>
													{/if}
												</div>
											{/each}
										</div>
									</td>
								{/if}
							</tr>
						{/each}
					</tbody>
				</table>
			</div>

			<AppTableFooter
				pageSize={resolvedConfig?.pagination?.configuration?.auto?.pageSize ?? 20}
				manualPagination={resolvedConfig?.pagination?.selected == 'manual'}
				result={filteredResult}
				{table}
				class={css?.tableFooter?.class}
				style={css?.tableFooter?.style}
			/>
		</div>
	{:else if result != undefined}
		<div class="flex flex-col h-full w-full overflow-auto">
			<Alert title="Parsing issues" type="error" size="xs" class="h-full w-full ">
				The result should be an array of objects. Received:
				<pre class="w-full bg-white p-2 rounded-md whitespace-pre-wrap"
					>{JSON.stringify(result, null, 4)}</pre
				>
			</Alert>
		</div>
	{/if}
</RunnableWrapper>
