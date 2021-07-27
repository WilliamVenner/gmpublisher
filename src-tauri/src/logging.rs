use std::{fs::OpenOptions, panic::PanicInfo};

use crossbeam::channel::Sender;

use crate::{ignore, app_data};

pub enum LogMessage {
	Stdout(String),
	Stderr(String)
}

lazy_static! {
	static ref LOGS_DIR: std::path::PathBuf = {
		let mut logs = app_data!().temp_dir().to_path_buf();
		logs.push("logs");
		logs
	};
}

lazy_static! {
	pub static ref LOG_CHANNEL: Sender<LogMessage> = {
		let (tx, rx) = crossbeam::channel::unbounded();
		std::thread::spawn(move || {
			use std::{fs::File, io::Write};

			if let Err(err) = std::fs::create_dir_all(&*LOGS_DIR) {
				return std::eprintln!("Failed to create logs directory: {:#?}", err);
			}

			let mut stdout = match File::create(LOGS_DIR.join("stdout.log")) {
				Ok(f) => f,
				Err(err) => return std::eprintln!("Failed to open stdout log file: {:#?}", err)
			};
			let mut stderr = match File::create(LOGS_DIR.join("stderr.log")) {
				Ok(f) => f,
				Err(err) => return std::eprintln!("Failed to open stderr log file: {:#?}", err)
			};

			while let Ok(log) = rx.recv() {
				match log {
					LogMessage::Stdout(log) => {
						ignore! { stdout.write_all(log.as_bytes()) };
						ignore! { stdout.write_all(&['\n' as u8]) };
					},
					LogMessage::Stderr(log) => {
						ignore! { stderr.write_all(log.as_bytes()) };
						ignore! { stderr.write_all(&['\n' as u8]) };
					},
				}
			}
		});
		tx
	};
}

#[macro_export]
macro_rules! println {
	($($arg:tt)*) => {
		let log = format!($($arg)*);
		std::println!("{}", &log);
		crate::ignore! { crate::logging::LOG_CHANNEL.send(crate::logging::LogMessage::Stdout(log)) };
	};
}

#[macro_export]
macro_rules! eprintln {
	($($arg:tt)*) => {
		let log = format!($($arg)*);
		std::eprintln!("{}", &log);
		crate::ignore! { crate::logging::LOG_CHANNEL.send(crate::logging::LogMessage::Stderr(log)) };
	};
}

pub fn panic(panic: &PanicInfo) {
	use std::io::Write;

	let backtrace = backtrace::Backtrace::new();

	if let Ok(mut f) = OpenOptions::new().append(true).create(true).open(LOGS_DIR.join("stderr.log")) {
		f.sync_data().ok();
		write!(f, "\n\n!!!!!!!!!!!!! PANIC !!!!!!!!!!!!!\n{}\n{:#?}\n\n", panic, &backtrace).ok();
	}

	std::eprintln!("{}\n{:#?}", panic, backtrace);
}
