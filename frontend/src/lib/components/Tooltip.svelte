<script lang="ts">
	import { faInfoCircle } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import type { PopoverPlacement } from './Popover.model'
	import Popover from './Popover.svelte'
	import { ExternalLink } from 'lucide-svelte'

	export let light = false
	export let scale = 0.8
	export let wrapperClass = ''
	export let placement: PopoverPlacement | undefined = undefined
	export let documentationLink: string | undefined = undefined
</script>

<Popover notClickable {placement} class={wrapperClass}>
	<Icon
		class="{light
			? 'text-gray-400 hover:text-gray-500'
			: ' text-gray-500 hover:text-gray-600'}  cursor-pointer transition-all font-thin flex h-4 p-0.5 w-4 justify-center items-center {$$props.class}"
		data={faInfoCircle}
		{scale}
	/>
	<svelte:fragment slot="text">
		<slot />
		{#if documentationLink}
			<a href={documentationLink} target="_blank" class="text-blue-300 text-xs">
				<div class="flex flex-row gap-2 mt-4">
					See documentation
					<ExternalLink size="16" />
				</div>
			</a>
		{/if}
	</svelte:fragment>
</Popover>
