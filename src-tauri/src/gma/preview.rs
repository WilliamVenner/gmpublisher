use atomic_refcell::AtomicRefCell;
use parking_lot::Mutex;
use crate::Transaction;

use super::{GMAFile, GMAError, GMAEntry, ExtractDestination};
use std::{any::Any, collections::HashMap, path::PathBuf, sync::Arc};
use crate::gma::extract::ExtractGMAImmut;

lazy_static! {
	static ref PREVIEW_GMA: Mutex<Option<Arc<GMAFile>>> = Mutex::new(None);
}

#[tauri::command]
pub fn preview_gma(path: Option<PathBuf>) -> Result<Option<serde_json::Value>, GMAError> {
	if let Some(path) = path {
		let mut lock = PREVIEW_GMA.lock();

		let mut gma = GMAFile::open(path)?;
		gma.entries()?;
		*lock = Some(Arc::new(gma));

		Ok(Some(json!(lock.as_ref().unwrap().entries.as_ref().unwrap())))
	} else {
		*PREVIEW_GMA.lock() = None;
		Ok(None)
	}
}

#[tauri::command]
pub fn extract_preview_entry(gma_path: PathBuf, entry_path: String) -> Option<u32> {
	let mut lock = PREVIEW_GMA.lock();
	if let Some(gma) = lock.as_mut() {
		if gma.path != gma_path {
			let mut race_gma = GMAFile::open(gma_path).ok()?;
			race_gma.entries().ok()?;
			*gma = Arc::new(race_gma);
		}

		let transaction = transaction!();
		let id = transaction.id;

		let gma_ref = gma.clone();
		rayon::spawn(move || { ignore! { gma_ref.extract_entry(entry_path, &transaction, true) }; });

		Some(id)
	} else {
		None
	}
}

#[tauri::command]
pub fn extract_preview_gma(gma_path: PathBuf, dest: ExtractDestination) -> Option<u32> {
	let mut lock = PREVIEW_GMA.lock();
	if let Some(gma) = lock.as_mut() {
		if gma.path != gma_path {
			let mut race_gma = GMAFile::open(gma_path).ok()?;
			race_gma.entries().ok()?;
			*gma = Arc::new(race_gma);
		}

		let transaction = transaction!();
		let id = transaction.id;

		let gma_ref = gma.clone();
		rayon::spawn(move || { ignore! { gma_ref.extract(dest, &transaction, true) }; });

		Some(id)
	} else {
		None
	}
}
