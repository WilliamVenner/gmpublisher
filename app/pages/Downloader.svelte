<script>
	import { Cross, Folder, LinkChain } from 'akar-icons-svelte';
	import { DateTime } from 'luxon';
	import { _ } from 'svelte-i18n';
	import { invoke } from '@tauri-apps/api/tauri';
	import { Addons } from '../addons';
	import DestinationSelect from '../components/DestinationSelect.svelte';
	import { Transaction } from '../transactions';
	import Dead from '../components/Dead.svelte';
	import Loading from '../components/Loading.svelte';
	import filesize from 'filesize';
	import { listen } from '@tauri-apps/api/event';
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

	let destination = {
		named_dir: AppSettings.create_folder_on_extract,
		path: null,
		tmp: true,
		addons: false,
		downloads: false,
	};

	let jobLog = [];
	let extractingJobs = [];
	let downloadingJobs = [];

	function pushLog(msg, msgData) {
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

		switch (jobType) {
			case JOB_TYPE_DOWNLOAD: {

				const { transaction, ws_id } = args;
				pushLog(LOG_DOWNLOAD_STARTED, { ws_id });

				const job = downloadingJobs[downloadingJobs.push(args)-1];
				job.timestamp = timestamp;

				downloadingJobs = downloadingJobs;

				transaction.listen(event => {
					if (event.finished) {

						pushLog(LOG_DOWNLOAD_FINISHED, {
							ws_id,
							time: elapsedTime(timestamp),
						});

						pushTransaction(JOB_TYPE_EXTRACT, {
							ws_id,
							transaction: new Transaction(event.data)
						});

					} else if (event.error) {
						pushLog(LOG_DOWNLOAD_FAILED, {
							ws_id,
							reason: event.error
						});
					} else {
						if (event.stream) {
							// Returns the total number of bytes
							job.total = Number(event.data);
						}
						downloadingJobs = downloadingJobs;
						return;
					}

					for (let i = 0; i < downloadingJobs.length; i++) {
						if (downloadingJobs[i] === job) {
							downloadingJobs.splice(i, 1);
							break;
						}
					}
				});

			} break;

			case JOB_TYPE_EXTRACT: {

				const { transaction, path, ws_id } = args;

				const fileName = path ? (RE_FILE_NAME.exec(path)?.[1] ?? path) : undefined;

				pushLog(LOG_EXTRACT_STARTED, { ws_id, fileName });

				const job = extractingJobs[extractingJobs.push(args)-1];
				job.timestamp = timestamp;

				extractingJobs = extractingJobs;

				transaction.listen(event => {
					if (event.finished) {
						pushLog(LOG_EXTRACT_FINISHED, {
							ws_id,
							fileName,
							path: event.data,
							time: elapsedTime(timestamp),
						});
					} else if (event.error) {
						pushLog(LOG_EXTRACT_FAILED, {
							ws_id,
							fileName,
							reason: event.error
						});
					} else {
						if (event.stream) {
							const [total, gmaName] = event.data;
							job.total = total;
							job.fileName = gmaName;
						}
						extractingJobs = extractingJobs;
						return;
					}

					for (let i = 0; i < extractingJobs.length; i++) {
						if (extractingJobs[i] === job) {
							extractingJobs.splice(i, 1);
							break;
						}
					}
					extractingJobs = extractingJobs;
				});

			} break;
		}
	}

	const RE_DELIMETERS = /(?:, *|\n+|\t+)/g;
	const RE_WORKSHOP_ID = /^(\d+)|(?:https?:\/\/(?:www\.)?steamcommunity\.com\/sharedfiles\/filedetails\/\?id=(\d+).*)$/gmi;
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

		let dedup = {};
		var result;
		while ((result = RE_WORKSHOP_ID.exec(input)) !== null) {
			const id = result[1] ?? result[2];
			if (id in dedup) continue;
			dedup[id] = true;
		}

		let ids = Object.keys(dedup);
		if (ids.length !== input.split('\n').length) {
			this.classList.add('error');
		} else {
			if (e.clipboardData) {
				e.preventDefault();
				e.stopPropagation();
			}

			this.classList.remove('error');
			this.value = '';

			invoke('download_workshop', {
				ids,
				...destination,
			}).then(([transactions, installed, failed]) => {

				for (let i = 0; i < transactions.length; i++) {
					const [transaction_id, ws_id] = transactions[i];
					pushTransaction(JOB_TYPE_DOWNLOAD, {
						ws_id,
						transaction: new Transaction(transaction_id)
					});
				}

				for (let i = 0; i < installed.length; i++) {
					const [transaction_id, ws_id, path] = installed[i];
					pushTransaction(JOB_TYPE_EXTRACT, {
						path,
						ws_id,
						transaction: new Transaction(transaction_id)
					});
				}

				for (let i = 0; i < failed.length; i++)
					pushLog(LOG_DOWNLOAD_FAILED, { ws_id: failed[i] });

			}, err => alert(err));
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

	function openExtractedGMA() {
		invoke('open_file', {
			path: this.dataset.path
		});
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

		const dest = extractPath[0];
		destination = {
			named_dir: AppSettings.create_folder_on_extract,
			path: dest === 'browse' ? extractPath[1] : null,
			tmp: dest === 'tmp',
			addons: dest === 'addons',
			downloads: dest === 'downloads',
		};
	}
	let destinationModal = false;
	function openDestination() {
		destinationModal = true;
	}
	function cancelDestination() {
		destinationModal = false;
	}

	listen('ExtractionStarted', ({ payload: [transaction_id, ws_id, path] }) => {
		pushTransaction(JOB_TYPE_EXTRACT, {
			path,
			ws_id,
			transaction: new Transaction(transaction_id)
		});
	});
</script>

<main class="hide-scroll">
	<div id="top-controls">
		<div id="download">
			<Folder size="1.2rem"/>
		</div>
		<div id="download-input-container">
			<input type="text" id="download-input" placeholder={$_('download-input')} on:paste={parseInput} on:change={parseInput} on:input={checkEmptyInput} on:focus={showFocusTip} on:blur={hideFocusTip}/>
			<LinkChain size="1rem"/>
		</div>
	</div>

	<div id="layout">
		<div id="extracting">
			<h2>{$_('extracting')}{#if extractingJobs.length > 0}<img src="/img/dog.gif" class="working"/>{/if}</h2>
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
										{#await Addons.getWorkshopMetadata(Number(job.ws_id))}
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
								<td class="speed">{calculateSpeed(job.timestamp, job.transaction.progress, job.total ?? 0) + '/s'}</td>
								<td class="total">{filesize(job.total ?? 0)}</td>
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

		<div id="downloading">
			<h2>{$_('downloading')}{#if downloadingJobs.length > 0}<img src="/img/dog.gif" class="working"/>{/if}</h2>
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
									{#await Addons.getWorkshopMetadata(Number(job.ws_id))}
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
								<td class="speed">{calculateSpeed(job.timestamp, job.transaction.progress, job.total ?? 0) + '/s'}</td>
								<td class="total">{filesize(job.total ?? 0)}</td>
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

	<div id="destination" class:active={destinationModal}>
		<DestinationSelect text={$_('set_destination')} active={destinationModal} callback={setDestination} cancel={cancelDestination} forceCreateFolder={true}/>
	</div>
</main>

<style>
	#destination {
		position: absolute;
		width: 100%;
		height: 100%;
		z-index: 100;
		top: 0;
		left: 0;
	}
	#destination:not(.active) {
		pointer-events: none;
	}

	main {
		display: flex;
		flex-direction: column;
		height: 100%;
		padding: 1.5rem;
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
			grid-row: 1;
			grid-column: 1;
		}
		#layout > #downloading {
			grid-row: 2;
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
			grid-column: 1;
		}
		#layout > #downloading {
			grid-row: 1;
			grid-column: 2;
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
		text-align: center;
		width: 10%;
		min-width: 4rem;
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
		max-width: calc(100% - 2rem);
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
	#top-controls #download :global(.icon) {
		opacity: .3;
	}
	#top-controls #download {
		cursor: pointer;
		margin-right: 1rem;
		background: #313131;
		box-shadow: 0px 0px 2px 0px rgb(0 0 0 / 40%);
		border-radius: 4px;
		padding: .9rem;
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

	#layout #log .table {
		font-size: .9em;
	}
	#layout #log .row {
		display: grid;
		grid-template-rows: min-content 1fr;
		grid-template-columns: min-content min-content 1fr;
		grid-gap: 1rem;
		padding: 1rem;
	}
	#layout #log .row > .icon-1 {
		grid-row: 1 / 3;
		grid-column: 1;
		justify-self: center;
		align-self: center;
	}
	#layout #log .row > .icon-2 {
		grid-row: 1 / 3;
		grid-column: 2;
		justify-self: center;
		align-self: center;
	}
	#layout #log .row > :global(.timestamp) {
		grid-row: 1;
		grid-column: 3;
	}
	#layout #log .row > .msg {
		grid-row: 2;
		grid-column: 3;
	}
	#layout #log .row.click {
		cursor: pointer;
	}

	.click-to-open {
		display: block;
		margin-top: .5rem;
		margin-bottom: 5px;
	}

	#layout #log .row.click, #layout #log .row.error {
		position: relative;
		z-index: 1;
	}
	#layout #log .row.click::after, #layout #log .row.error::after {
		content: '';
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		animation: flash 5s forwards;
		z-index: -1;
	}
	#layout #log .row.click::after {
		background-color: green;
	}
	#layout #log .row.error::after {
		background-color: red;
	}
	@keyframes flash {
		from {
			opacity: .5;
		}
		to {
			opacity: 0;
		}
	}
</style>
