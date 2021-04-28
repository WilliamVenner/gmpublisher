<svelte:options accessors={true}/>

<script>
	import { Cross, Check, CircleAlert } from "akar-icons-svelte";
	import { tippyFollow } from '../tippy.js';
	import { _ } from 'svelte-i18n';
	import { translateError } from '../i18n';
	import { taskHeight, tasksMax, tasks, tasksNum } from '../transactions.js';
	import { onDestroy, onMount } from "svelte";
	import Loading from './Loading.svelte';

	export let transaction;
	export let statusTextFn;
	export let pos;

	export let expired = false;
	export let destroyed = false;

	let progress = 0;
	let error = null;
	let finished = false;
	let cancelled = false;

	let taskElem;

	let y = 0;
	export let shift = () => {
		let swingPos = pos;
		if ($tasksNum < $tasksMax)
			swingPos = (($tasksMax - $tasksNum) + pos);

		y = (swingPos * taskHeight) + (swingPos * (taskHeight / 2));
	}
	shift();
	let subscriptions = [tasksNum.subscribe(shift), tasksMax.subscribe(shift), tasks.subscribe(shift)];

	function unsubscribe() {
		for (let i = 0; i < subscriptions.length; i++)
			subscriptions[i]();
	}

	function destroy() {
		if (destroyed) return;
		unsubscribe();
		destroyed = true;
		$tasks = $tasks;
	}
	onDestroy(destroy);

	function expire() {
		if (destroyed || expired) return;
		expired = true;
		setTimeout(destroy, 500);
	}

	function finish() {
		if (finished || destroyed || expired) return;
		finished = true;
		let delay = setTimeout(expire, 2500);
		subscriptions.push(() => clearTimeout(delay));
	}

	function cancel() {
		if (finished || cancelled || destroyed || expired) return;
		cancelled = true;
		transaction.cancel();
		finish();
	}

	let statusText;
	onMount(() => {
		if (!transaction) {
			// Task message
			finish();
		} else {
			transaction?.listen(event => {
				if ('progress' in event) {
					progress = event.progress;
				} else if (event.finished) {
					finish();
				} else if (event.error) {
					error = [event.error, event.data];
					finish();
				} else if (event.cancelled) {
					cancelled = true;
					expire();
				}

				if (!finished && !cancelled && statusText) {
					statusText.textContent = statusTextFn(transaction);
				}
			});
		}
	});

	// FIXME errors that are long overflow and cause bad things!
	// TODO internationalize error strings
</script>

<div bind:this={taskElem} class="task" class:error={error || cancelled} class:pending={!finished} style="transform: translateY({y}px)" class:expired={expired}>
	<div>
		{#if !error && !cancelled}
			<div id="progress" style="width: {finished ? 100 : progress}%"></div>
		{/if}
		<div id="content">
			<div id="status">
				<Loading inline={true}/>
				<Check id="finished" stroke-width="3"/>
				<CircleAlert id="error" stroke-width="3"/>
			</div>
			{#if error}
				{translateError(...error)}
			{:else if cancelled}
				{$_('cancelled')}
			{:else if finished}
				{#if transaction}
					{$_('done')}
				{:else}
					{statusTextFn}
				{/if}
			{:else if transaction}
				<span bind:this={statusText}>{statusTextFn({ progress: 0 })}</span>
			{/if}
		</div>
		{#if transaction && !finished && !cancelled && !expired}
			<div id="cancel" use:tippyFollow={$_('cancel')} on:click={cancel}><Cross id="cancel" stroke-width="3"/></div>
		{/if}
	</div>
</div>

<style>
	.task {
		z-index: 1;
		height: 49px;

		grid-row: 1;
		grid-column: 1;

		transition: transform .5s cubic-bezier(0.34, 1.56, 0.64, 1), opacity .5s;
		opacity: 1;
	}
	.task:not(.pending) {
		z-index: -1;
	}
	.task > div {
		border-radius: .4rem;
		box-shadow: 0 0 6px 1px rgba(0, 0, 0, .35);
		overflow: hidden;
		background-color: #006cc7;

		position: relative;
		display: flex;
		align-items: center;

		transition: transform .5s, opacity .5s;
		transition-timing-function: cubic-bezier(0.16, 1, 0.3, 1);

		animation: init .5s cubic-bezier(0.16, 1, 0.3, 1);
		transform: scale(0, 0);
		opacity: 0;

		z-index: 1;
	}
	.task:not(.expired) > div {
		transform: scale(1, 1);
		opacity: 1;
	}
	.task.expired > div {
		transform: scale(0, 0);
		opacity: 0;
	}
	.task.error #progress {
		display: none;
	}
	.task.error > div {
		background-color: #822828;
	}
	.task #progress {
		position: absolute;
		height: 100%;
		/*transition: width .25s cubic-bezier(0.16, 1, 0.3, 1);*/
		background-color: #30A560;
		z-index: -1;
	}
	.task :global(img), .task :global(.icon) {
		width: 1rem;
	}
	.task #content {
		padding: 1rem;
		flex: 1;
		display: flex;
		justify-content: center;
		align-items: center;
		text-shadow: 0px 1px 0px rgba(0, 0, 0, 0.6);
	}
	.task #cancel {
		padding: 1rem;
		cursor: pointer;
		display: flex;
		position: absolute;
		right: 0;
	}
	.task.pending #cancel {
		pointer-events: all;
	}
	.task #cancel :global(.icon) {
		width: 1rem;
	}
	.task #status {
		display: grid;
		grid-template-columns: 1fr;
		grid-template-rows: 1fr;
		margin-right: .5rem;
	}

	.task #status :global(.loading) {
		grid-row: 1;
		grid-column: 1;
		transition: transform .5s, opacity .5s;
		opacity: 1;
		transform: scale(1, 1);
	}
	.task #status :global(#finished), .task #status :global(#error) {
		grid-row: 1;
		grid-column: 1;
		transition: transform .5s, opacity .5s;
		opacity: 0;
		transform: scale(0, 0) rotate(-90deg);
	}
	.task:not(.pending) #status :global(.loading) {
		opacity: 0;
		transform: scale(0, 0);
	}
	.task:not(.pending):not(.error) #status :global(#finished) {
		opacity: 1;
		transform: scale(1, 1) rotate(0);
	}
	.task.error #status :global(#error) {
		opacity: 1;
		transform: scale(1, 1) rotate(0);
	}

	@keyframes init {
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
