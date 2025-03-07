<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import { faGithub, faGitlab, faGoogle, faMicrosoft } from '@fortawesome/free-brands-svg-icons'
	import { onMount } from 'svelte'
	import { OauthService, SettingsService, UserService, WorkspaceService } from '$lib/gen'
	import { clearStores, usersWorkspaceStore, workspaceStore, userStore } from '$lib/stores'
	import { classNames, parseQueryParams } from '$lib/utils'
	import { getUserExt } from '$lib/user'
	import { Button, Skeleton } from '$lib/components/common'
	import { WindmillIcon } from '$lib/components/icons'
	import { sendUserToast } from '$lib/toast'
	import { isCloudHosted } from '$lib/cloud'
	import { refreshSuperadmin } from '$lib/refreshUser'

	let email = $page.url.searchParams.get('email') ?? ''
	let password = $page.url.searchParams.get('password') ?? ''
	const error = $page.url.searchParams.get('error') ?? undefined
	const rd = $page.url.searchParams.get('rd')
	const providers = [
		{
			type: 'github',
			name: 'GitHub',
			icon: faGithub
		},
		{
			type: 'gitlab',
			name: 'GitLab',
			icon: faGitlab
		},
		{
			type: 'google',
			name: 'Google',
			icon: faGoogle
		},
		{
			type: 'microsoft',
			name: 'Microsoft',
			icon: faMicrosoft
		}
	] as const

	const providersType = providers.map((p) => p.type as string)

	let showPassword = false
	let logins: string[] | undefined = undefined

	async function login(): Promise<void> {
		const requestBody = {
			email,
			password
		}

		try {
			await UserService.login({ requestBody })
		} catch (err) {
			sendUserToast('Invalid credentials', true)
			return
		}

		// Once logged in, we can fetch the workspaces
		$usersWorkspaceStore = await WorkspaceService.listUserWorkspaces()
		// trigger a reload of the user
		if ($workspaceStore) {
			$userStore = await getUserExt($workspaceStore)
		}

		// Finally, we check whether the user is a superadmin
		refreshSuperadmin()
		redirectUser()
	}

	async function redirectUser() {
		const firstTimeCookie =
			document.cookie.match('(^|;)\\s*first_time\\s*=\\s*([^;]+)')?.pop() || '0'
		if (Number(firstTimeCookie) > 0 && email === 'admin@windmill.dev') {
			goto('/user/first-time')
			return
		}

		if (rd?.startsWith('http')) {
			goto(rd)
			return
		}
		if ($workspaceStore) {
			goto(rd ?? '/')
		} else {
			let workspaceTarget = parseQueryParams(rd ?? undefined)['workspace']
			if (rd && workspaceTarget) {
				$workspaceStore = workspaceTarget
				goto(rd)
				return
			}

			if (!$usersWorkspaceStore) {
				try {
					usersWorkspaceStore.set(await WorkspaceService.listUserWorkspaces())
				} catch {}
			}

			const allWorkspaces = $usersWorkspaceStore?.workspaces

			if (allWorkspaces?.length == 1) {
				$workspaceStore = allWorkspaces[0].id
				goto('/')
				return
			}
			if (rd?.startsWith('/user/workspaces')) {
				goto(rd)
			} else {
				goto(`/user/workspaces${rd ? `?rd=${encodeURIComponent(rd)}` : ''}`)
			}
		}
	}

	async function loadLogins() {
		logins = await OauthService.listOAuthLogins()
		showPassword = logins.length == 0
	}

	onMount(async () => {
		try {
			loadLogins()
			await UserService.getCurrentEmail()
			redirectUser()
		} catch {
			clearStores()
		}
	})

	function handleKeyUp(event: KeyboardEvent) {
		const key = event.key

		if (key === 'Enter') {
			event.preventDefault()
			login()
		}
	}

	function storeRedirect(provider: string) {
		if (rd) {
			try {
				localStorage.setItem('rd', rd)
			} catch (e) {
				console.error('Could not persist redirection to local storage', e)
			}
		}
		goto('/api/oauth/login/' + provider)
	}

	$: if (error) {
		sendUserToast(error, true)
	}

	let version = ''

	onMount(async () => {
		version = await SettingsService.backendVersion()
	})
</script>

<div class="flex flex-col justify-center py-12 sm:px-6 lg:px-8 relative bg-gray-50 h-screen">
	<div class="absolute top-0 right-0 text-2xs text-gray-800 italic px-3 py-1">
		<span class="font-mono">{version}</span>
	</div>
	<div class="sm:mx-auto sm:w-full sm:max-w-md">
		<div class="mx-auto flex justify-center">
			<WindmillIcon height="80px" width="80px" spin="slow" />
		</div>
		<h2 class="mt-6 text-center text-3xl font-bold tracking-tight text-gray-900">
			Login or sign up
		</h2>
		<p class="mt-2 text-center text-sm text-gray-600">
			Login or sign up with any of the methods below
		</p>
	</div>

	<div
		class={classNames('mt-8 sm:mx-auto sm:w-full sm:max-w-xl', showPassword ? 'mb-16' : 'mb-48')}
	>
		<div class="bg-white px-4 py-8 shadow md:border sm:rounded-lg sm:px-10">
			<div class="grid {logins && logins.length > 2 ? 'grid-cols-2' : ''} gap-4">
				{#if !logins}
					{#each Array(4) as _}
						<Skeleton layout={[0.5, [2.375]]} />
					{/each}
				{:else}
					{#each providers as { type, icon, name }}
						{#if logins.includes(type)}
							<Button
								color="dark"
								variant="border"
								endIcon={{ icon }}
								btnClasses="w-full !border-gray-300"
								on:click={() => storeRedirect(type)}
							>
								{name}
							</Button>
						{/if}
					{/each}
					{#each logins.filter((x) => !providersType.includes(x)) as login}
						<Button
							color="dark"
							variant="border"
							btnClasses="mt-2 w-full !border-gray-300"
							on:click={() => storeRedirect(login)}
						>
							{login}
						</Button>
					{/each}
				{/if}
			</div>
			{#if logins && logins.length > 0}
				<div class={classNames('center-center', logins.length > 0 ? 'mt-6' : '')}>
					<Button
						size="xs"
						color="blue"
						variant="border"
						btnClasses="!border-none"
						on:click={() => {
							showPassword = !showPassword
						}}
					>
						Login without third-party
					</Button>
				</div>
			{/if}

			{#if showPassword}
				<div>
					<div class="space-y-6">
						{#if isCloudHosted()}
							<p class="text-xs text-gray-500 italic pb-6">
								To get credentials without the OAuth providers above, send an email at
								contact@windmill.dev
							</p>
						{/if}
						<div>
							<label for="email" class="block text-sm font-medium leading-6 text-gray-900">
								Email
							</label>
							<div>
								<input
									type="email"
									bind:value={email}
									id="email"
									autocomplete="email"
									class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-frost-600 sm:text-sm sm:leading-6"
								/>
							</div>
						</div>

						<div>
							<label for="password" class="block text-sm font-medium leading-6 text-gray-900">
								Password
							</label>
							<div>
								<input
									on:keyup={handleKeyUp}
									bind:value={password}
									id="password"
									type="password"
									autocomplete="current-password"
									class="block w-full rounded-md border-0 py-1.5 text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 placeholder:text-gray-400 focus:ring-2 focus:ring-inset focus:ring-frost-600 sm:text-sm sm:leading-6"
								/>
							</div>
						</div>

						<div class="pt-2">
							<button
								on:click={login}
								disabled={!email || !password}
								class="flex w-full justify-center rounded-md bg-frost-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-frost-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-frost-600"
							>
								Sign in
							</button>
						</div>
					</div>

					{#if isCloudHosted()}
						<p class="text-2xs text-gray-500 italic mt-10 text-center">
							By logging in, you agree to our
							<a href="https://docs.windmill.dev/terms_of_service" target="_blank" rel="noreferrer">
								Terms of Service
							</a>
							and
							<a href="https://docs.windmill.dev/privacy_policy" target="_blank" rel="noreferrer">
								Privacy Policy
							</a>
						</p>
					{/if}
				</div>
			{/if}
		</div>
	</div>
</div>
