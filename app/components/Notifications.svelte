<script>
	import { Bell } from "akar-icons-svelte";
	import { notifications } from "../notifications";
	import { tippy } from "../tippy";
	import { _ } from "svelte-i18n";
</script>

<audio id="sound-alert">
	<source src="/sound/alert.wav" type="audio/wav">
</audio>

<audio id="sound-success">
	<source src="/sound/success.wav" type="audio/wav">
</audio>

<audio id="sound-error">
	<source src="/sound/error.wav" type="audio/wav">
</audio>

<main id="notifications" class="nav-icon" use:tippy={$notifications.length === 0 ? $_('no_notifications') : null}>
	<Bell size="1.5rem" stroke-width="1.5"/>
</main>

<style>
	#notifications {
		cursor: pointer;
	}
</style>
