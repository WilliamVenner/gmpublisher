use std::path::PathBuf;

use crate::{
	gma::{ExtractDestination, ExtractGMAMut},
	GMAFile,
};

lazy_static! {
	pub static ref CLI_MODE: bool = std::env::args_os().len() > 1;
}

// 1. CLI
// 2. File associations
// 3. Context menu option

mod sys;

pub(super) fn stdin() -> bool {
	use clap::{App, Arg};

	if !*CLI_MODE {
		return false;
	}

	let temp_dir = std::env::temp_dir().join("gmpublisher");

	let app = App::new("gmpublisher");

	let matches = app
	.version(env!("CARGO_PKG_VERSION"))
	.author("William Venner <william@venner.io>")
	.about("Publish, extract and work with GMA files")
	.args(&[
		Arg::with_name("extract")
		.short("e")
		.long("extract")
		.value_name("FILE")
		.takes_value(true)
		.help("Extracts a .GMA file"),
		//.conflicts_with_all(&["update", "in", "changes", "icon"]),

		Arg::with_name("out")
		.short("o")
		.long("out")
		.value_name("PATH")
		.takes_value(true)
		.help("Sets the output path for extracting GMAs. Defaults to the temp directory.")
		.requires("extract")
		.default_value_os(temp_dir.as_os_str())
		//.conflicts_with_all(&["update", "in", "changes", "icon"])
	])
	/*.args(&[
		Arg::with_name("update")
		.short('u')
		.long("update")
		.value_name("PublishedFileId")
		.takes_value(true)
		.help("Publishes an update.")
		.requires("in")
		.conflicts_with_all(&["out", "extract"]),

		Arg::with_name("in")
		.long("in")
		.value_name("PATH")
		.takes_value(true)
		.help("Sets the directory the GMA for updating will be built from.")
		.requires("update")
		.conflicts_with_all(&["out", "extract"]),

		Arg::with_name("changes")
		.long("changes")
		.value_name("CHANGES")
		.takes_value(true)
		.help("Sets the changelog for an update.")
		.requires("update")
		.conflicts_with_all(&["out", "extract"]),

		Arg::with_name("icon")
		.long("icon")
		.value_name("PATH")
		.takes_value(true)
		.help("Path to a (max 1 MB) JPG/PNG/GIF file for Workshop preview image updating.")
		.requires("update")
		.conflicts_with_all(&["out", "extract"])
	])*/
	.get_matches();

	dprintln!("{:#?}", matches);

	if let Some(extract_path) = matches.value_of("extract") {
		let extract_path = PathBuf::from(extract_path);

		if !extract_path.is_file() {
			eprintln!("Invalid GMA file path provided.");
			return true;
		}

		if let Ok(mut gma) = GMAFile::open(extract_path) {
			let dest = match matches.value_of("out") {
				Some(out) => ExtractDestination::Directory(PathBuf::from(out)),
				None => ExtractDestination::Temp,
			};

			if let Err(err) = gma.extract(dest, &transaction!(), true) {
				eprintln!("Error: {:#?}", err);
			}
		}
	}

	true
}
