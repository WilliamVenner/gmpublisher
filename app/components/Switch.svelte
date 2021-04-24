<script>
	import { playSound, stopSound } from "../sounds";

	export let id;
	export let value = false;
	export let beforeChange = null;
	export let afterChange = null;

	let checked = value;

	let checkbox;
	async function toggle(e) {
		e.preventDefault();
		e.stopPropagation();

		const newValue = !checked;

		if (beforeChange) {
			const override = await beforeChange(newValue);
			if (override != newValue) return;
		}

		checkbox.checked = newValue;
		checked = newValue;

		stopSound('btn-on');
		stopSound('btn-off');
		playSound(checked ? 'btn-on' : 'btn-off');

		if (afterChange) afterChange.call(checkbox);
	}
</script>

<span class="switch-container">
	<span class="switch" on:click={toggle} class:checked={checked}>
		<input type="checkbox" id={id} name={id} checked={value ? true : null} bind:this={checkbox}/>
		<div class="circle"></div>
	</span>
	<label for={id}><slot></slot></label>
</span>

<style>
	.switch-container {
		display: inline-flex;
		cursor: pointer;
	}
	.switch-container > label {
		margin-left: .4rem;
		cursor: pointer;
	}

	.switch > input {
		visibility: hidden;
		position: absolute;
		width: 0;
		height: 0;
		pointer-events: none;
	}
	.switch {
		position: relative;
		width: 2.5rem;
		height: 1.1rem;
		border-radius: 1rem;
		transition: background-color .25s;
		display: inline-flex;
		box-shadow: inset 0 0 3px 0px rgb(0 0 0 / 50%);
	}
	.switch > .circle {
		position: absolute;
		top: 0;
		left: 0;
		border-radius: 100%;
		width: .9rem;
		height: .9rem;
		margin: .1rem;
		background-color: #fff;
		transition: left .25s;
	}
	.switch.checked {
		background-color: #009aff;
	}
	.switch:not(.checked) {
		background-color: #949494;
	}
	.switch.checked > .circle {
		left: calc(100% - .9rem - .2rem);
	}
	.switch:not(.checked) > .circle {
		left: 0;
	}
</style>
