<script>
	import { Folder } from "akar-icons-svelte";
	import Switch from "./Switch.svelte";

	export let id;
	export let type;
	export let value;
	export let initial = null;
	export let beforeChange = null;
</script>

<setting>
	{#if type === 'bool'}
		<div><Switch {id} {value} {beforeChange}><slot></slot></Switch></div>
	{:else}
		<label class="name" for={id}><slot></slot></label>
		{#if type === 'path'}
			<div class="path-container">
				<input type="text" {id} name={id} placeholder={initial} {value}/>
				<div class="browse icon-button"><Folder size="1rem"/></div>
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
		margin-bottom: 1rem;
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
</style>
