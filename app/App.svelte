<script>
	import { _ } from 'svelte-i18n';
	import TasksOverlay from './components/TasksOverlay.svelte';
	import FileDrop from './components/FileDrop.svelte';
	import Navbar from './components/Navbar.svelte';
	import Sidebar, { pages, activePage } from './components/Sidebar.svelte';

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
</script>

<FileDrop/>

<main>

	<TasksOverlay/>

	<Navbar/>

	<Sidebar/>

	<div id="content">
		{#if !activePage.persist}
			<svelte:component this={activePage.component}/>
		{/if}
		{#each pages as page}
			{#if page.persist && page.created}
				<div class="persist" class:active={activePage == page}><svelte:component this={page.component}/></div>
			{/if}
		{/each}
	</div>

</main>

<style>
	#file-drop {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		opacity: 0;
		display: flex;
		justify-content: center;
		align-items: center;
		background-color: rgba(0,0,0,.4);
		backdrop-filter: grayscale(0) blur(0px);
		pointer-events: none;
		transition: opacity .25s;
		z-index: 9999;
	}
	:global(body.file-drop #file-drop) {
		opacity: 1 !important;
		backdrop-filter: grayscale(.5) blur(1px) !important;
	}
	#file-drop :global(.icon) {
		width: min(50vw, 50vh);
	}

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
	#sources > div > .persist {
		height: 100%;
	}
	#sources > div > .persist:not(.active) {
		display: none;
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
