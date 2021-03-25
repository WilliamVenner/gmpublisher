<script>
	import { Cross, Download, LinkChain } from 'akar-icons-svelte';
	import { DateTime } from 'luxon';
	import { _ } from 'svelte-i18n';
	import { promisified } from 'tauri/api/tauri';
	import { Addons } from '../addons';
	import Timestamp from '../components/Timestamp.svelte';
	import { Transaction } from '../transactions';
	import Dead from '../components/Dead.svelte';
	import Loading from '../components/Loading.svelte';
	import filesize from 'filesize';

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
		jobLog.unshift({
			timestamp: new Date().getTime() / 1000,
			type: msg,
			...msgData
		});
		jobLog = jobLog;
		return jobLog[0];
	}

	function pushTransaction(jobType, args) {
		const timestamp = new Date().getTime() / 1000;

		switch (jobType) {
			case JOB_TYPE_DOWNLOAD: {

				const { transaction, ws_id } = args;
				pushLog(LOG_DOWNLOAD_STARTED, { ws_id, });

				const job = downloadingJobs[downloadingJobs.push(args)-1];
				downloadingJobs = downloadingJobs;

				transaction.listen(event => {
					if (event.data) {
						// Returns the total number of bytes
						job.total = Number(event.data);
						downloadingJobs = downloadingJobs;
						return;
					} else {
						if (event.finished) {
							pushLog(LOG_DOWNLOAD_FINISHED, {
								ws_id,
								elapsed: DateTime.fromSeconds((new Date().getTime() / 1000) - timestamp).toRelative(),
							});
						} else if (event.error) {
							pushLog(LOG_DOWNLOAD_ERROR, {
								ws_id,
								reason: event.error
							});
						} else {
							downloadingJobs = downloadingJobs;
							return;
						}
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

				const { transaction, path } = args;

				const fileName = RE_FILE_NAME.exec(path)?.[1] ?? path;

				pushLog(LOG_EXTRACT_STARTED, {
					fileName,
					path,
				});

				const job = extractingJobs[extractingJobs.push(args)-1];
				extractingJobs = extractingJobs;

				transaction.listen(event => {
					if (event.finished) {
						pushLog(LOG_DOWNLOAD_FINISHED, {
							fileName,
							path,
							elapsed: DateTime.fromSeconds((new Date().getTime() / 1000) - timestamp).toRelative(),
						});
					} else if (event.error) {
						pushLog(LOG_EXTRACT_FAILED, {
							fileName,
							path,
							reason: event.error
						});
					} else {
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
	const RE_WORKSHOP_ID = /^(\d+)|(?:https?:\/\/(?:www\.)?steamcommunity\.com\/sharedfiles\/filedetails\/\?id=(\d+)\/?)$/gmi;
	function parseInput(e) {
		const input = this.value.trim().replace(RE_DELIMETERS, '\n');

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
			this.classList.remove('error');
			this.value = '';

			promisified({
				cmd: 'downloadWorkshop',
				ids,
				...destination,
			}).then(([transactions, failed]) => {
				
				for (let i = 0; i < transactions.length; i++) {
					const [transaction_id, ws_id] = transactions[i];
					pushTransaction(JOB_TYPE_DOWNLOAD, {
						ws_id,
						transaction: new Transaction(transaction_id)
					});
				}
				
				for (let i = 0; i < failed.length; i++)
					pushLog(LOG_DOWNLOAD_FAILED, { ws_id: failed[i] });

			}, err => alert(err));
		}
	}

	function showFocusTip() {
		this.setAttribute('placeholder', $_('download-input-tip'));
	}
	function hideFocusTip() {
		this.setAttribute('placeholder', $_('download-input'));
		this.classList.remove('error');
	}

	function cancelJob() {
		// TODO
	}

	function setDestination() {
		// TODO
	}

	// const busy = extractingJobs.length > 0 || downloadingJobs.length > 0;
	// TODO prevent navigation if busy
</script>

<main>
	<div id="top-controls">
		<div id="download">
			<Download size="1.2rem"/>
		</div>
		<div id="download-input-container">
			<input type="text" id="download-input" placeholder={$_('download-input')} on:paste={parseInput} on:change={parseInput} on:focus={showFocusTip} on:blur={hideFocusTip}/>
			<LinkChain size="1rem"/>
		</div>
	</div>

	<div id="layout">
		<div id="extracting" class:idle={extractingJobs.length === 0}>
			<h2>{$_('extracting')}<img src="/img/dog.gif" class="working"/></h2>
			<div class="table hide-scroll">
				<div class="idle">
					<div><img src="/img/dog_sleep.gif"/></div>
					<div>{$_('waiting')}</div>
					<div class="tip">{$_('extraction_tip')}</div>
					<div class="btn" on:click={setDestination}>{$_('set_destination')}</div>
				</div>
				{#each extractingJobs as job}

				{/each}
			</div>
		</div>

		<div id="downloading" class:idle={downloadingJobs.length === 0}>
			<h2>{$_('downloading')}<img src="/img/dog.gif" class="working"/></h2>
			<div class="table hide-scroll">
				<div class="idle">
					<div><img src="/img/dog_sleep.gif"/></div>
					<div>{$_('waiting')}</div>
				</div>
				<table>
					<thead>
						<tr>
							<th></th>
							<th>{$_('addon')}</th>
							<th>{$_('speed')}</th>
							<th>{$_('total_filesize')}</th>
							<th>{$_('progress')}</th>
						</tr>
					</thead>
					<tbody>
						{#each downloadingJobs as job}
							<tr data-ws-id={job.ws_id}>
								<td class="controls">
									<Cross size="1rem" on:click={cancelJob}/>
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
								<td>{#if job.speed} {job.speed} {/if}</td>
								<td>{#if job.total} {filesize(job.total)} {/if}</td>
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

		<div id="log">
			<h2 style="text-align:center">{$_('log')}</h2>
			<div class="table hide-scroll">
				{#each jobLog as log}
					<div class="row">
						<div class="timestamp"><Timestamp unix={log.timestamp}/></div>
						<div class="msg">
							{#if log.type === LOG_DOWNLOAD_STARTED}
								{$_('LOG_DOWNLOAD_STARTED', { values: { /*XXX*/ } })}
							{:else if log.type === LOG_DOWNLOAD_FINISHED}
								{$_('LOG_DOWNLOAD_FINISHED', { values: { /*XXX*/ } })}
							{:else if log.type === LOG_DOWNLOAD_FAILED}
								{$_('LOG_DOWNLOAD_FAILED', { values: { /*XXX*/ } })}
							{:else if log.type === LOG_EXTRACT_STARTED}
								{$_('LOG_EXTRACT_STARTED', { values: { /*XXX*/ } })}
							{:else if log.type === LOG_EXTRACT_FINISHED}
								{$_('LOG_EXTRACT_FINISHED', { values: { /*XXX*/ } })}
							{:else if log.type === LOG_EXTRACT_FAILED}
								{$_('LOG_EXTRACT_FAILED', { values: { /*XXX*/ } })}
							{/if}
						</div>
					</div>
				{/each}
			</div>
		</div>
	</div>
</main>

<style>
	main {
		display: flex;
		flex-direction: column;
		height: calc(100% - 1.5rem);
	}

	#layout {
		flex: 1;
		display: grid;
		grid-template-columns: 1fr 20%;
		grid-template-rows: 1fr 1fr;
		grid-gap: 1.5rem;
	}
	#layout > div {
		display: flex;
		flex-direction: column;
	}
	#layout #extracting {
		grid-column: 1;
		grid-row: 1;
	}
	#layout #downloading {
		grid-column: 1;
		grid-row: 2;
	}
	#layout #log {
		grid-column: 2;
		grid-row: 1 / 3;
	}
	#layout .table {
		position: relative;
		flex: 1;
		overflow: auto;
		background-color: #292929;
		box-shadow: inset 0 0 6px 2px rgb(0 0 0 / 20%);
		border: 1px solid #101010;
		border-radius: .4rem;
	}
	#layout .table tr {
		padding: .6rem;
		font-size: .9em;
		text-align: left;
		transition: background-color .1s;
		word-break: break-all;
	}
	#layout .table tr {
		box-shadow: inset 0 0 3px #00000087;
	}
	#layout .table tbody tr:nth-child(2n-1) {
		background-color: rgba(0, 0, 0, .24);
	}
	#layout .table tbody tr:nth-child(2n) {
		background-color: rgb(0, 0, 0, .12);
	}
	#layout .table table {
		width: 100%;
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
		padding-right: 0;
	}
	#layout .table td:last-child, #layout .table th:last-child {
		padding-right: .5rem;
	}
	#layout .table table .controls {
		width: 1px;
		white-space: nowrap;
	}
	#layout .table table .details {
		width: max-content;
		width: 25%;
	}
	#layout .table table td.progress {
		text-align: center;
		position: relative;
		z-index: 1;
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
	#layout h2 {
		margin-top: 0;
	}
	#layout > div.idle .idle {
		position: absolute;
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
	}
	#layout > div.idle .idle img {
		display: inline-block;
	}
	#layout > div.idle .working, #layout > div:not(.idle) .idle {
		display: none;
	}
	#layout > div .working {
		vertical-align: middle;
		margin-left: .5rem;
		width: 2rem;
	}

	#top-controls {
		display: grid;
		grid-template-columns: auto 1fr;
		grid-template-rows: 1fr;
		margin-bottom: 1.5rem;
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
		background: rgba(255,255,255,.1);
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
		background: rgba(255, 255, 255, .1);
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
</style>