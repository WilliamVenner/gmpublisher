<script>
	import { DateTime } from "luxon";
	import { onDestroy } from "svelte";
	import { writable } from "svelte/store";
	import { tippyFollow } from "../tippy.js";

	export let unix;
	export let absolute = false;

	const date = new DateTime.fromSeconds(unix);

	const timestampStr = absolute ? writable(date.toLocaleString(DateTime.DATETIME_FULL)) : writable(""); if (!absolute) {
		function updateTimestamp() { $timestampStr = date.toRelative() }
		updateTimestamp();

		const interval = window.setInterval(updateTimestamp, 59000);
		onDestroy(() => clearInterval(interval));
	}
</script>

<span class="timestamp" use:tippyFollow={!absolute && date.toLocaleString(DateTime.DATETIME_FULL)}>{$timestampStr}</span>

<style>
	.timestamp {
		text-decoration: underline;
		text-underline-offset: 5px;
		text-decoration-style: dotted;
	}
</style>