<script>
	import { tippy } from '../tippy';
	import { _ } from 'svelte-i18n';
	import { Gear } from 'akar-icons-svelte';

	let active = false;

	const toggle = e => {
		if (!e) {
			active = !active;
		} else if (e.target?.tagName === 'MODAL') {
			active = false;
		}
	};
</script>

<modal class:active={active} on:click={toggle}>
	<div class="content hide-scroll">
		sneed
	</div>
</modal>

<span class="nav-icon" use:tippy={$_('settings')} on:click={() => toggle()}><Gear size="1.5rem" stroke-width="1.5" id="settings"/></span>

<style>
	modal {
		position: fixed;
		width: 100%;
		height: 100%;
		z-index: 999;
		pointer-events: none;
		top: 0;
		left: 0;
		transition: background-color .25s, backdrop-filter .25s;
	}
	@media (max-width: 1000px), (max-height: 700px) {
		modal {
			width: 100%;
			height: 100%;
		}
	}
	modal > .content {
		position: fixed;
		max-width: 100%;
   		max-height: 100%;
		width: 1000px;
		height: 700px;
		margin: auto;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
	}

	modal.active {
		pointer-events: all;
		background-color: rgba(0,0,0,.6);
		backdrop-filter: blur(2px);
		animation: modal .25s;
	}

	modal > .content {
		background-color: #131313;
		box-shadow: rgba(0, 0, 0, .24) 0px 3px 8px;
		transition: transform .25s, opacity .25s;
	}
	modal:not(.active) > .content {
		transform: scale(0, 0);
		opacity: 0;
	}
	modal.active > .content {
		transform: scale(1, 1);
		opacity: 1;
		animation: modal-content .25s;
	}
	@keyframes modal {
		from {
			background-color: rgba(0,0,0,0);
			backdrop-filter: blur(0px);
		}
		to {
			background-color: rgba(0,0,0,.6);
			backdrop-filter: blur(2px);
		}
	}
	@keyframes modal-content {
		from {
			transform: scale(0, 0);
			opacity: 0;
		}
		to {
			transform: scale(1, 1);
			opacity: 1;
		}
	}
</style>
