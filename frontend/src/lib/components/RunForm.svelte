<script lang="ts">
	import {
		defaultIfEmptyString,
		displayDaysAgo,
		emptyString,
		getToday,
		truncateHash
	} from '$lib/utils'

	import type { Schema } from '$lib/common'
	import { runFormStore, userStore } from '$lib/stores'
	import CliHelpBox from './CliHelpBox.svelte'
	import { Badge, Button } from './common'
	import InlineCodeCopy from './InlineCodeCopy.svelte'
	import SchemaForm from './SchemaForm.svelte'
	import SharedBadge from './SharedBadge.svelte'
	import Toggle from './Toggle.svelte'
	import Tooltip from './Tooltip.svelte'
	import CollapseLink from './CollapseLink.svelte'
	import { SCRIPT_VIEW_SHOW_RUN_FROM_CLI, SCRIPT_VIEW_SHOW_SCHEDULE_RUN_LATER } from '$lib/consts'

	export let runnable:
		| {
				summary?: string
				schema?: Schema | any
				description?: string
				path?: string
				is_template?: boolean
				hash?: string
				kind?: string
				can_write?: boolean
				created_at?: string
				created_by?: string
				extra_perms?: Record<string, boolean>
		  }
		| undefined
	export let runAction: (
		scheduledForStr: string | undefined,
		args: Record<string, any>,
		invisible_to_owner?: boolean
	) => void
	export let buttonText = 'Run'
	export let schedulable = true
	export let detailed = true
	export let autofocus = false
	export let topButton = false
	export let loading = false
	export let noVariablePicker = false
	export let viewCliRun = false
	export let isFlow: boolean

	export let args: Record<string, any> = {}

	if ($runFormStore) {
		args = $runFormStore
		$runFormStore = undefined
	}

	export function run() {
		runAction(scheduledForStr, args, invisible_to_owner)
	}

	export let isValid = true

	let scheduledForStr: string | undefined
	let invisible_to_owner: false

	$: cliCommand = `wmill ${isFlow ? 'flow' : 'script'} run ${runnable?.path} -d '${JSON.stringify(
		args
	)}'`
</script>

<div class="max-w-3xl">
	{#if detailed}
		{#if runnable}
			<div class="flex flex-row flex-wrap justify-between gap-4">
				<div>
					<div class="flex flex-col mb-2">
						<h1 class="break-words py-2 mr-2">
							{defaultIfEmptyString(runnable.summary, runnable.path ?? '')}
						</h1>
						{#if !emptyString(runnable.summary)}
							<h2 class="font-bold pb-4">{runnable.path}</h2>
						{/if}

						<div class="flex items-center gap-2">
							<span class="text-sm text-gray-500">
								{#if runnable}
									Edited {displayDaysAgo(runnable.created_at || '')} by {runnable.created_by ||
										'unknown'}
								{/if}
							</span>
							<Badge color="dark-gray">
								{truncateHash(runnable?.hash ?? '')}
							</Badge>
							{#if runnable?.is_template}
								<Badge color="blue">Template</Badge>
							{/if}
							{#if runnable && runnable.kind !== 'runnable'}
								<Badge color="blue">
									{runnable?.kind}
								</Badge>
							{/if}
							<SharedBadge
								canWrite={runnable.can_write ?? true}
								extraPerms={runnable?.extra_perms ?? {}}
							/>
						</div>
					</div>
				</div>
			</div>
		{:else}
			<h1>Loading...</h1>
		{/if}
	{/if}
	{#if topButton}
		<Button
			btnClasses="!px-6 !py-1 w-full"
			disabled={!isValid}
			on:click={() => runAction(undefined, args)}
		>
			{buttonText}
		</Button>
	{/if}
	{#if runnable?.schema}
		<div class="my-2" />
		{#if !runnable.schema.properties || Object.keys(runnable.schema.properties).length === 0}
			<div class="text-sm p-4">No arguments</div>
		{:else}
			<SchemaForm
				prettifyHeader
				{noVariablePicker}
				{autofocus}
				schema={runnable.schema}
				bind:isValid
				bind:args
			/>
		{/if}
	{:else}
		<div class="text-xs text-gray-600">No arguments</div>
	{/if}
	{#if schedulable}
		<div class="mt-10" />
		<div class="flex gap-2 items-start flex-wrap justify-between mt-2 md:mt-6 mb-6">
			<div class="flex-row-reverse flex grow">
				<Button
					{loading}
					color="dark"
					btnClasses="!px-6 !py-1"
					disabled={!isValid}
					on:click={() => runAction(scheduledForStr, args, invisible_to_owner)}
				>
					{scheduledForStr ? 'Schedule to run later' : buttonText}
				</Button>
			</div>
		</div>
		<CollapseLink small text="Advanced">
			<div class="flex flex-col gap-4 mt-2 border p-2">
				<div class="flex flex-col gap-2">
					{#if SCRIPT_VIEW_SHOW_SCHEDULE_RUN_LATER}
						<div class="border rounded-md p-3 pt-4">
							<div class="px-2 font-semibold text-sm">Schedule to run later</div>

							<div class="flex flex-row items-end">
								<div class="w-max md:w-2/3 mt-2 mb-1">
									<label for="run-time" />
									<input
										class="inline-block"
										type="datetime-local"
										id="run-time"
										name="run-scheduled-time"
										bind:value={scheduledForStr}
										min={getToday().toISOString().slice(0, 16)}
									/>
								</div>
								<Button
									variant="border"
									color="blue"
									size="sm"
									btnClasses="mx-2 mb-1"
									on:click={() => {
										scheduledForStr = undefined
									}}
								>
									Clear
								</Button>
							</div>
						</div>
					{/if}
				</div>
				{#if runnable?.path?.startsWith(`u/${$userStore?.username}`) != true && (runnable?.path?.split('/')?.length ?? 0) > 2}
					<div class="flex items-center gap-1">
						<Toggle
							options={{
								right: `make run invisible to others`
							}}
							bind:checked={invisible_to_owner}
						/>
						<Tooltip
							>By default, runs are visible to the owner(s) of the script or flow being triggered</Tooltip
						>
					</div>
				{/if}
			</div>
		</CollapseLink>
	{:else if !topButton}
		<Button
			btnClasses="!px-6 !py-1 w-full"
			disabled={!isValid}
			on:click={() => runAction(undefined, args, invisible_to_owner)}
		>
			{buttonText}
		</Button>
	{/if}

	{#if viewCliRun}
		<div>
			<div class="mt-4" />
			{#if SCRIPT_VIEW_SHOW_RUN_FROM_CLI}
				<CollapseLink small text="Run it from CLI">
					<div class="mt-2" />
					<InlineCodeCopy content={cliCommand} />
					<CliHelpBox />
				</CollapseLink>
			{/if}
			<div class="mb-20" />
		</div>
	{/if}
</div>
