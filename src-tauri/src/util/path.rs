use serde::{de::Visitor, Deserialize, Serialize};
use std::{
	fmt::Debug,
	path::{Path, PathBuf},
};

pub fn canonicalize(path: PathBuf) -> PathBuf {
	dunce::canonicalize(path.clone()).unwrap_or(path)
}

#[cfg(not(target_os = "windows"))]
pub fn normalize(path: PathBuf) -> PathBuf {
	canonicalize(path)
}

#[cfg(target_os = "windows")]
pub fn normalize(path: PathBuf) -> PathBuf {
	match dunce::canonicalize(&path) {
		Ok(canonicalized) => PathBuf::from(canonicalized.to_string_lossy().to_string().replace('\\', "/")),
		Err(_) => path,
	}
}

#[derive(Clone)]
pub struct NormalizedPathBuf {
	pub normalized: PathBuf,
	path: PathBuf,
}
impl NormalizedPathBuf {
	pub fn new() -> NormalizedPathBuf {
		NormalizedPathBuf {
			path: PathBuf::new(),
			normalized: PathBuf::new(),
		}
	}
}
impl AsRef<Path> for NormalizedPathBuf {
	fn as_ref(&self) -> &Path {
		self.path.as_ref()
	}
}
impl PartialEq for NormalizedPathBuf {
	fn eq(&self, other: &Self) -> bool {
		self.path.eq(&other.path)
	}
}
impl Eq for NormalizedPathBuf {}
impl PartialOrd for NormalizedPathBuf {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.path.partial_cmp(&other.path)
	}
}
impl Ord for NormalizedPathBuf {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.path.cmp(&other.path)
	}
}
impl std::ops::Deref for NormalizedPathBuf {
	type Target = PathBuf;
	fn deref(&self) -> &Self::Target {
		&self.path
	}
}
impl From<PathBuf> for NormalizedPathBuf {
	fn from(path: PathBuf) -> Self {
		Self {
			path: path.clone(),
			normalized: normalize(path),
		}
	}
}
impl From<&PathBuf> for NormalizedPathBuf {
	fn from(path: &PathBuf) -> Self {
		let path = path.to_owned();
		Self {
			path: path.clone(),
			normalized: normalize(path),
		}
	}
}
impl From<String> for NormalizedPathBuf {
	fn from(path: String) -> Self {
		let path = PathBuf::from(path);
		Self {
			path: path.clone(),
			normalized: normalize(path),
		}
	}
}
impl From<&str> for NormalizedPathBuf {
	fn from(path: &str) -> Self {
		let path = PathBuf::from(path);
		Self {
			path: path.clone(),
			normalized: normalize(path),
		}
	}
}
impl Debug for NormalizedPathBuf {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		self.path.fmt(f)
	}
}

impl Serialize for NormalizedPathBuf {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(&self.normalized.to_string_lossy())
	}
}

struct NormalizedPathBufVisitor;
impl<'de> Visitor<'de> for NormalizedPathBufVisitor {
	type Value = String;

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.write_str("a string")
	}
}
impl<'de> Deserialize<'de> for NormalizedPathBuf {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Ok(NormalizedPathBuf::from(deserializer.deserialize_string(NormalizedPathBufVisitor)?))
	}
}

#[inline]
pub fn has_extension<P: AsRef<Path>, S: AsRef<str>>(path: P, extension: S) -> bool {
	path.as_ref()
		.extension()
		.and_then(|x| Some(x.to_str().and_then(|x| Some(x.eq_ignore_ascii_case(extension.as_ref()))).unwrap_or(false)))
		.unwrap_or(false)
}

pub fn open<P: AsRef<Path>>(path: P) {
	let path = path.as_ref();
	
	#[cfg(target_os = "linux")]{
		let _ = std::process::Command::new("xdg-open").arg(path).spawn();
		return;
	}

	if let Err(_) = opener::open(path) {
		tauri::api::dialog::message(None::<&tauri::Window<tauri::Wry>>, "File", path.to_string_lossy());
	}
}

pub fn open_file_location<P: AsRef<Path>>(path: P) {
	let canonicalized = dunce::canonicalize(path.as_ref()).unwrap_or_else(|_| path.as_ref().to_path_buf());
	let path = canonicalized.display();

	if let Err(_) = (|| {
		#[cfg(target_os = "windows")]
		return std::process::Command::new("explorer").arg(format!("/select,{}", path)).spawn();

		#[cfg(target_os = "macos")]
		return std::process::Command::new("open").arg("-R").arg(path.to_string()).spawn();

		#[cfg(target_os = "linux")]
		return std::process::Command::new("xdg-open").arg(canonicalized.parent().unwrap_or(std::path::Path::new("."))).spawn();

		#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
		Err(std::io::Error::new(std::io::ErrorKind::Other, "Unsupported OS"))
	})() {
		tauri::api::dialog::message(None::<&tauri::Window<tauri::Wry>>, "File Location", path.to_string());
	}
}
