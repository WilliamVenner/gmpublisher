<script>
	import { taskHeight, tasksMax, tasks as tasksStore, tasksNum } from '../transactions.js';
	import Task from './Task.svelte';

	let tasksContainer;

	tasksStore.subscribe(tasks => {
		let update = false;

		let pos = 0;
		let i = 0;
		while (i < tasks.length) {
			const [transaction, statusTextFn, elem] = tasks[i];
			if (!elem) {
				update = true;
				tasks[i][2] = new Task({
					target: tasksContainer,
					props: {
						pos: pos++,
						transaction,
						statusTextFn
					}
				});
			} else {
				if (elem.destroyed) {
					update = true;
					elem.$destroy();
					tasks.splice(i, 1);
				} else if (!elem.expired) {
					elem.pos = pos++;
					elem.shift();
				}
			}
			i++;
		}

		$tasksNum = pos;
		if (update) tasksStore.set(tasks);
	});

	let height = 0;
	tasksMax.subscribe(tasksMax => {
		height = (taskHeight * tasksMax) + ((taskHeight / 2) * Math.max(tasksMax - 1, 0));
	});

</script>

<div bind:this={tasksContainer} id="tasks" style="height: {height}px"></div>

<style>
	#tasks {
		position: absolute;
		line-height: 0;
		width: 100%;
		bottom: 0;
		left: 0;
		pointer-events: none;
		z-index: 9999;
		text-align: center;
		display: inline-grid;
		justify-content: center;
		grid-template-columns: 45vw;
		overflow-y: hidden;
		padding-top: 24.5px;
		padding-bottom: 24.5px;
		box-sizing: content-box;
	}
</style>
