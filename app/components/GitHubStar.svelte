<script>
	import { onMount } from "svelte";
	import Modal from './Modal.svelte';
	import { _ } from 'svelte-i18n';
	export let active = false;
	onMount(() => active = true);

	let gluaEnhanced;
	onMount(() => {
		gluaEnhanced.innerText = $_('vscode_glua_enhanced');
		gluaEnhanced.innerHTML = gluaEnhanced.innerHTML.replace('VSCODE_GLUA_ENHANCED_LINK', '<a class="color" href="https://marketplace.visualstudio.com/items?itemName=venner.vscode-glua-enhanced" target="_blank">VSCode GLua Enhanced</a>');
	});

	function pissOff() {
		active = false;
	}
</script>

<Modal id="github-star-modal" {active} cancel={pissOff}>
	<img src="/img/logo.svg" class="logo"/>
	<h2>{$_('enjoying_gmpublisher')}<img src="/img/dog.gif"/></h2>
	<p>{$_('github_star_plz_i_need_a_job_maybe')}<br><span bind:this={gluaEnhanced}></span></p>
	<div class="btn" on:mousedown={pissOff} on:click={pissOff}>Piss off</div>
</Modal>

<style>
	:global(#github-star-modal > .hide-scroll) {
		text-align: center;
		padding: 1.5rem;
		width: 31rem;
		height: 31rem;
	}
	:global(#github-star-modal p) {
		line-height: 2.4;
	}

	h2 img {
		margin-left: .5rem;
	}
	h2 {
		display: flex;
		justify-content: center;
		align-items: center;
		margin-top: 0;
	}

	.logo {
		transform: rotate(-10deg);
		margin-bottom: 1.5rem;
		margin-top: 1rem;
		width: 10rem;
	}
	.btn {
		cursor: pointer;
		background: #313131;
		box-shadow: 0px 0px 2px 0px rgb(0 0 0 / 40%);
		border-radius: 4px;
		padding: .7rem;
		margin-top: 1rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.btn:active {
		background: #252525;
	}
</style>
