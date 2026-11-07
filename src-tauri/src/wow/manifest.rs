use crate::types::{AddonManifest, ManifestAddon};
use std::fs;
use std::path::{Path, PathBuf};

pub const MANIFEST_FILE: &str = "wow-addon-manager.json";

pub fn manifest_path(addons_path: &Path) -> PathBuf {
	addons_path.join(MANIFEST_FILE)
}

pub fn load(addons_path: &Path) -> AddonManifest {
	let path = manifest_path(addons_path);
	match fs::read_to_string(path) {
		Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
		Err(_) => AddonManifest {
			version: 1,
			addons: Vec::new(),
		},
	}
}

pub fn save(addons_path: &Path, manifest: &AddonManifest) -> Result<(), String> {
	fs::create_dir_all(addons_path).map_err(|e| e.to_string())?;
	let path = manifest_path(addons_path);
	let content = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
	fs::write(path, content).map_err(|e| e.to_string())
}

pub fn upsert(addons_path: &Path, addon: ManifestAddon) -> Result<(), String> {
	let mut manifest = load(addons_path);
	if let Some(existing) = manifest
		.addons
		.iter_mut()
		.find(|a| a.folder_name == addon.folder_name)
	{
		*existing = addon;
	} else {
		manifest.addons.push(addon);
	}
	manifest.version = 1;
	save(addons_path, &manifest)
}

pub fn remove(addons_path: &Path, folder_name: &str) -> Result<(), String> {
	let mut manifest = load(addons_path);
	manifest.addons.retain(|a| a.folder_name != folder_name);
	save(addons_path, &manifest)
}

pub fn find<'a>(manifest: &'a AddonManifest, folder_name: &str) -> Option<&'a ManifestAddon> {
	manifest.addons.iter().find(|a| a.folder_name == folder_name)
}
