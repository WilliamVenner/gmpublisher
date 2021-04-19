<script>
	import { CloudDownload, Cross, Folder, FolderAdd, LinkChain } from 'akar-icons-svelte';
	import { DateTime } from 'luxon';
	import { _ } from 'svelte-i18n';
	import { invoke } from '@tauri-apps/api/tauri';
	import { listen } from '@tauri-apps/api/event';
	import { Addons } from '../addons';
	import DestinationSelect from '../components/DestinationSelect.svelte';
	import { Transaction } from '../transactions';
	import Dead from '../components/Dead.svelte';
	import Loading from '../components/Loading.svelte';
	import filesize from 'filesize';
	import { NOTIFICATION_ERROR, NOTIFICATION_SUCCESS, NOTIFICATION_ALERT, pushNotification } from '../notifications';
	import * as dialog from '@tauri-apps/api/dialog';
	import { tippyFollow } from '../tippy';
	import { playSound } from '../sounds';
	//import Destination from '../modals/Destination.svelte';

	const RE_FILE_NAME = /(?:\\|\/|^)([^\/\\]+?)$/m;

	const LOG_DOWNLOAD_STARTED = 0;
	const LOG_DOWNLOAD_FINISHED = 1;
	const LOG_DOWNLOAD_FAILED = 2;
	const LOG_EXTRACT_STARTED = 3;
	const LOG_EXTRACT_FINISHED = 4;
	const LOG_EXTRACT_FAILED = 5;

	const JOB_TYPE_DOWNLOAD = 0;
	const JOB_TYPE_EXTRACT = 1;

	let jobLog = [];
	let extractingJobs = [];
	let downloadingJobs = [];

	function pushLog(msg, msgData) {
		// TODO pushNotification

		const log = {
			timestamp: new Date().getTime() / 1000,
			type: msg,
			addon: msgData.fileName ?? msgData.ws_id,
			...msgData
		};

		if (!msgData.fileName && msgData.ws_id) {
			Addons.getWorkshopMetadata(Number(msgData.ws_id)).then(metadata => {
				log.addon = metadata.title;
				jobLog = jobLog;
			});
		}

		jobLog.unshift(log);
		jobLog = jobLog;
		return jobLog[0];
	}

	function elapsedTime(timestamp) {
		const elapsed = new Date().getTime() - timestamp;
		if (elapsed < 1000) {
			return Math.round((elapsed + Number.EPSILON) * 100) / 100 + 'ms';
		} else {
			return DateTime.fromMillis(timestamp).toRelative();
		}
	}

	function calculateSpeed(timestamp, progress, total) {
		if (total > 0 && progress > 0) {
			const elapsed = ((new Date().getTime() - timestamp) / 1000);
			if (elapsed > 0) {
				return filesize((total * (progress / 100)) / elapsed);
			}
		}
		return filesize(0);
	}

	function pushTransaction(jobType, args) {
		const timestamp = new Date().getTime();

		console.log('sneed');

		switch(jobType) {
			case JOB_TYPE_DOWNLOAD:
				var [transaction] = args;
				transaction.listen(event => {
					if (event.finished) {
						const index = downloadingJobs.find(elem => elem.transaction === transaction);
						if (index) {
							downloadingJobs.splice(index, 1);
							downloadingJobs = downloadingJobs;
						}
						const path = event.data;
						/*Addons.getWorkshopAddon()
						pushNotification({
							title: $_('download_complete'),
							body:
						});*/
					} else if (event.stream) {
						const [ws_id, size] = event.data;
						downloadingJobs.push({
							transaction,
							timestamp,
							size,
							ws_id,
						});
						downloadingJobs = downloadingJobs;
					}
				});
				break;

			case JOB_TYPE_EXTRACT:
				var [transaction, fileName, ws_id] = args;
				transaction.listen(event => {
					if (event.finished) {
						const index = extractingJobs.find(elem => elem.transaction === transaction);
						if (index) {
							extractingJobs.splice(index, 1);
							extractingJobs = extractingJobs;
						}
					} else if (event.stream) {
						const size = event.data;
						extractingJobs.push({
							transaction,
							timestamp,
							size,
							ws_id,
							fileName,
						});
						extractingJobs = extractingJobs;
					}
				});
				break;
		}
	}

	const RE_DELIMETERS = /(?:, *|\n+|\t+)/g;
	const RE_WORKSHOP_ID = /^(\d+)|(?:https?:\/\/(?:www\.)?steamcommunity\.com\/sharedfiles\/filedetails\/?.*(?:\?|&)id=(\d+)(?=$|&))/gmi;
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
			const id = result[1] ?? result[2];
			if (!id || id in ids || !parseInt(id)) continue;
			ids[id] = true;
		}

		var ids = Object.keys(ids);
		if (ids.length !== input.split('\n').length) {
			this.classList.add('error');
			playSound('error');
		} else {
			if (e.clipboardData) {
				e.preventDefault();
				e.stopPropagation();
			}

			this.classList.remove('error');
			this.value = '';

			invoke('workshop_download', { ids: ids.map(id => parseInt(id)) });
		}
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

	function cancelJob() {
		const transaction = Transaction.get(Number(this.dataset.transaction));
		transaction?.cancel();

		const jobs = this.dataset.extracting == true ? extractingJobs : downloadingJobs;
		for (let i = 0; i < jobs.length; i++) {
			if (jobs[i] === job) {
				jobs.splice(i, 1);
				jobs = jobs;
				return;
			}
		}

		// TODO make sure all async stuff in the backend is respecting aborted()
	}

	function setDestination(extractPath) {
		destinationModal = false;

		switch (extractPath[0]) {
			case 'browse':
				AppSettings.extract_destination = {'Directory': extractPath[1]};
				break;

			case 'tmp':
				AppSettings.extract_destination = 'Temp';
				break;

			case 'addons':
				AppSettings.extract_destination = 'Addons';
				break;

			case 'downloads':
				AppSettings.extract_destination = 'Downloads';
				break;
		}

		invoke('update_settings', { settings: AppSettings });
	}
	let destinationModal = false;
	function openDestination() {
		destinationModal = true;
	}
	function cancelDestination() {
		destinationModal = false;
	}

	listen('ExtractionStarted', ({ payload: [transaction_id, fileName, ws_id] }) => {
		pushTransaction(JOB_TYPE_EXTRACT, [new Transaction(transaction_id), fileName, ws_id]);
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
				{#if downloadingJobs.length > 0}
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
						{/if}
						{#each downloadingJobs as job}
							<tr>
								<td class="controls">
									<Cross size="1rem" on:click={cancelJob} data-transaction={job.transaction.id} data-downloading={true}/>
									<a target="_blank" href="https://steamcommunity.com/sharedfiles/filedetails/?id={job.ws_id}"><LinkChain size="1rem"/></a>
								</td>
								<td class="details">
									{#await Addons.getWorkshopAddon(job.ws_id)}
										<Loading inline/>&nbsp;{job.ws_id}
									{:then metadata}
										{#if !metadata || metadata.dead}
											<Dead inline/>&nbsp;{job.ws_id}
										{:else}
											{metadata.title}
										{/if}
									{:catch}
										<Dead inline/>&nbsp;{job.ws_id}
									{/await}
								</td>
								<td class="speed">{calculateSpeed(job.timestamp, job.transaction.progress, job.size ?? 0) + '/s'}</td>
								<td class="total">{filesize(job.size ?? 0)}</td>
								<td class="progress">
									<div class="progress" style="width: calc({job.transaction.progress}% - .6rem)"></div>
									<div class="pct">{job.transaction.progress}%</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		</div>

		<div id="extracting">
			<h2>
				<FolderAdd size="2rem"/>
				{$_('extracting')}
				{#if extractingJobs.length > 0}
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
										<div class="tip">{$_('extraction_tip')}</div>
										<div class="btn" on:click={openDestination}>{$_('set_destination')}</div>
									</div>
								</td>
							</tr>
						{/if}
						{#each extractingJobs as job}
							<tr>
								<td class="controls">
									<Cross size="1rem" on:click={cancelJob} data-transaction={job.transaction.id} data-extracting={true}/>
									{#if job.ws_id}
										<a target="_blank" href="https://steamcommunity.com/sharedfiles/filedetails/?id={job.ws_id}"><LinkChain size="1rem"/></a>
									{/if}
								</td>
								<td class="details">
									{#if job.ws_id}
										{#await Addons.getWorkshopAddon(job.ws_id)}
											<Loading inline text={job.ws_id}/>
										{:then metadata}
											{#if !metadata || metadata.dead}
												<Dead inline text={job.ws_id}/>
											{:else}
												{metadata.title}
											{/if}
										{:catch}
											<Dead inline text={job.ws_id}/>
										{/await}
									{:else}
										{job.fileName}
									{/if}
								</td>
								<td class="speed">{calculateSpeed(job.timestamp, job.transaction.progress, job.size ?? 0) + '/s'}</td>
								<td class="total">{filesize(job.size ?? 0)}</td>
								<td class="progress">
									<div class="progress" style="width: calc({job.transaction.progress}% - .6rem)"></div>
									<div class="pct">{job.transaction.progress}%</div>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
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
	#layout .table > .hide-scroll {
		min-height: 100%;
		max-height: 100%;
		height: 100%;
		z-index: -1;
	}
	#layout .table tr {
		font-size: .9em;
		text-align: left;
		transition: background-color .1s;
		word-break: break-all;
		box-shadow: inset 0 0 3px #00000087;
	}
	#layout .table tbody tr:not(.idle):nth-child(2n-1), #layout .table .row:nth-child(2n-1) {
		background-color: rgba(0, 0, 0, .24);
	}
	#layout .table tbody tr:not(.idle):nth-child(2n), #layout .table .row:nth-child(2n) {
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
	#layout .table table td.progress, #layout .table table th.progress {
		text-align: center;
		position: relative;
		z-index: 1;
		width: 30%;
	}
	#layout .table table div.progress {
		position: absolute;
		height: calc(100% - .6rem);
		margin: .3rem;
		top: 0;
		left: 0;
		z-index: -1;
		background-color: #007d00;
	}
	#layout .table table td.progress::before {
		content: '';
		position: absolute;
		height: calc(100% - .6rem);
		margin: .3rem;
		top: 0;
		left: 0;
		z-index: -2;
		box-shadow: inset 0 0 3px #00000087;
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
		box-shadow: inset 0 0 0px 1.5px #6d0000 !important;
		color: #a90000 !important;
	}
	:global(#download-input.error + .icon) {
		opacity: 1 !important;
		color: #a90000 !important;
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
</style>
