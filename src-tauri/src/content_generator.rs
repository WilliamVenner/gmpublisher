use std::{
	fs::File,
	io::{BufReader, BufWriter},
	path::PathBuf,
};

use chrono::Utc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use steamworks::PublishedFileId;

#[derive(Serialize, Deserialize)]
struct AddWorkshopEntry {
	id: PublishedFileId,
	name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AddWorkshopManifest {
	id: u16,
	name: String,
	date: chrono::DateTime<Utc>,
	collection: Option<PublishedFileId>,
	contents: Vec<AddWorkshopEntry>,
}
impl PartialEq for AddWorkshopManifest {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}
impl Eq for AddWorkshopManifest {}
impl PartialOrd for AddWorkshopManifest {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		self.date.partial_cmp(&other.date)
	}
}
impl Ord for AddWorkshopManifest {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.date.cmp(&other.date)
	}
}

#[derive(Serialize, Deserialize)]
pub struct ContentGenerator {
	saved: Vec<AddWorkshopManifest>,
	id: u16,
}
impl ContentGenerator {
	pub fn init() -> Self {
		let mut saved = Vec::new();
		let mut id = 0;

		std::fs::create_dir_all(&*manifests_path()).expect("Failed to create content generator manifests directory");

		if let Ok(dir) = manifests_path().read_dir() {
			for entry in dir {
				(|| -> Option<()> {
					let entry = entry.ok()?;
					let contents: AddWorkshopManifest = bincode::deserialize_from(BufReader::new(File::open(entry.path()).ok()?)).ok()?;
					id = id.max(contents.id);

					saved.insert(
						match saved.binary_search(&contents) {
							Ok(pos) => pos,
							Err(pos) => pos,
						},
						contents,
					);

					Some(())
				})();
			}
		}

		Self { saved, id }
	}
}

lazy_static! {
	pub static ref CONTENT_GENERATOR: Mutex<ContentGenerator> = Mutex::new(ContentGenerator::init());
}

fn manifests_path() -> PathBuf {
	app_data!().user_data_dir().join("content_generator")
}

#[tauri::command]
pub fn get_content_generator_manifests() -> &'static Vec<AddWorkshopManifest> {
	unsafe { &*(&CONTENT_GENERATOR.lock().saved as *const _) }
}

#[tauri::command]
pub fn update_content_generator_manifest(manifest: AddWorkshopManifest) -> bool {
	try_block!({
		let mut content_generator = CONTENT_GENERATOR.lock();

		let f = File::create(manifests_path().join(manifest.id.to_string()))?;
		bincode::serialize_into(BufWriter::new(f), &manifest)?;

		match content_generator.saved.binary_search(&manifest) {
			Ok(pos) => content_generator.saved[pos] = manifest,
			Err(pos) => content_generator.saved.insert(pos, manifest),
		}
	})
	.is_ok()
}
