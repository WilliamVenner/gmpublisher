<script>
	import { CloudDownload } from 'akar-icons-svelte';
	import { _ } from 'svelte-i18n';
	import tippy from 'tippy.js';

	function compareSemver(current, cmp) {
		const currentParts = current.split('.').map(n => parseInt(n));
		const cmpParts = cmp.split('.').map(n => parseInt(n));
		for (let i = 0; i < 3; i++) {
			if (currentParts[i] < cmpParts[i]) {
				return true;
			} else if (currentParts[i] !== cmpParts[i]) {
				break;
			}
		}
		return false;
	}

	const CARGO_PKG_VERSION = /((?:\.?\d+)+)$/;
	const updateAvailable = new Promise((resolve, reject) => {
		fetch('https://api.github.com/repos/WilliamVenner/gmpublisher/releases/latest')
			.then(response => response.json(), reject)
			.then(data => {

				if (!data || !data.tag_name) return reject();

				const cargoVersion = data.tag_name.match(CARGO_PKG_VERSION);
				if (!cargoVersion || !cargoVersion[1]) return reject();

				if (compareSemver(AppData.version, cargoVersion[1])) {
					resolve(data.tag_name);
				}

			}, reject);
	});

	function tooltip(node, version) {
		const instance = tippy(node, {
			content: $_('update_available', { values: { version } }),
			interactive: false,
			showOnCreate: true,
			trigger: 'manual',
		});
		instance.popper.classList.add('update-popper');

		setTimeout(() => {
			instance.hide();
			instance.setProps({
				trigger: 'mouseenter'
			});
		}, 10000);
	}
</script>

{#await updateAvailable} {:then newVersion}
	<a href="https://github.com/WilliamVenner/gmpublisher/releases/tag/{newVersion}" target="_blank" use:tooltip={newVersion} class="nav-icon">
		<CloudDownload size="1.5rem" stroke-width="1.5" id="update-icon"/>
	</a>
{/await}

<style>
	:global(.update-popper) {
		text-align: center !important;
	}
	:global(#update-icon) {
		animation: update-available 1.5s infinite alternate;
		opacity: 1 !important;
	}

	@keyframes update-available {
		0% {
			color: #9effc9;
		}
		100% {
			color: white;
		}
	}
</style>
