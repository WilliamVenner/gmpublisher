<script>
	import { Bell } from "akar-icons-svelte";
	import { notifications } from "../notifications";
	import { tippy } from "../tippy";
	import { _ } from "svelte-i18n";
</script>

<main id="notifications" class="nav-icon" use:tippy={$notifications.length === 0 ? $_('no_notifications') : null}>
	<Bell size="1.5rem" stroke-width="1.5"/>
</main>

<style>
	#notifications {
		cursor: pointer;
	}
</style>
