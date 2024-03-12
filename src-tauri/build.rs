fn main() {
	if cfg!(target_os = "linux") {
		println!("cargo:rustc-link-arg=-Wl,-rpath,.");
	}

    tauri_build::build()
}
