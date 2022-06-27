<script context="module">
	export const JOB_TYPE_DOWNLOAD = 0;
	export const JOB_TYPE_EXTRACT = 1;

	export const downloaderJobs = writable(0);
</script>

<script>
	import { CloudDownload, Cross, Folder, FolderAdd, LinkChain } from 'akar-icons-svelte';
	import { _ } from 'svelte-i18n';
	import { invoke } from '@tauri-apps/api/tauri';
	import { listen } from '@tauri-apps/api/event';
	import DestinationSelect from '../components/DestinationSelect.svelte';
	import { Transaction } from '../transactions';
	import * as dialog from '@tauri-apps/api/dialog';
	import { tippyFollow } from '../tippy';
	import { playSound } from '../sounds';
	import DownloaderJob from '../components/DownloaderJob.svelte';
	import { Steam } from '../steam';
	import { writable } from 'svelte/store';

	let extractingJobs = [];
	let downloadingJobs = [];

	let extractingWorkers = 0;
	let downloadingWorkers = 0;

	function pushTransaction(jobType, args) {
		const timestamp = new Date().getTime();
		var incrWorkers = true;

		switch(jobType) {
			case JOB_TYPE_DOWNLOAD:
				downloadingWorkers++;

				var [transaction] = args;
				var job = {
					transaction,
					timestamp,
					type: JOB_TYPE_DOWNLOAD,
				};
				transaction.listen(event => {
					if (event.finished || event.cancelled) {
						if (incrWorkers) {
							downloadingWorkers--;
							incrWorkers = false;
						}
						const index = downloadingJobs.findIndex(elem => elem.transaction === transaction);
						if (index != -1) {
							downloadingJobs.splice(index, 1);
						}
					} else if (event.stream) {
						const [tag, data] = event.data;
						if (tag === 0) {
							job.ws_id = data;
							downloadingJobs.push(job);
						} else if (tag === 1) {
							job.size = data;
						}
					} else if (event.error) {
						if (incrWorkers) {
							downloadingWorkers--;
							incrWorkers = false;
						}
					}
					downloadingJobs = downloadingJobs;
				});
				break;

			case JOB_TYPE_EXTRACT:
				extractingWorkers++;

				var [transaction, srcPath, fileName, ws_id] = args;

				var job = {
					transaction,
					timestamp,
					ws_id,
					srcPath,
					fileName,
					type: JOB_TYPE_EXTRACT,
				};
				extractingJobs.push(job);
				extractingJobs = extractingJobs;

				transaction.listen(event => {
					if (event.finished) {
						if (incrWorkers) {
							extractingWorkers--;
							incrWorkers = false;
						}
						job.path = event.data;
					} else if (event.stream) {

						const [gmaName, size] = event.data;

						if (gmaName) {
							job.fileName = gmaName;
						} else if (srcPath) {
							Steam.getAddon(srcPath).then(gma => {
								if (gma?.installed?.title) {
									if (gma.installed.ws_id && !job.ws_id) job.ws_id = gma.installed.ws_id;
									job.fileName = gma.installed.title;
									extractingJobs = extractingJobs;
								}
							});
						}

						job.size = size;

					} else if (event.error) {
						if (incrWorkers) {
							extractingWorkers--;
							incrWorkers = false;
						}
					} else if (event.cancelled) {
						if (incrWorkers) {
							extractingWorkers--;
							incrWorkers = false;
						}
						const index = extractingJobs.findIndex(elem => elem.transaction === transaction);
						if (index != -1) {
							extractingJobs.splice(index, 1);
						}
					}
					extractingJobs = extractingJobs;
				});
				break;
		}
	}

	const RE_DELIMETERS = /(?:, *|\n+|\t+| +)/g;
	const RE_WORKSHOP_ID = /^(\d+)|(?:https?:\/\/(?:www\.)?steamcommunity\.com\/(?:sharedfiles\/filedetails|workshop(?:\/filedetails)?)(?:\/?.*(?:\?|&)id=(\d+)(?=$|&)|\/(\d+)$))/gmi;
	function parseInput(e) {
		let input = this.value;
		if (e.clipboardData) {
			input = e.clipboardData.getData('text') ?? '';
		}
		input = input.trim().replace(RE_DELIMETERS, '\n');
		if (input.length === 0) {
			this.classList.remove('error');
			return;
		}

		var ids = {};
		var result;
		while ((result = RE_WORKSHOP_ID.exec(input)) !== null) {
			const id = result[1] ?? result[2] ?? result[3];
			if (!id || id in ids || !parseInt(id)) continue;
			ids[id] = true;
		}

		var ids = Object.keys(ids);
		if (e.clipboardData) {
			e.preventDefault();
			e.stopPropagation();
		}

		this.classList.remove('error');
		this.value = '';

		invoke('workshop_download', { ids: ids.map(id => parseInt(id)) });

		playSound('success');
	}
	function checkEmptyInput() {
		if (this.value.length === 0)
			this.classList.remove('error');
	}

	function showFocusTip() {
		this.setAttribute('placeholder', $_('download-input-tip'));
	}
	function hideFocusTip() {
		this.setAttribute('placeholder', $_('download-input'));
		this.classList.remove('error');
	}

	function setDestination(extractDestination) {
		destinationModal = false;
		AppSettings.extract_destination = extractDestination;

		invoke('update_settings', { settings: AppSettings });
	}
	let destinationModal = false;
	function openDestination() {
		destinationModal = true;
	}
	function cancelDestination() {
		destinationModal = false;
	}

	listen('ExtractionStarted', ({ payload: [transaction_id, srcPath, fileName, ws_id] }) => {
		pushTransaction(JOB_TYPE_EXTRACT, [new Transaction(transaction_id), srcPath, fileName, ws_id]);
	});

	listen('DownloadStarted', ({ payload: transaction_id }) => {
		pushTransaction(JOB_TYPE_DOWNLOAD, [new Transaction(transaction_id)]);
	});

	function browseGMA() {
		dialog.open({
			multiple: true,
			filters: [{
				name: $_('gma_type_name'),
				extensions: ['gma']
			}]
		}).then(files => {
			if (files && files.length > 0) {
				invoke('downloader_extract_gmas', { paths: files });
			}
		});
	}

	function removeAll(jobs) {
		while (jobs.length > 0) {
			const job = jobs[0];
			if (job.transaction.finished) {
				job.transaction.emit({ cancelled: true });
			} else {
				job.transaction.cancel();
			}
		}
	}

	function openAll() {
		for (let i = 0; i < extractingJobs.length; i++) {
			const job = extractingJobs[i];
			if (job.path) {
				invoke('open', { path: job.path });
			}
		}
	}

	$: {
		downloaderJobs.set(extractingWorkers + downloadingWorkers);
	}
</script>

<main class="hide-scroll">
	<div id="top-controls">
		<div id="download" class="icon-button" on:click={browseGMA} use:tippyFollow={$_('bulk_extract_gmas')}>
			<Folder size="1.2rem"/>
		</div>
		<div id="download-input-container">
			<input type="text" id="download-input" placeholder={$_('download-input')} on:paste={parseInput} on:change={parseInput} on:input={checkEmptyInput} on:focus={showFocusTip} on:blur={hideFocusTip}/>
			<LinkChain size="1rem"/>
		</div>
	</div>

	<div id="layout">
		<div id="downloading">
			<h2>
				<CloudDownload size="2rem"/>
				{$_('downloading')}
				{#if downloadingWorkers > 0}
					<img src="/img/dog.gif" class="working"/>
				{/if}
			</h2>
			<div class="table hide-scroll">
				<table class:idle={downloadingJobs.length === 0}>
					<thead>
						<tr>
							<th class="controls"></th>
							<th class="details">{$_('addon')}</th>
							<th class="speed">{$_('speed')}</th>
							<th class="total">{$_('total_filesize')}</th>
							<th class="progress">{$_('progress')}</th>
						</tr>
					</thead>
					<tbody>
						{#if downloadingJobs.length === 0}
							<tr class="idle">
								<td colspan="5">
									<div>
										<div><img src="/img/dog_sleep.gif"/></div>
										<div>{$_('waiting')}</div>
										<div class="tip">{$_('downloading_tip')}</div>
										<a href="https://steamcommunity.com/app/4000/workshop/" target="_blank"><div class="btn">{$_('open_workshop')}</div></a>
									</div>
								</td>
							</tr>
						{:else}
							{#each downloadingJobs as job}
								{#if job.transaction.progress > 0 && job.transaction.progress < 100}
									<DownloaderJob {job}/>
								{/if}
							{/each}
							{#each downloadingJobs as job}
								{#if job.transaction.progress <= 0 && !('error' in job.transaction)}
									<DownloaderJob {job}/>
								{/if}
							{/each}
							{#each downloadingJobs as job}
								{#if 'error' in job.transaction}
									<DownloaderJob {job}/>
								{/if}
							{/each}
						{/if}
					</tbody>
				</table>
			</div>
			<div class="buttons">
				<div class="btn" on:click={() => removeAll(downloadingJobs)}>{$_('remove_all')}</div>
			</div>
		</div>

		<div id="extracting">
			<h2>
				<FolderAdd size="2rem"/>
				{$_('extracting')}
				{#if extractingWorkers > 0}
					<img src="/img/dog.gif" class="working"/>
				{/if}
			</h2>
			<div class="table hide-scroll">
				<table class:idle={extractingJobs.length === 0}>
					<thead>
						<tr>
							<th class="controls"><Cross size="1rem"/><LinkChain size="1rem"/></th>
							<th class="details">{$_('addon')}</th>
							<th class="speed">{$_('speed')}</th>
							<th class="total">{$_('total_filesize')}</th>
							<th class="progress">{$_('progress')}</th>
						</tr>
					</thead>
					<tbody>
						{#if extractingJobs.length === 0}
							<tr class="idle" rowspan="2">
								<td colspan="5">
									<div>
										<div><img src="/img/dog_sleep.gif"/></div>
										<div>{$_('waiting')}</div>
										<!--<div class="tip">{$_('extraction_tip')}</div>-->
										<div class="btn" on:click={openDestination}>{$_('set_destination')}</div>
									</div>
								</td>
							</tr>
						{:else}
							{#each extractingJobs as job}
								{#if job.transaction.progress > 0 && job.transaction.progress < 100}
									<DownloaderJob {job}/>
								{/if}
							{/each}
							{#each extractingJobs as job}
								{#if job.transaction.progress <= 0 && !('error' in job.transaction)}
									<DownloaderJob {job}/>
								{/if}
							{/each}
							{#each extractingJobs as job}
								{#if job.transaction.progress >= 100 || 'error' in job.transaction}
									<DownloaderJob {job}/>
								{/if}
							{/each}
						{/if}
					</tbody>
				</table>
			</div>
			<div class="buttons">
				<div class="btn" on:click={() => removeAll(extractingJobs)}>{$_('remove_all')}</div>
				<div class="btn" on:click={openAll}>{$_('open_all')}</div>
			</div>
		</div>
	</div>

	<DestinationSelect text={$_('set_destination')} active={destinationModal} callback={setDestination} cancel={cancelDestination} forceCreateFolder={true}/>
</main>

<style>
	main {
		display: flex;
		flex-direction: column;
		height: 100%;
		padding: 1.5rem;
	}

	h2 {
		display: flex;
		align-items: center;
		justify-content: center;
	}
	h2 > :global(.icon) {
		margin-right: .5rem;
	}

	#layout {
		flex: 1;
		display: grid;
		grid-gap: 1.5rem;
		height: 100%;
		min-height: 0;
	}
	@media (orientation: portrait) {
		#layout {
			grid-template-columns: 1fr;
			grid-template-rows: calc(50% - 0.75rem) calc(50% - 0.75rem);
		}
		#layout > #extracting {
			grid-row: 2;
			grid-column: 1;
		}
		#layout > #downloading {
			grid-row: 1;
			grid-column: 1;
		}
	}
	@media (orientation: landscape) {
		#layout {
			grid-template-columns: calc(50% - 0.75rem) calc(50% - 0.75rem);
			grid-template-rows: 1fr;
		}
		#layout > #extracting {
			grid-row: 1;
			grid-column: 2;
		}
		#layout > #downloading {
			grid-row: 1;
			grid-column: 1;
		}
		#layout h2 {
			text-align: center;
		}
	}
	#layout > div {
		display: flex;
		flex-direction: column;
		height: 100%;
	}
	#layout .table {
		position: relative;
		flex: 1;
		background-color: #292929;
		box-shadow: inset 0 0 6px 2px rgb(0 0 0 / 20%);
		border: 1px solid #101010;
		border-radius: .4rem;
		height: 100%;
		flex-basis: 0;
	}
	#layout .table tr {
		font-size: .9em;
		text-align: left;
		transition: background-color .1s;
		word-break: break-all;
		box-shadow: inset 0 0 3px #00000087;
	}
	#layout .table tbody tr:not(.idle):nth-child(2n-1) {
		background-color: rgba(0, 0, 0, .24);
	}
	#layout .table tbody tr:not(.idle):nth-child(2n) {
		background-color: rgb(0, 0, 0, .12);
	}
	#layout .table table {
		width: calc(100% + 0.5px);
		border-collapse: collapse;
	}
	#layout .table table thead tr {
		background-color: #212121;
	}
	#layout .table th {
		text-align: center;
	}
	#layout .table td, #layout .table th {
		padding: .5rem;
	}
	#layout .table td:last-child, #layout .table th:last-child {
		padding-right: 1rem;
	}
	#layout .table table th.controls {
		font-size: .9em;
		opacity: 0;
	}
	#layout .table table .speed, #layout .table table .total {
		text-align: right;
		width: 10%;
		min-width: 7rem;
	}
	#layout .table table .controls {
		width: 1px;
		white-space: nowrap;
		text-align: center;
	}
	#layout .table table .controls {
		padding: .5rem;
		padding-right: 0rem;
	}
	#layout .table table .controls > :global(*:not(:last-child)) {
		margin-right: .2rem;
	}
	#layout .table table th.progress {
		text-align: center;
		position: relative;
		z-index: 1;
		width: 30%;
	}
	#layout h2 {
		margin-top: 0;
	}
	#layout .idle {
		box-shadow: none !important;
	}
	#layout .idle > td {
		padding: 1.5rem;
		max-width: 0;
	}
	#layout .idle > td > div {
		margin: auto;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
		width: max-content;
		height: max-content;
		text-align: center;
		line-height: 1.6;
		text-shadow: 0px 1px 0px rgb(0, 0, 0, .6);
		max-width: min(calc(100% - 2rem), 16rem);
	}
	#layout .working {
		vertical-align: middle;
		margin-left: .5rem;
		width: 2rem;
	}

	#top-controls {
		display: grid;
		grid-template-columns: auto 1fr;
		grid-template-rows: 1fr;
		margin-bottom: 1.5rem;

		position: sticky;
		top: 0;
	}
	#download-input-container {
		position: relative;
	}
	#download-input-container :global(.icon) {
		position: absolute;
		top: calc(50% - .5rem);
		left: 1rem;
		opacity: .3;
		vertical-align: initial !important;
	}
	#download-input {
		appearance: none;
		font: inherit;
		border-radius: 4px;
		border: none;
		background: #313131;
		box-shadow: 0px 0px 2px 0px rgba(0, 0, 0, .4);
		width: 100%;
		padding: 1rem;
		padding-left: 2.5rem;
		color: #fff;
	}
	#download-input:focus {
		outline: none;
		box-shadow: inset 0 0 0px 1.5px #127cff;
	}
	:global(#download-input.error) {
		box-shadow: inset 0 0 0px 1.5px var(--error-dark) !important;
		color: var(--error) !important;
	}
	:global(#download-input.error + .icon) {
		opacity: 1 !important;
		color: var(--error) !important;
	}
	#download-input:focus + :global(.icon) {
		opacity: 1;
	}
	#download {
		margin-right: .75rem;
	}

	#layout .btn {
		display: inline-block;
		padding: .5rem;
		background-color: #212121;
		padding-left: 1rem;
		padding-right: 1rem;
		box-shadow: inset 0 0 3px #0000002e;
		border-radius: .3rem;
		margin-top: 1rem;
		border: 1px solid #1a1a1a;
		cursor: pointer;
		text-align: center;
	}
	#layout .btn:active {
		background-color: #1b1b1b;
	}

	#layout table.idle {
		height: calc(100% + 0.5px);
	}
	#layout table td {
		word-break: break-word;
	}

	table .tip {
		margin-top: .5rem;
	}

	.buttons {
		display: flex;
	}
	.buttons .btn {
		flex: 1;
		flex-basis: 0;
		background-color: #292929 !important;
		font-size: .9em;
		padding: .7rem !important;
	}
	.buttons .btn:active {
		background-color: #212121 !important;
	}
	.buttons .btn:not(:last-child) {
		margin-right: 1rem;
	}
</style>
