<script>
	import { Cross, LinkChain } from "akar-icons-svelte";
	import Dead from "./Dead.svelte";
	import { _ } from 'svelte-i18n';
	import { Steam } from '../steam';
	import filesize from 'filesize';
	import { tippyFollow } from '../tippy';
	import { JOB_TYPE_EXTRACT } from '../pages/Downloader.svelte';
	import { invoke } from '@tauri-apps/api/tauri';

	function calculateSpeed(timestamp, progress, total) {
		if (total > 0 && progress > 0) {
			const elapsed = ((new Date().getTime() - timestamp) / 1000);
			if (elapsed > 0) {
				return filesize((total * (progress / 100)) / elapsed);
			}
		}
		return filesize(0);
	}

	export let job;

	function cancelJob() {
		if (job.transaction.finished) {
			job.transaction.emit({ cancelled: true });
		} else {
			job.transaction.cancel();
		}
	}

	function deadCallback() {
		if (!job.transaction.error && !job.transaction.finished) {
			job.transaction.setError("ERR_ITEM_NOT_FOUND");
		}
	}
</script>

<tr>
	<td class="controls">
		<span on:click={cancelJob}><Cross size="1rem"/></span>
		<a target="_blank" href="https://steamcommunity.com/sharedfiles/filedetails/?id={job.ws_id}"><LinkChain size="1rem"/></a>
	</td>
	<td class="details">
		{#if job.ws_id}
			{#await Steam.getWorkshopAddon(job.ws_id, job.type !== JOB_TYPE_EXTRACT ? deadCallback : undefined)}
				{#if job.fileName}
					{job.fileName}
				{:else}
					{job.ws_id}
				{/if}
			{:then metadata}
				{#if !metadata || metadata.dead}
					{#if job.fileName}
						<Dead inline/>&nbsp;{job.fileName}
					{:else}
						<Dead inline/>&nbsp;{job.ws_id}
					{/if}
				{:else}
					{metadata.title}
				{/if}
			{:catch}
				{#if job.fileName}
					<Dead inline/>&nbsp;{job.fileName}
				{:else}
					<Dead inline/>&nbsp;{job.ws_id}
				{/if}
			{/await}
		{:else}
			{job.fileName ?? '¯\\_(ツ)_/¯'}
		{/if}
	</td>
	<td class="speed">
		{#if !job.transaction.finished && !job.transaction.error && job.transaction.progress > 0 && job.transaction.progress < 100 && !!job.size}
			{calculateSpeed(job.timestamp, job.transaction.progress, job.size) + '/s'}
		{/if}
	</td>
	<td class="total">
		{#if !!job.size}
			{filesize(job.size)}
		{/if}
	</td>
	{#if job.type === JOB_TYPE_EXTRACT && job.transaction.finished}
		<td class="progress finished" on:click={() => invoke('open', { path: job.path })}>
			<div class="progress" style="width: calc(100% - .6rem)"></div>
			<div class="pct">{$_('open').toLocaleUpperCase()}</div>
		</td>
	{:else if job.transaction.error}
		<td class="progress" use:tippyFollow={$_(job.transaction.error[0], { values: { error: job.transaction.error[1] ? job.transaction.error[1] : undefined }})}>
			<div class="progress error" style="width: calc(100% - .6rem)"></div>
			<div class="pct">{$_('error')}</div>
		</td>
	{:else}
		<td class="progress">
			<div class="progress" style="width: calc({job.transaction.progress}% - .6rem)"></div>
			<div class="pct">{job.transaction.progress === 0 ? $_('queued') : (job.transaction.progress + '%')}</div>
		</td>
	{/if}
</tr>

<style>
	tr {
		font-size: .9em;
		text-align: left;
		transition: background-color .1s;
		word-break: break-all;
		box-shadow: inset 0 0 3px #00000087;
	}
	tr:nth-child(2n-1) {
		background-color: rgba(0, 0, 0, .24);
	}
	tr:nth-child(2n) {
		background-color: rgb(0, 0, 0, .12);
	}
	td.progress {
		text-align: center;
		position: relative;
		z-index: 1;
		width: 30%;
	}
	div.progress {
		position: absolute;
		height: calc(100% - .6rem);
		margin: .3rem;
		top: 0;
		left: 0;
		z-index: -1;
		background-color: #007d00;
	}
	div.progress.error {
		background-color: #7d0000;
	}
	td.progress.finished {
		cursor: pointer;
	}
	td.progress.finished > div.progress {
		background-color: #006fa5;
	}
	td.progress::before {
		content: '';
		position: absolute;
		height: calc(100% - .6rem);
		margin: .3rem;
		top: 0;
		left: 0;
		z-index: -2;
		box-shadow: inset 0 0 3px #00000087;
	}
	.speed, .total {
		text-align: right;
		width: 10%;
		min-width: 7rem;
	}
	td {
		word-break: break-word;
		padding: .5rem;
	}
	td:last-child {
		padding-right: 1rem;
	}
	.controls {
		width: 1px;
		white-space: nowrap;
		text-align: center;
	}
	.controls {
		padding: .5rem;
		padding-right: 0rem;
	}
	.controls :global(.icon) {
		cursor: pointer;
	}
	.controls > :global(*:not(:last-child)) {
		margin-right: .2rem;
	}
</style>
