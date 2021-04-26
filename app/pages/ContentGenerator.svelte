<script>
	import { invoke } from "@tauri-apps/api/tauri";
	import { Download, Plus } from "akar-icons-svelte";
	import { _ } from 'svelte-i18n';
	import Dead from "../components/Dead.svelte";
	import Modal from "../components/Modal.svelte";
	import { tippy } from '../tippy';

	let bundles = invoke('get_bundles');
	let selectedBundle;

	let showHelp = false;

	let newBundle = false;
	function showNewBundle() {
		newBundle = true;
		// TODO remove prev values
	}

	function importBundle() {

	}
</script>

<main>
	<Modal id="new-bundle" active={newBundle} cancel={() => newBundle = false}>
		<h2>{$_('new_bundle')}</h2>
		<input type="text" placeholder={$_('bundle_name')} id="bundle-name"/>
		<div id="collection-link">
			<img src="/img/steam_anonymous.jpg"/>
			<span>{$_('based_on_collection')}</span>
			<input type="text" placeholder={$_('collection_link')}/>
		</div>
		<div class="btn" on:click={showNewBundle}><Plus size="1rem"/>{$_('create_exclamation')}</div>
	</Modal>

	{#await bundles}
		{$_('loading')}
	{:then bundles}
		{#if bundles.length === 0}
			<Modal id="bundles-help" active={showHelp} cancel={() => showHelp = false}>
				<h2>{$_('bundles_what')}</h2>
				<p>{$_('bundles_help')}</p>
			</Modal>
			<div id="no-bundles"><div>
				<Dead inline={true} size="4rem"/>
				<h2>{$_('no_bundles')}</h2>
				<a class="color" href="#" on:click={() => showHelp = true}>{$_('bundles_what')}</a><br>
				<div class="btn" on:click={showNewBundle}><Plus size="1rem"/>{$_('new_bundle')}</div>
				<div class="btn" on:click={importBundle}><Download size="1rem"/>{$_('import_bundle')}</div>
			</div></div>
		{:else}
			<div class="icon-btn" on:click={importBundle} use:tippy={$_('import_bundle')}><Download size="1rem"/></div>
			<div class="icon-btn" on:click={showNewBundle} use:tippy={$_('new_bundle')}><Plus size="1rem"/></div>

			<select>
				{#each bundles as bundle}
					{JSON.stringify(bundle)}
				{/each}
			</select>

			{#if selectedBundle}

			{/if}
		{/if}
	{:catch err}
		<Dead size="2rem"/>&nbsp;{err}
	{/await}
</main>

<style>
	main {
		height: 100%;
	}
	#no-bundles {
		height: 100%;
		display: flex;
		flex-direction: column;
		justify-content: center;
		align-items: center;
	}

	:global(#bundles-help > div) {
		padding: 1.5rem;
		width: 28rem;
	}
	:global(#bundles-help h2) {
		margin-top: 0;
	}
	p {
		white-space: pre-line;
		margin-bottom: 0;
		line-height: 1.6;
	}

	#no-bundles > div > a {
		margin-bottom: calc(2rem - 4px);
		display: inline-block;
	}
	#no-bundles > div {
		text-align: center;
	}
	#no-bundles > div > .btn {
		display: inline-flex;
		width: 9rem;
	}
	#no-bundles > div > .btn:not(:first-child) {
		margin-left: .5rem;
	}

	.btn {
		cursor: pointer;
		background: #313131;
		box-shadow: 0px 0px 2px 0px rgb(0 0 0 / 40%);
		border-radius: 4px;
		padding: .7rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.btn:active {
		background: #252525;
	}
	.btn > :global(.icon) {
		margin-right: .5rem;
	}

	input[type='text'] {
		appearance: none;
		font: inherit;
		border-radius: 4px;
		border: none;
		background: rgba(255,255,255,.1);
		box-shadow: 0px 0px 2px 0px rgba(0, 0, 0, .4);
		padding: .7rem;
		color: #fff;
		font-size: .85em;
		width: 100%;
	}
	input[type='text']:focus {
		box-shadow: inset 0 0 0px 1.5px #127cff;
		outline: none;
	}

	:global(#new-bundle > div) {
		padding: 1.5rem;
		width: 30rem;
	}
	:global(#new-bundle > div > .btn) {
		margin-top: 1.5rem;
	}
	:global(#new-bundle > div > h2) {
		margin-top: 0;
		margin-bottom: 1.5rem;
		text-align: center;
	}
	#collection-link {
		display: grid;
		grid-template-columns: min-content 1fr;
		grid-template-rows: 1fr min-content;
		grid-gap: 1rem;
		margin-top: 1.5rem;
	}
	#collection-link > img {
		grid-row: 1 / 3;
		grid-column: 1;
		height: 0;
		min-height: 100%;
	}
	#collection-link > span {
		grid-row: 1;
		grid-column: 2;
	}
	#collection-link > input {
		grid-row: 2;
		grid-column: 2;
	}
</style>
