<script>
	import { _ } from 'svelte-i18n';
	import { Rss, TriangleAlert } from 'akar-icons-svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import { listen } from '@tauri-apps/api/event';
	import Search from './Search.svelte';

	let timeOfDay = 'morning';
	{
		const hours = new Date().getHours();

		// morning   = 04:00 - 11:59
		// afternoon = 12:00 - 16:59
		// evening   = 17:00 - 03:59

		if (hours >= 12 || hours < 4) {
			if (hours >= 12 && hours <= 16) {
				timeOfDay = 'afternoon';
			} else {
				timeOfDay = 'evening';
			}
		}
	}

	let steamConnected = false;
	{
		invoke('is_steam_connected').then(connected => {
			steamConnected = connected;
		});

		listen('SteamConnected', () => {
			steamConnected = true;
		});

		listen('SteamDisconnected', () => {
			steamConnected = false;
		});
	}

	let steamUser = invoke('get_steam_user');
</script>

<nav>
	{#await steamUser}
		<img src="/img/steam_anonymous.jpg" id="avatar"/>
		{$_('loading')}
	{:then [name, avatar]}
		<img src="data:image/png;base64,{avatar}" id="avatar"/>
		{$_('greetings.' + timeOfDay, { values: { name } })}
	{:catch}
		<img src="/img/steam_anonymous.jpg" id="avatar"/>
		{$_('greetings.' + timeOfDay + '.anon')}
	{/await}

	<Search/>

	{#if steamConnected}
		<TriangleAlert id="steam-connection" class="error" size="1.25rem"/>
	{:else}
		<Rss id="steam-connection" size="1.25rem"/>
	{/if}
</nav>

<style>
	nav {
		position: fixed;
		width: 100%;
		height: 70px;
		min-height: 70px;
		max-height: 70px;
		padding: .8rem;
		background-color: #323232;
		box-shadow: 0px 0px 10px rgba(0,0,0,0.4);
		overflow: hidden;
		top: 0;
		left: 0;
		display: flex;
		align-items: center;
		font-size: 1.1rem;
		z-index: 998;
	}
	#avatar {
		border-radius: 50%;
		margin-right: 1rem;
		height: 100%;
	}

	:global(#steam-connection) {
		color: red;
	}
	:global(#steam-connection.error) {
		animation: steam-connection-error 1.5s infinite alternate;
	}

	@keyframes steam-connection-error {
		0% {
			color: white;
		}
		100% {
			color: red;
		}
	}
</style>
