<script>
	import { tippy } from '../tippy';
	import { _ } from 'svelte-i18n';
	import { Rss, TriangleAlert } from 'akar-icons-svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import { listen } from '@tauri-apps/api/event';
	import Search from './Search.svelte';
	import Notifications from './Notifications.svelte';
	import Settings from './Settings.svelte';

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
		<span id="greeting">{$_('loading')}</span>
	{:then [name, avatar]}
		<img src="data:image/png;base64,{avatar}" id="avatar"/>
		<span id="greeting">{$_('greetings.' + timeOfDay, { values: { name } })}</span>
	{:catch}
		<img src="/img/steam_anonymous.jpg" id="avatar"/>
		<span id="greeting">{$_('greetings.' + timeOfDay + '.anon')}</span>
	{/await}

	<Search/>

	{#if steamConnected}
		<span class="nav-icon" use:tippy={'✔ ' + $_('steam_connected')}><Rss id="steam-connection" size="1.5rem" stroke-width="1.5"/></span>
	{:else}
		<span class="nav-icon" use:tippy={'❌ ' + $_('steam_disconnected')}><TriangleAlert class="icon error" id="steam-connection" size="1.5rem" stroke-width="1.5"/></span>
	{/if}

	<Notifications/>

	<Settings/>
</nav>

<style>
	nav {
		position: fixed;
		width: 100%;
		height: 70px;
		min-height: 70px;
		max-height: 70px;
		padding: .8rem;
		padding-right: 1.5rem;
		background-color: #323232;
		box-shadow: 0px 0px 10px rgba(0,0,0,0.4);
		overflow: hidden;
		top: 0;
		left: 0;
		display: flex;
		align-items: center;
		z-index: 998;
	}
	#avatar {
		border-radius: 50%;
		margin-right: 1rem;
		height: 100%;
	}

	:global(#steam-connection.error) {
		color: red;
		animation: steam-connection-error 1.5s infinite alternate;
		opacity: 1 !important;
	}

	:global(nav .nav-icon:not(:last-child)) {
		margin-right: 1rem;
	}
	:global(nav .nav-icon) {
		opacity: .4;
		transition: opacity .1s;
	}
	:global(nav .nav-icon:hover) {
		opacity: 1;
	}

	:global(.nav-icon > #settings) {
		cursor: pointer;
	}

	#greeting {
		font-size: 1.2em;
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
