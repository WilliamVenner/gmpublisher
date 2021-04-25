<script>
	import { Folder } from "akar-icons-svelte";
	import Switch from "./Switch.svelte";
	import * as dialog from '@tauri-apps/api/dialog';
	import { invoke } from '@tauri-apps/api/tauri';
	import { playSound } from "../sounds";

	export let id;
	export let type;
	export let value;
	export let choices = null;
	export let initial = null;
	export let beforeChange = null;
	export let afterChange = null;

	if (!beforeChange && type === 'directory') {
		beforeChange = async (before, after) => {
			if (after.trim().length > 0) {
				if (!(await invoke('check_dir', { path: after }))) {
					return before;
				} else {
					playSound('success');
				}
			}
			return after;
		};
	}

	let prevVal = '';

	function browse() {
		const input = this.parentNode.querySelector(':scope > input');

		dialog.open({
			defaultPath: input.value.trim().length > 0 ? input.value : (input.placeholder.length > 0 ? input.placeholder : null),
			directory: true,
		}).then(async path => {
			if (path) {
				if (input.value === path) return;
				if (beforeChange) {
					const checkedVal = await beforeChange(prevVal, path);
					if (checkedVal === prevVal) {
						playSound('error');
						return;
					} else {
						input.value = checkedVal;
						prevVal = input.value;
					}
				} else {
					input.value = path;
				}
				if (afterChange) afterChange.call(input);
			}
		});
	}

	async function change() {
		if (this.value === prevVal) return;
		if (beforeChange) {
			this.value = await beforeChange(prevVal, this.value);
			if (this.value === prevVal) {
				playSound('error');
				return;
			}
		}
		prevVal = this.value;
		if (afterChange) afterChange.call(this);
	}
</script>

<setting>
	{#if type === 'bool'}
		<div><Switch {id} {value} {beforeChange} {afterChange}><slot></slot></Switch></div>
	{:else if type === 'select'}
		<label class="name" for={id}><slot></slot></label>
		<select {id} {value} on:change={beforeChange || afterChange ? change : null} on:blur={beforeChange || afterChange ? change : null}>
			{#each choices as [key, label]}
				<option value={key}>{label}</option>
			{/each}
		</select>
	{:else}
		<label class="name" for={id}><slot></slot></label>
		{#if type === 'directory'}
			<div class="path-container">
				<input type="text" {id} name={id} placeholder={initial} {value} on:change={beforeChange || afterChange ? change : null} required={initial == null ? true : null}/>
				<div class="browse icon-button" on:click={browse}><Folder size="1rem"/></div>
			</div>
		{/if}
	{/if}
</setting>

<style>
	setting {
		display: flex;
		flex-direction: column;
	}
	setting > .name {
		font-size: .9em;
		padding-bottom: .8rem;
		text-shadow: 0px 1px 0px rgba(0, 0, 0, .6);
	}
	setting:not(:last-child) {
		margin-bottom: 1.5rem;
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
	}
	input[type='text']:focus {
		outline: none;
		box-shadow: inset 0 0 0px 1.5px #127cff;
	}
	.path-container {
		display: flex;
	}
	.path-container > input {
		flex: 1;
	}
	.browse {
		margin-left: .75rem;
		padding: .7rem;
		width: 2.4rem;
		height: 2.4rem;
		display: flex;
	}

	select {
		-webkit-appearance: none;
		-moz-appearance: none;
		appearance: none;
		font: inherit;
		border-radius: 4px;
		border: none;
		background: rgba(255,255,255,.1);
		box-shadow: 0px 0px 2px 0px rgb(0 0 0 / 40%);
		padding: .7rem;
		color: #fff;
		font-size: .85em;
		width: 100%;
		cursor: pointer;
		text-align: center;
		text-align-last: center;
	}
	select:focus {
		box-shadow: inset 0 0 0px 1.5px #127cff;
		outline: none;
	}
	option {
		background: #313131;
		color: #fff;
	}
	option:hover {
		background: #CECECE;
		color: #313131;
	}
</style>
