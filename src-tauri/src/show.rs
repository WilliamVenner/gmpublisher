extern crate msgbox;
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
pub fn opener(url: &str) {
	if opener::open(url).is_err() {
		msg(String::from(url));
	}
}