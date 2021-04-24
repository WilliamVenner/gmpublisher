// 1. CLI
// 2. File associations
// 3. Context menu option

mod sys;

pub(super) fn stdin() -> bool {
	use tauri::api::clap::{App, Arg};

	let app = App::new("gmpublisher");

	let matches = app
	.version(env!("CARGO_PKG_VERSION"))
	.author("William Venner <william@venner.io>")
	.about("Publish, extract and work with GMA files")
	.args(&[
		Arg::new("extract")
		.short('e')
		.long("extract")
		.value_name("FILE")
		.takes_value(true)
		.about("Extracts a .GMA file")
		.conflicts_with_all(&["update", "in", "changes", "icon"]),

		Arg::new("out")
		.short('o')
		.long("out")
		.value_name("PATH")
		.takes_value(true)
		.about("Sets the output path for extracting GMAs. Defaults to the temp directory.")
		.requires("extract")
		.default_missing_value_os(std::env::temp_dir().join("gmpublisher").as_os_str())
		.conflicts_with_all(&["update", "in", "changes", "icon"])
	])
	.args(&[
		Arg::new("update")
		.short('u')
		.long("update")
		.value_name("PublishedFileId")
		.takes_value(true)
		.about("Publishes an update.")
		.requires("in")
		.conflicts_with_all(&["out", "extract"]),

		Arg::new("in")
		.long("in")
		.value_name("PATH")
		.takes_value(true)
		.about("Sets the directory the GMA for updating will be built from.")
		.requires("update")
		.conflicts_with_all(&["out", "extract"]),

		Arg::new("changes")
		.long("changes")
		.value_name("CHANGES")
		.takes_value(true)
		.about("Sets the changelog for an update.")
		.requires("update")
		.conflicts_with_all(&["out", "extract"]),

		Arg::new("icon")
		.long("icon")
		.value_name("PATH")
		.takes_value(true)
		.about("Path to a (max 1 MB) JPG/PNG/GIF file for Workshop preview image updating.")
		.requires("update")
		.conflicts_with_all(&["out", "extract"])
	])
	.get_matches();

	println!("{:#?}", matches);

	false
}
