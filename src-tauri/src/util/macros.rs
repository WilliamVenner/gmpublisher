#[macro_export]
macro_rules! ignore {
	( $x:expr ) => {
		#[cfg(debug_assertions)]
		$x.unwrap();
		#[cfg(not(debug_assertions))]
		$x
	};
}

#[macro_export]
macro_rules! dprintln {
	($($x:expr),+) => {
		#[cfg(debug_assertions)]
		println!($($x),+)
	};
}

#[macro_export]
macro_rules! sleep {
	( $x:expr ) => {
		std::thread::sleep(std::time::Duration::from_secs($x))
	};
}

#[macro_export]
macro_rules! sleep_ms {
	( $x:expr ) => {
		std::thread::sleep(std::time::Duration::from_millis($x))
	};
}

#[macro_export]
macro_rules! main_thread_forbidden {
	() => {
		#[cfg(debug_assertions)]
		if !*crate::cli::CLI_MODE {
			debug_assert_ne!(
				std::thread::current().name(),
				Some("main"),
				"This should never be called from the main thread"
			);
		}
	};
}

#[macro_export]
macro_rules! json {
	( $x:expr ) => {
		serde_json::to_value($x).unwrap()
	};
}

#[macro_export]
macro_rules! mutex_wait {
	( $mutex:expr, $loop:block ) => {
		loop {
			if let Some(lock) = $mutex.try_lock() {
				if lock.is_some() {
					break;
				}
			}
			$loop
		}
	};
}

#[macro_export]
macro_rules! try_block {
	( $code:block ) => {
		(|| -> Result<(), anyhow::Error> {
			$code
			Ok(())
		})()
	};

	( $code:block, $ty:ty ) => {
		(|| -> Result<$ty, anyhow::Error> { $code })()
	};
}

lazy_static! {
	pub static ref NUM_THREADS: usize = usize::max(num_cpus::get() - 2, 2);
}
#[macro_export]
macro_rules! thread_pool {
	( $n:expr ) => {
		rayon::ThreadPoolBuilder::new().num_threads(isize::max(isize::min($n - 2, num_cpus::get() as isize), 2) as usize).build().unwrap()
	};

	() => {
		rayon::ThreadPoolBuilder::new().num_threads(*crate::NUM_THREADS).build().unwrap()
	};
}
