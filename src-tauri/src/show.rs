extern crate msgbox;
use std::{ffi::OsStr, path::PathBuf};

use msgbox::IconType;

#[allow(dead_code)]
pub fn panic(panic: String) {
	if msgbox::create("gmpublisher PANIC", &panic, IconType::Error).is_err() {
		panic!("gmpublisher PANIC: {}", &panic);
	}
}

#[allow(dead_code)]
pub fn error(err: String) {
	if msgbox::create("gmpublisher ERROR", &err, IconType::Error).is_err() {
		println!("gmpublisher ERROR: {}", &err);
	}
}

#[allow(dead_code)]
pub fn msg(msg: String) {
	if msgbox::create("gmpublisher", &msg, IconType::Info).is_err() {
		println!("gmpublisher INFO: {}", &msg);
	}
}

#[allow(dead_code)]
pub fn open(url: &str) {
	if opener::open(url).is_err() {
		msg(String::from(url));
	}
}

// TODO use a crate, or make a crate for this?
pub fn open_file_location(path: String) -> Result<std::process::Child, std::io::Error> {
	#[cfg(target_os = "windows")]
	return std::process::Command::new("explorer")
		.arg(format!("/select,{}", path))
		.spawn();

	#[cfg(target_os = "macos")]
	return std::process::Command::new("open")
		.arg("-R")
		.arg(path)
		.spawn();

	#[cfg(target_os = "linux")]
	return std::process::Command::new("xdg-open")
		.arg("--select")
		.arg(path)
		.spawn();

	#[allow(unreachable_code)]
	Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Unsupported OS"))
}
