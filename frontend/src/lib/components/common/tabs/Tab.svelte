<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { TabsContext } from './Tabs.svelte'

	export let value: string
	export let size: 'xs' | 'sm' | 'md' | 'lg' | 'xl' = 'sm'
	let c = ''
	export { c as class }
	export let style = ''
	export let selectedClass = ''
	export let selectedStyle = ''

	export let disabled: boolean = false

	const fontSizeClasses = {
		xs: 'text-xs',
		sm: 'text-sm',
		md: 'text-md',
		lg: 'text-lg',
		xl: 'text-xl'
	}

	const { selected, update, hashNavigation } = getContext<TabsContext>('Tabs')
</script>

<button
	class={twMerge(
		'border-b-2 py-1 px-4 cursor-pointer transition-all z-10 ease-linear font-medium',
		$selected?.startsWith(value)
			? 'border-gray-600 text-gray-800 '
			: 'border-gray-300 border-opacity-0 hover:border-opacity-100 text-gray-600',
		fontSizeClasses[size],
		c,
		$selected?.startsWith(value) ? selectedClass : '',
		disabled ? 'cursor-not-allowed text-gray-400' : ''
	)}
	style={`${style} ${$selected?.startsWith(value) ? selectedStyle : ''}`}
	on:click={() => {
		if (hashNavigation) {
			window.location.hash = value
		} else {
			update(value)
		}
	}}
	on:pointerdown|stopPropagation
	{disabled}
>
	<slot />
</button>
