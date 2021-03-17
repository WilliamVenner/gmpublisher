pub mod path {
    use std::{fmt::Debug, path::PathBuf};
	use serde::{Deserialize, Serialize, de::Visitor};

	pub fn canonicalize(path: PathBuf) -> PathBuf {
		dunce::canonicalize(path.clone()).unwrap_or(path)
	}

	pub fn normalize(path: PathBuf) -> PathBuf {
		#[cfg(not(target_os = "windows"))]
		canonicalize(path);
		
		#[cfg(target_os = "windows")]
		match dunce::canonicalize(&path) {
			Ok(canonicalized) => PathBuf::from(canonicalized.to_string_lossy().to_string().replace('\\', "/")),
			Err(_) => path
		}
	}

	#[derive(Clone)]
	pub struct NormalizedPathBuf {
		pub normalized: PathBuf,
		path: PathBuf
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
        	S: serde::Serializer
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
			D: serde::Deserializer<'de>
		{
			Ok(NormalizedPathBuf::from(
				deserializer.deserialize_string(NormalizedPathBufVisitor)?
			))
		}
	}
}