<script>
	import { _ } from 'svelte-i18n';
	import { Check } from 'akar-icons-svelte';
	import { onDestroy } from 'svelte';

	let self;
	let bye = false;
	let timeout = setTimeout(() => {
		bye = true;
		timeout = setTimeout(() => self.remove(), 2500);
	}, 500);
	onDestroy(() => clearTimeout(timeout));
</script>

<div id="saved-toast" bind:this={self} class:bye={bye}><Check size=".9rem"/>{$_('settings.saved')}</div>

<style>
	#saved-toast {
		position: absolute;
		bottom: 1.5rem;
		margin-left: auto;
		margin-right: auto;
		left: 1rem;
		right: 1rem;
		width: max-content;
		height: max-content;
		padding: .4rem;
		background: #21a049;
		border-radius: .3rem;
		min-width: 10rem;
		max-width: calc(100% - 2rem);
		font-size: .8em;
		display: flex;
		justify-content: center;
		align-items: center;
		box-shadow: 0 0 2px #000;
		color: #fff;

		animation: saved .5s;
	}
	#saved-toast.bye {
		animation: bye .5s;
		animation-delay: 2s;
		animation-fill-mode: forwards;
	}
	#saved-toast > :global(.icon) {
		margin-right: .2rem;
	}

	@keyframes saved {
		from {
			transform: scale(0);
			bottom: calc(0rem - 1.6em - .8rem);
		}
		to {
			transform: scale(1);
			bottom: 1.5rem;
		}
	}
	@keyframes bye {
		from {
			transform: scale(1);
			bottom: 1.5rem;
		}
		to {
			transform: scale(0);
			bottom: calc(0rem - 1.6em - .8rem);
		}
	}
</style>
