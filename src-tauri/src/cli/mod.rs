// 1. CLI
// 2. File associations
// 3. Context menu option

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod win;
