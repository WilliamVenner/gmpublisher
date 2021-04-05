<script>
	import { DateTime } from "luxon";
	import { onDestroy } from "svelte";
	import { _ } from "svelte-i18n";
	import { writable } from "svelte/store";
	import { tippyFollow } from "../tippy.js";

	export let unix;
	export let absolute = false;
	export let elapsed = false;

	const date = new DateTime.fromSeconds(unix);

	let interval;
	onDestroy(() => window.clearInterval(interval));

	let timestampStr = absolute ? date.toLocaleString(DateTime.DATETIME_FULL) : '';
	if (!absolute) {
		function updateTimestamp() {
			if (DateTime.now() - date < 1000) {
				if (elapsed) {
					timestampStr = elapsed + 'ms';
				} else {
					timestampStr = $_('time_just_now');
				}
			} else {
				timestampStr = date.toRelative();
			}
		}
		updateTimestamp();

		interval = window.setInterval(updateTimestamp, unix < 60 ? 5000 : 59000);
	}
</script>

<span class="timestamp" use:tippyFollow={!absolute && date.toLocaleString(DateTime.DATETIME_FULL)}>{timestampStr}</span>

<style>
	.timestamp {
		text-decoration: underline;
		text-underline-offset: 5px;
		text-decoration-style: dotted;
	}
</style>