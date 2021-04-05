<script>
	export let dead = false;
</script>

<div class="workshop-preview-img" class:dead={dead}>
	<!-- TODO -->
</div>

<style>
	.workshop-preview-img {
		width: 100%;
		flex: 0;
		margin-top: .8rem;
		box-shadow: 0 0 2px 1px rgba(0, 0, 0, .5);
	}
	.workshop-preview-img.dead {
		position: relative;
		background-color: #0c0c0c;
	}
	.workshop-preview-img.dead::after {
		content: "";
		display: block;
		padding-bottom: 100%;
	}
	.workshop-preview-img.dead :global(svg) {
		position: absolute;
		margin: auto;
		left: 0;
		top: 0;
		right: 0;
		bottom: 0;
		color: #212121;
	}
</style>