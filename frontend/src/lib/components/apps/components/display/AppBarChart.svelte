<script lang="ts">
	import type { ChartOptions } from 'chart.js'
	import {
		BarElement,
		CategoryScale,
		Chart as ChartJS,
		Legend,
		LinearScale,
		LineElement,
		PointElement,
		Title,
		Tooltip
	} from 'chart.js'
	import { getContext } from 'svelte'
	import { Bar, Line } from 'svelte-chartjs'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let configuration: RichConfigurations
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'barchartcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	let outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	ChartJS.register(
		Title,
		Tooltip,
		Legend,
		LineElement,
		LinearScale,
		PointElement,
		CategoryScale,
		BarElement
	)

	let result: { data: number[]; labels?: string[] } | undefined = undefined
	let theme: string = 'theme1'
	let lineChart = false

	$: backgroundColor = {
		theme1: ['#FF6384', '#4BC0C0', '#FFCE56', '#E7E9ED', '#36A2EB'],
		// blue theme
		theme2: ['#4e73df', '#1cc88a', '#36b9cc', '#f6c23e', '#e74a3b'],
		// red theme
		theme3: ['#e74a3b', '#4e73df', '#1cc88a', '#36b9cc', '#f6c23e']
	}[theme]

	const lineOptions: ChartOptions<'line'> = {
		responsive: true,
		animation: false,
		maintainAspectRatio: false,
		plugins: {
			legend: {
				display: false
			}
		}
	}

	const barOptions: ChartOptions<'bar'> = {
		responsive: true,
		animation: false,
		maintainAspectRatio: false,
		plugins: {
			legend: {
				display: false
			}
		}
	}

	$: data = {
		labels: result?.labels ?? [],
		datasets: [
			{
				data: result?.data ?? [],
				backgroundColor
			}
		]
	}

	$: css = concatCustomCss($app.css?.barchartcomponent, customCss)
</script>

<InputValue {id} input={configuration.theme} bind:value={theme} />
<InputValue {id} input={configuration.line} bind:value={lineChart} />

<RunnableWrapper {outputs} {render} autoRefresh {componentInput} {id} bind:initializing bind:result>
	<div class="w-full h-full {css?.container?.class ?? ''}" style={css?.container?.style ?? ''}>
		{#if result}
			{#if lineChart}
				<Line {data} options={lineOptions} />
			{:else}
				<Bar {data} options={barOptions} />
			{/if}
		{/if}
	</div>
</RunnableWrapper>
