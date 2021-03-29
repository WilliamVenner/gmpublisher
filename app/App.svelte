<script>
	import { invoke } from "@tauri-apps/api/tauri";

	window.setTimeout(function() {
		console.log('invoking');
		console.log(invoke("new_cmd_test", {
			test: true
		}).then(response => {
			console.log(response);
		}));
	}, 2000);

	let name = 'world';
</script>

<h1>Hello {name}!</h1>