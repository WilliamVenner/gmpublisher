<script>
	import { Bell } from "akar-icons-svelte";
	import { notifications } from "../notifications";
	import { tippy } from "../tippy";
	import { _ } from "svelte-i18n";
</script>

{#if $notifications.length === 0}
	<main id="notifications" class="nav-icon" use:tippy={$_('no_notifications')}>
		<Bell size="1.5rem" stroke-width="1.5"/>
	</main>
{:else if $notifications.length === 1}
	<main id="notifications" class="nav-icon active" use:tippy={$_('notifications_one')}>
		<Bell size="1.5rem" stroke-width="1.5"/>
	</main>
{:else}
	<main id="notifications" class="nav-icon active" use:tippy={$_('notifications_count', { values: { count: $notifications.length } })}>
		<Bell size="1.5rem" stroke-width="1.5"/>
	</main>
{/if}

<style>
	#notifications {
		position: relative;
		cursor: pointer;
	}
	#notifications :global(.icon) {
		transition: opacity .25s, color .25s;
	}
	#notifications.active > :global(.icon) {
		opacity: 1 !important;
		color: #0096ff;
	}
</style>
