<script>
	import { tippy } from '../tippy';
	import { _ } from 'svelte-i18n';
	import { Gear } from 'akar-icons-svelte';
	import Modal from './Modal.svelte';
	import Sidebar from './Sidebar.svelte';
	import { writable } from 'svelte/store';
	import SidebarItem from './SidebarItem.svelte';
	import Setting from './Setting.svelte';
	import * as notification from '@tauri-apps/api/notification';
	import { enabled as desktopNotificationsEnabled } from '../notifications';
	import { playSound } from '../sounds';

	let active = false;
	function toggle() {
		active = !active;
	}

	let activeItem = writable('paths');

	function saveSettings(e) {
		e.preventDefault();
		console.log(e);
	}

	async function desktopNotificationsChange(val) {
		if (val === true) {
			const granted = await notification.isPermissionGranted();
			if (granted) {
				return true;
			} else {
				const request = await notification.requestPermission();
				if (request === 'granted' || request === true) {
					playSound('success');
					return true;
				}
			}
			playSound('error');
			return false;
		}
		return val;
	}
</script>

<Modal id="settings" active={active} cancel={toggle}>
	<Sidebar id="settings-sidebar">

		<SidebarItem {activeItem} id="paths">{$_('settings.paths.paths')}</SidebarItem>
		<SidebarItem {activeItem} id="notifications">{$_('settings.notifications.notifications')}</SidebarItem>
		<SidebarItem {activeItem} id="resets">{$_('settings.resets.resets')}</SidebarItem>

	</Sidebar>

	<form id="content" class="hide-scroll" on:submit={saveSettings}>
		{#if $activeItem === 'paths'}
			<Setting id="gmod" type="path" initial={AppData.gmod_dir} value={AppSettings.gmod}>{$_('settings.paths.gmod')}</Setting>
			<Setting id="downloads" type="path" initial={AppData.downloads_dir} value={AppSettings.downloads}>{$_('settings.paths.downloads')}</Setting>
			<Setting id="user_data" type="path" initial={AppData.user_data_dir} value={AppSettings.user_data}>{$_('settings.paths.user_data')}</Setting>
			<Setting id="temp" type="path" initial={AppData.temp_dir} value={AppSettings.temp}>{$_('settings.paths.temp')}</Setting>
		{:else if $activeItem === 'notifications'}
			<Setting id="notification_sounds" type="bool" value={AppSettings.notification_sounds}>{$_('settings.notifications.sounds')}</Setting>
			<Setting id="desktop_notifications" type="bool" value={$desktopNotificationsEnabled} beforeChange={desktopNotificationsChange}>{$_('settings.notifications.desktop')}</Setting>
		{:else if $activeItem === 'resets'}
			resets
		{/if}
	</form>
</Modal>

<span class="nav-icon" use:tippy={$_('settings.settings')} on:click={toggle}><Gear size="1.5rem" stroke-width="1.5" id="settings"/></span>

<style>
	:global(#settings-sidebar) {
		width: 200px;
		background-color: rgb(0, 0, 0, .1);
		box-shadow: 0 0 10px 0px rgba(0, 0, 0, .35);
	}
	:global(#settings > div) {
		display: flex;
		flex-direction: row;
		width: 42rem;
		height: 30rem;
	}
	#content {
		flex: 1;
		display: flex;
		flex-direction: column;
		padding: 1.5rem;
	}
</style>
