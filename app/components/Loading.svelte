<script>
	export let size = '1rem';
	export let inline = false;
	export let text = null;
</script>

<svg class="loading" class:inline={inline} style="width: {size}; height: {size}" width={size} height={size} xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" fill="none" viewBox="0 0 100 100" preserveAspectRatio="xMidYMid">
	<g transform="translate(20 20)">
		<rect x="-15" y="-15" width="30" height="30" fill="#ffffff">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.4s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(50 20)">
		<rect x="-15" y="-15" width="30" height="30" fill="#ffffff">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.3s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(80 20)">
		<rect x="-15" y="-15" width="30" height="30" fill="#ffffff">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.2s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(20 50)">
		<rect x="-15" y="-15" width="30" height="30" fill="#ffffff">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.3s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(50 50)">
		<rect x="-15" y="-15" width="30" height="30" fill="#ffffff">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.2s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(80 50)">
		<rect x="-15" y="-15" width="30" height="30" fill="#ffffff">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.1s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(20 80)">
		<rect x="-15" y="-15" width="30" height="30" fill="#ffffff">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.2s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(50 80)">
		<rect x="-15" y="-15" width="30" height="30" fill="#ffffff">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.1s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(80 80)">
		<rect x="-15" y="-15" width="30" height="30" fill="#ffffff">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="0s"></animateTransform>
		</rect>
	</g>
</svg>
{#if text}
	<span class="label">&nbsp;{text}</span>
{/if}

<style>
	.loading:not(.inline) {
		display: block;
		margin: auto;
	}
	.loading, .loading + span.label {
		vertical-align: middle;
	}
</style>
