<script>
	import { tippy } from '../tippy';
	import { getLocaleFromNavigator, _ } from 'svelte-i18n';
	import { Gear } from 'akar-icons-svelte';
	import Modal from './Modal.svelte';
	import Sidebar from './Sidebar.svelte';
	import { writable } from 'svelte/store';
	import SidebarItem from './SidebarItem.svelte';
	import Setting from './Setting.svelte';
	import { playSound } from '../sounds';
	import { invoke } from '@tauri-apps/api/tauri';
	import { switchLanguage } from '../i18n';

	let active = false;
	function toggle() {
		active = !active;
	}

	let activeItem = writable('general');

	function saveSettings(e) {
		e.preventDefault();
		invoke('update_settings', { settings: AppSettings });
	}

	async function validateGmod(before, after) {
		if (after.trim().length > 0) {
			if (!(await invoke('validate_gmod', { path: after }))) {
				return before.trim().length > 0 ? before : null;
			} else {
				playSound('success');
			}
		}
		return after;
	}

	let form;
	function afterChange() {
		if (this.type === 'checkbox') {
			AppSettings[this.id] = this.checked;
		} else if (typeof this.value === 'string') {
			const trimmed = this.value.trim();
			AppSettings[this.id] = trimmed.length > 0 ? trimmed : null;
		} else {
			AppSettings[this.id] = this.value;
		}
		form.requestSubmit();
	}

	function afterChangeColor() {
		AppSettings[this.id] = parseInt(this.value.substr(1), 16);
		form.requestSubmit();
	}

	function changeCustomColor() {
		afterChangeColor.call(this);
		updateCustomColor(this.id.substr('color_'.length), AppSettings[this.id]);
	}

	const languages = [
		['default', 'Automatic'],
		['en', 'English'],
	];
	for (let lang in APP_LANGUAGES) {
		if (lang === 'en') continue;
		languages.push([lang, APP_LANGUAGES[lang]?.LANGUAGE_NAME ?? lang]);
	}
	function chooseLanguage() {
		if (this.value === 'default') {
			AppSettings.language = null;
			switchLanguage(getLocaleFromNavigator());
		} else {
			AppSettings.language = this.value;
			switchLanguage(this.value);
		}
		form.requestSubmit();
	}

	const extractOverwriteModes = [
		['Overwrite', ['settings.extract_overwrite_mode.overwrite']],
		['Recycle', ['settings.extract_overwrite_mode.recycle']],
		['Delete', ['settings.extract_overwrite_mode.delete']]
	];
</script>

<Modal id="settings" active={active} cancel={toggle}>
	<Sidebar id="settings-sidebar">

		<SidebarItem {activeItem} id="general">{$_('settings.general.general')}</SidebarItem>
		<SidebarItem {activeItem} id="paths">{$_('settings.paths.paths')}</SidebarItem>
		<SidebarItem {activeItem} id="accessibility">{$_('settings.accessibility.accessibility')}</SidebarItem>
		<!-- TODO <SidebarItem {activeItem} id="resets">{$_('settings.resets.resets')}</SidebarItem>-->

	</Sidebar>

	<form id="content" class="hide-scroll" on:submit={saveSettings} bind:this={form}>
		{#if $activeItem === 'general'}
			<div id="open-count">
				<div>
					<Setting id="language" type="select" value={AppSettings.language ?? 'default'} choices={languages} afterChange={chooseLanguage}>Language</Setting>
					<Setting {afterChange} id="extract_overwrite_mode" type="select" value={AppSettings.extract_overwrite_mode} choices={extractOverwriteModes} tooltip={$_('settings.extract_overwrite_mode.tooltip')}>{$_('settings.extract_overwrite_mode.extract_overwrite_mode')}</Setting>
					<Setting {afterChange} id="sounds" type="bool" value={AppSettings.sounds}>{$_('settings.general.sounds')}</Setting>
				</div>
				<div>{$_('open_count', { values: { count: AppData.open_count } })}</div>
			</div>
		{:else if $activeItem === 'paths'}
			<Setting {afterChange} id="gmod" type="directory" initial={AppData.gmod_dir ?? $_('ERR_UNKNOWN')} value={AppSettings.gmod} beforeChange={validateGmod}>{$_('settings.paths.gmod')}</Setting>
			<Setting {afterChange} id="downloads" type="directory" initial={AppData.downloads_dir} value={AppSettings.downloads}>{$_('settings.paths.downloads')}</Setting>
			<Setting {afterChange} id="user_data" type="directory" initial={AppData.user_data_dir} value={AppSettings.user_data}>{$_('settings.paths.user_data')}</Setting>
			<Setting {afterChange} id="temp" type="directory" initial={AppData.temp_dir} value={AppSettings.temp}>{$_('settings.paths.temp')}</Setting>
		{:else if $activeItem === 'accessibility'}
			<Setting afterChange={changeCustomColor} id="color_neutral" type="color" value={AppSettings.color_neutral}>{$_('settings.accessibility.color_neutral')}</Setting>
			<Setting afterChange={changeCustomColor} id="color_success" type="color" value={AppSettings.color_success}>{$_('settings.accessibility.color_success')}</Setting>
			<Setting afterChange={changeCustomColor} id="color_error" type="color" value={AppSettings.color_error}>{$_('settings.accessibility.color_error')}</Setting>
		{:else if $activeItem === 'resets'}
			<!-- TODO -->
		{/if}
	</form>
</Modal>

<span class="nav-icon" use:tippy={$_('settings.settings')} on:click={toggle}><Gear size="1.5rem" stroke-width="1.5" id="settings"/></span>

<style>
	:global(#settings-sidebar) {
		width: 200px;
		background-color: rgb(0, 0, 0, .1);
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
	#open-count {
		min-height: 100%;
		display: flex;
		flex-direction: column;
	}
	#open-count > div:first-child {
		flex: 1;
	}
	#open-count > div:last-child {
		margin-top: 1rem;
		text-align: center;
		font-size: .8em;
		padding-bottom: 1.5rem;
		margin-bottom: -1.5rem;
	}
</style>
