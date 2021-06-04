<script>
	export let id = null;
	export let active = false;
	export let cancel = null;
	export let padding = null;

	let modal;
	function doCancel(e) {
		if (cancel && e.target === modal)
			cancel();
	}

	function keydown(e) {
		if (active && document.activeElement === this && (e.keyCode === 27 || e.key === 'Escape')) {
			cancel();
		}
	}

	$: {
		if (modal) {
			if (active) {
				modal.focus();
			} else {
				modal.blur();
			}
		}
	}
</script>

<modal id={id} on:click={doCancel} bind:this={modal} class:active={active} on:keydown={keydown} tabindex="0">
	<div class="hide-scroll" style={padding ? ('padding:' + padding) : null}>
		<slot></slot>
	</div>
</modal>

<style>
	modal {
		pointer-events: none;

		transition: backdrop-filter .25s, background-color .25s;
		background-color: rgba(0,0,0,0);

		z-index: 4;
		position: fixed;
		width: 100%;
		height: 100%;
		top: 0;
		left: 0;
		z-index: 999;
	}
	modal.active {
		pointer-events: all;

		backdrop-filter: grayscale(.5) blur(1px);
		background-color: rgba(0,0,0,.4);
	}
	modal.active > div {
		transform: scale(1, 1);
	}
	modal > div {
		transition: transform .25s;
		transform: scale(0, 0);

		position: absolute;
		top: 0;
		left: 0;
		bottom: 0;
		right: 0;
		margin: auto;

		background-color: #1a1a1a;
		border-radius: .3rem;
		box-shadow: 0 0 10px rgba(0, 0, 0, .25);

		width: min-content;
		height: min-content;
		max-width: 90%;
		max-height: 90%;
	}
</style>
