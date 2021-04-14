<script>
	import { _ } from 'svelte-i18n';
	import TasksOverlay from './components/TasksOverlay.svelte';
	import FileDrop from './components/FileDrop.svelte';
	import Navbar from './components/Navbar.svelte';
	import Sidebar, { pages, activePage } from './components/Sidebar.svelte';
</script>

<FileDrop/>

<main>

	<TasksOverlay/>

	<Navbar/>

	<Sidebar/>

	<div id="content">
		{#if !$activePage.persist}
			<svelte:component this={$activePage.component}/>
		{/if}
		{#each $pages as page}
			{#if page.persist && page.created}
				<div class="persist" class:active={$activePage == page}><svelte:component this={page.component}/></div>
			{/if}
		{/each}
	</div>

</main>

<style>
	/*
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
	*/

	main {
		width: 100%;
		height: 100%;
	}

	#content {
		width: 100%;
		height: 100%;
		padding-top: 70px;
		padding-left: min(26.04%, 250px);
	}
	#content .persist {
		height: 100%;
	}
	#content .persist:not(.active) {
		display: none;
	}
</style>
