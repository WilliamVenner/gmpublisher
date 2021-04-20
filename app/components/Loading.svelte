<script>
	export let size = '1rem';
	export let inline = false;
	export let text = null;
</script>

<svg class="loading" class:inline={inline} style="width: {size}; height: {size}" width={size} height={size} fill="currentColor" viewBox="0 0 135 140" xmlns="http://www.w3.org/2000/svg">
	<rect y="10" width="15" height="120" rx="6">
		<animate attributeName="height"
			 begin="0.5s" dur="1s"
			 values="120;110;100;90;80;70;60;50;40;140;120" calcMode="linear"
			 repeatCount="indefinite" />
		<animate attributeName="y"
			 begin="0.5s" dur="1s"
			 values="10;15;20;25;30;35;40;45;50;0;10" calcMode="linear"
			 repeatCount="indefinite" />
	</rect>
	<rect x="30" y="10" width="15" height="120" rx="6">
		<animate attributeName="height"
			 begin="0.25s" dur="1s"
			 values="120;110;100;90;80;70;60;50;40;140;120" calcMode="linear"
			 repeatCount="indefinite" />
		<animate attributeName="y"
			 begin="0.25s" dur="1s"
			 values="10;15;20;25;30;35;40;45;50;0;10" calcMode="linear"
			 repeatCount="indefinite" />
	</rect>
	<rect x="60" width="15" height="140" rx="6">
		<animate attributeName="height"
			 begin="0s" dur="1s"
			 values="120;110;100;90;80;70;60;50;40;140;120" calcMode="linear"
			 repeatCount="indefinite" />
		<animate attributeName="y"
			 begin="0s" dur="1s"
			 values="10;15;20;25;30;35;40;45;50;0;10" calcMode="linear"
			 repeatCount="indefinite" />
	</rect>
	<rect x="90" y="10" width="15" height="120" rx="6">
		<animate attributeName="height"
			 begin="0.25s" dur="1s"
			 values="120;110;100;90;80;70;60;50;40;140;120" calcMode="linear"
			 repeatCount="indefinite" />
		<animate attributeName="y"
			 begin="0.25s" dur="1s"
			 values="10;15;20;25;30;35;40;45;50;0;10" calcMode="linear"
			 repeatCount="indefinite" />
	</rect>
	<rect x="120" y="10" width="15" height="120" rx="6">
		<animate attributeName="height"
			 begin="0.5s" dur="1s"
			 values="120;110;100;90;80;70;60;50;40;140;120" calcMode="linear"
			 repeatCount="indefinite" />
		<animate attributeName="y"
			 begin="0.5s" dur="1s"
			 values="10;15;20;25;30;35;40;45;50;0;10" calcMode="linear"
			 repeatCount="indefinite" />
	</rect>
</svg>

<!--

<svg class="loading" class:inline={inline} style="width: {size}; height: {size}" width={size} height={size} xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" fill="none" viewBox="0 0 100 100" preserveAspectRatio="xMidYMid">
	<g transform="translate(20 20)">
		<rect x="-15" y="-15" width="30" height="30" fill="currentColor">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.4s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(50 20)">
		<rect x="-15" y="-15" width="30" height="30" fill="currentColor">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.3s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(80 20)">
		<rect x="-15" y="-15" width="30" height="30" fill="currentColor">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.2s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(20 50)">
		<rect x="-15" y="-15" width="30" height="30" fill="currentColor">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.3s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(50 50)">
		<rect x="-15" y="-15" width="30" height="30" fill="currentColor">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.2s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(80 50)">
		<rect x="-15" y="-15" width="30" height="30" fill="currentColor">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.1s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(20 80)">
		<rect x="-15" y="-15" width="30" height="30" fill="currentColor">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.2s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(50 80)">
		<rect x="-15" y="-15" width="30" height="30" fill="currentColor">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="-0.1s"></animateTransform>
		</rect>
	</g>
	<g transform="translate(80 80)">
		<rect x="-15" y="-15" width="30" height="30" fill="currentColor">
			<animateTransform attributeName="transform" type="scale" repeatCount="indefinite" calcMode="spline" dur="1s" values="1;1;0.2;1;1" keyTimes="0;0.2;0.5;0.8;1" keySplines="0.5 0.5 0.5 0.5;0 0.1 0.9 1;0.1 0 1 0.9;0.5 0.5 0.5 0.5" begin="0s"></animateTransform>
		</rect>
	</g>
</svg>

-->

{#if text}
	<span class="label">&nbsp;{text}</span>
{/if}

<style>
	.loading:not(.inline) {
		display: block;
		margin: auto;
		top: 0;
		left: 0;
		right: 0;
		bottom: 0;
	}
	.loading, .loading + span.label {
		vertical-align: middle;
	}
</style>
