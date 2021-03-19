<script>
	import { tippy } from '../tippy.js';
	import { _ } from 'svelte-i18n';
	import { modals, clearModals } from '../modals.js';
	import WorkshopBrowser from '../pages/WorkshopBrowser.svelte';
	import GameAddonsBrowser from '../pages/GameAddonsBrowser.svelte';
	//import FileSystemAddonsBrowser from '../pages/FileSystemAddonsBrowser.svelte';
	import AddonSizeAnalyzer from '../pages/AddonSizeAnalyzer.svelte';
	import Tasks from '../modals/Tasks.svelte';

	const hours = new Date().getHours();

	// morning   = 04:00 - 11:59
	// afternoon = 12:00 - 16:59
	// evening   = 17:00 - 03:59

	let timeOfDay = 'morning';
	if (hours >= 12 || hours < 4) {
		if (hours >= 12 && hours <= 16) {
			timeOfDay = 'afternoon';
		} else {
			timeOfDay = 'evening';
		}
	}

	const sources = [
		{
			name: 'my_workshop',
			component: WorkshopBrowser,
			props: {}
		},
		{
			name: 'game',
			component: GameAddonsBrowser,
			props: {}
		},
		{
			name: 'filesystem',
			component: null,//FileSystemAddonsBrowser,
			props: {}
		},
		{
			name: 'size_analyzer',
			component: AddonSizeAnalyzer,
			props: {}
		},
	];

	let source = sources[0];
</script>

<main>

	<Tasks/>

	<div id="modals" class:active={$modals.length > 0} on:click={clearModals}>
		{#each $modals as modal}
			<svelte:component this={modal.component} {...modal.props}/>
		{/each}
	</div>

	<div id="ribbon">
		<a href="https://steamcommunity.com/profiles/{AppData.user.steamid64}" target="_blank">
			<img use:tippy={AppData.user.name} id="avatar" src={AppData.user.avatar ? ('data:image/png;base64,' + AppData.user.avatar) : '/img/steam_anonymous.jpg'} alt="Avatar"/>
		</a>
		<span id="greeting">
			{#if AppData.user?.name}
				{$_('greetings.' + timeOfDay, { values: { name: AppData.user.name } })}
			{:else}
				{$_('greetings.' + timeOfDay + '.anon')}
			{/if}
		</span>
	</div>

	<div id="sidebar">
		<div>
			{#each sources as choice, i}
				<div class:active={ source.name === choice.name } on:click="{ e => source = sources[e.target.dataset.choice] }" data-choice={i}>{$_(choice.name)}</div>
			{/each}
		</div>

		<div id="credits">
			<img src="/img/logo.svg" alt="Logo" id="logo" /><br>
			gmpublisher v{AppData.version} by Billy<br>
			<a href="https://github.com/WilliamVenner/gmpublisher/stargazers" target="_blank">{$_('github_star')}</a>&nbsp;🤩
		</div>
	</div>

	<div id="content">
		<div id="sources">
			<div>
				<svelte:component this={source.component} {...source.props}/>
			</div>
		</div>
	</div>

</main>

<style>
	main {
		width: 100%;
		height: 100%;
	}
	#ribbon {
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
	#ribbon a {
		height: 100%;
	}
	#ribbon #avatar {
		border-radius: 50%;
		height: 100%;
		margin-right: 1rem;
	}
	#ribbon span {
		flex: 1;
		overflow: hidden;
		text-overflow: hidden;
		white-space: nowrap;
	}

	#content {
		width: 100%;
		height: 100%;
		padding-top: 70px;
		padding-left: min(26.04%, 250px);
	}

	#sidebar {
		position: fixed;
		padding: 1.5rem;
		padding-right: 0;
		width: 26.04%;
		max-width: 250px;
		height: calc(100% - 70px);
		top: 70px;
		left: 0;
		display: flex;
		flex-direction: column;
	}
	#sidebar > div:first-child {
		flex: 1;
		overflow-y: auto;
		overflow-x: hidden;
	}
	#sidebar > div:first-child > div {
		padding: .5rem;
		padding-left: .7rem;
		padding-right: .7rem;
		cursor: pointer;
		border-radius: 4px;
	}
	#sidebar > div:first-child > div.active {
		background-color: #2A2A2A;
	}
	#sidebar > div:first-child > div:not(:last-child) {
		margin-bottom: .5rem;
	}

	#sources, #sources > div {
		height: 100%;
	}
	#sources > div {
		padding: 1.5rem;
		padding-bottom: 0;
	}

	#credits {
		text-align: center;
		font-size: .8rem;
		padding-top: 1rem;
		padding-bottom: 1rem;
		line-height: 1.8em;
	}
	#credits a {
		color: #636363;
		transition: color .25s;
	}
	#credits a:hover {
		color: #fff;
	}
	#logo {
		margin-bottom: .5rem;
		width: min(calc(100% - 1rem), 50px);
	}

	#modals {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		z-index: 999;
		pointer-events: none;

		display: flex;
		justify-content: center;
		align-items: center;

		transition: background-color .25s, backdrop-filter .25s;
	}
	#modals.active {
		pointer-events: initial;
		background-color: rgba(0,0,0,.5);
		backdrop-filter: blur(2px);
	}
	#modals > :global(*) {
		animation: modal .25s;
	}
</style>