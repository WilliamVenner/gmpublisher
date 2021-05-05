#![allow(dead_code, unused_imports)]

use std::{
	path::PathBuf,
	process::{Command, Stdio},
};

fn check() -> bool {
	if let Some(arg) = std::env::args_os().nth(1) {
		if arg.to_string_lossy() != "bundle" {
			return false;
		}
	} else {
		return false;
	}
	true
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
pub fn bundler() -> bool {
	check()
}

#[cfg(target_os = "macos")]
pub fn bundler() -> bool {
	if !check() {
		return false;
	}

	let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/bundle/macos/gmpublisher.app/Contents/MacOS/gmpublisher");
	assert!(path.is_file());

	Command::new("install_name_tool")
		.arg("-change")
		.arg("@loader_path/libsteam_api.dylib")
		.arg("@executable_path/../Frameworks/libsteam_api.dylib")
		.arg(path.as_os_str())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.output()
		.expect("Failed to link Steamworks API for MacOS bundle");

	true
}
