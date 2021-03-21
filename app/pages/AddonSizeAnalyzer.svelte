<script>
	import { afterUpdate, onDestroy, onMount } from 'svelte';
	import { _ } from 'svelte-i18n';
	import { promisified, invoke } from 'tauri/api/tauri';
	import Loading from '../components/Loading.svelte';
	import WorkshopAddon from '../components/WorkshopAddon.svelte';
	import Dead from '../../public/img/dead.svg';
	import { Transaction } from '../transactions';

	let container;
	let canvas;

	let addonSizes = new Promise(() => {});

	let transaction;
	let analyzedBounds = [];
	let treemap;

	const DeadSVG = new Image();
	DeadSVG.src = '/img/dead-canvas.svg';

	function analyze() {
		transaction?.cancel();

		analyzedBounds[0] = container.clientWidth;
		analyzedBounds[1] = container.clientHeight;

		promisified({

			cmd: 'analyzeAddonSizes',
			w: analyzedBounds[0],
			h: analyzedBounds[1]

		}).then(transactionId => {
			transaction = new Transaction(transactionId);

			addonSizes = new Promise((resolve, reject) => {
				transaction.listen(event => {
					console.log(event);
					if (event.finished) {
						treemap = event.data;
						resolve();
					}
				});
			});
		});
	}

	let resizedTimeout;
	function resized() {
		if (analyzedBounds[0] !== container.clientWidth || analyzedBounds[1] !== container.clientHeight) {
			clearTimeout(resizedTimeout);
			resizedTimeout = setTimeout(analyze, 1000);
		}
	}

	function updateCanvas() {
		if (!canvas || !treemap) return;

		var scale = window.devicePixelRatio;
		canvas.width = Math.floor(container.clientWidth * scale);
		canvas.height = Math.floor(container.clientHeight * scale);

		const ctx = canvas.getContext('2d');
		ctx.scale(scale, scale);
		ctx.clearRect(0, 0, canvas.width, canvas.height);

		for (let i = 0; i < treemap.length; i++) {
			const addon = treemap[i];
			
			let r; let g; let b;

			if (addon.preview_url) {
				// r = Math.floor(Math.random() * (255 - 0 + 1) + 0);
				// g = Math.floor(Math.random() * (255 - 0 + 1) + 0);
				// b = Math.floor(Math.random() * (255 - 0 + 1) + 0);
				// ctx.fillStyle = `rgb(${r},${g},${b})`;
				// ctx.fillRect(addon.x, addon.y, addon.w, addon.h);

				const image = new Image(addon.w, addon.h);
				image.onload = () => ctx.drawImage(image, addon.x, addon.y, addon.w, addon.h);
				image.src = addon.preview_url;
			} else {
				ctx.fillStyle = '#0c0c0c';
				ctx.fillRect(addon.x, addon.y, addon.w, addon.h);
				
				const size = 50;
				ctx.drawImage(DeadSVG, addon.x + (addon.w / 2) - (size / 2), addon.y + (addon.h / 2) - (size / 2), size, size); // TODO make size relative
			}
		}
	}

	onMount(analyze);
	afterUpdate(updateCanvas);
	onDestroy(() => {
		clearTimeout(resizedTimeout);
		transaction?.cancel();
		invoke({ cmd: 'freeAddonSizeAnalyzer' });
	});
</script>

<svelte:window on:resize={resized}/>

<main bind:this={container}>
	{#await addonSizes}
		<Loading/>
	{:then}
		<canvas bind:this={canvas}></canvas>
	{:catch}
		<Dead/>
	{/await}
</main>

<style>
	main {
		background-color: #1a1a1a;
		border-radius: .3rem;
		box-shadow: 0 0 0 #000 inset;

		width: 100%;
		height: calc(100% - 1.5rem);
		max-height: calc(100% - 1.5rem);
		min-height: calc(100% - 1.5rem);
	}
	main > canvas {
		width: 100%;
		height: 100%;
	}
</style>