use crate::types::WowInstall;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
struct FlavorMeta {
	folder: &'static str,
	flavor_key: &'static str,
	label: &'static str,
	game_version_type_id: i64,
}

const FLAVORS: &[FlavorMeta] = &[
	FlavorMeta {
		folder: "_retail_",
		flavor_key: "retail",
		label: "Retail",
		game_version_type_id: 517,
	},
	FlavorMeta {
		folder: "_classic_",
		flavor_key: "mop-classic",
		label: "MoP Classic",
		game_version_type_id: 79434,
	},
	FlavorMeta {
		folder: "_classic_era_",
		flavor_key: "classic",
		label: "Classic",
		game_version_type_id: 67408,
	},
	FlavorMeta {
		folder: "_classic_beta_",
		flavor_key: "classic-beta",
		label: "Classic Beta",
		game_version_type_id: 79434,
	},
	FlavorMeta {
		folder: "_classic_ptr_",
		flavor_key: "classic-ptr",
		label: "Classic PTR",
		game_version_type_id: 79434,
	},
	FlavorMeta {
		folder: "_ptr_",
		flavor_key: "ptr",
		label: "PTR",
		game_version_type_id: 517,
	},
	FlavorMeta {
		folder: "_xptr_",
		flavor_key: "xptr",
		label: "xPTR",
		game_version_type_id: 517,
	},
	FlavorMeta {
		folder: "_forever_",
		flavor_key: "forever",
		label: "Forever",
		game_version_type_id: 88568,
	},
	FlavorMeta {
		folder: "_anniversary_",
		flavor_key: "classic-tbc",
		label: "Classic TBC",
		game_version_type_id: 73246,
	},
];

pub fn detect_installs() -> Vec<WowInstall> {
	let mut found: BTreeMap<String, WowInstall> = BTreeMap::new();

	for root in candidate_roots() {
		collect_from_root(&root, &mut found);
	}

	found.into_values().collect()
}

pub fn install_from_path(path: &str) -> Result<Vec<WowInstall>, String> {
	let path = PathBuf::from(path);
	if !path.exists() {
		return Err("路径不存在".into());
	}

	let mut found: BTreeMap<String, WowInstall> = BTreeMap::new();

	// User selected a flavor folder directly.
	if is_flavor_dir(&path) {
		if let Some(install) = build_install(&path) {
			found.insert(install.id.clone(), install);
		}
	}

	// User selected World of Warcraft root.
	collect_from_root(&path, &mut found);

	// User selected parent containing "World of Warcraft".
	if found.is_empty() {
		for entry in fs::read_dir(&path).map_err(|e| e.to_string())? {
			let entry = entry.map_err(|e| e.to_string())?;
			let child = entry.path();
			if child.is_dir() {
				collect_from_root(&child, &mut found);
			}
		}
	}

	if found.is_empty() {
		Err("未在该路径下识别到魔兽世界客户端".into())
	} else {
		Ok(found.into_values().collect())
	}
}

fn collect_from_root(root: &Path, found: &mut BTreeMap<String, WowInstall>) {
	if !root.exists() {
		return;
	}
	for meta in FLAVORS {
		let flavor_path = root.join(meta.folder);
		if let Some(install) = build_install_with_meta(&flavor_path, meta) {
			found.insert(install.id.clone(), install);
		}
	}

	// Also scan unknown _*_ folders that contain Interface/AddOns.
	if let Ok(entries) = fs::read_dir(root) {
		for entry in entries.flatten() {
			let path = entry.path();
			if !path.is_dir() {
				continue;
			}
			let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
			if !(name.starts_with('_') && name.ends_with('_')) {
				continue;
			}
			if FLAVORS.iter().any(|f| f.folder == name) {
				continue;
			}
			if let Some(install) = build_install(&path) {
				found.entry(install.id.clone()).or_insert(install);
			}
		}
	}
}

fn build_install(path: &Path) -> Option<WowInstall> {
	let name = path.file_name()?.to_str()?;
	if let Some(meta) = FLAVORS.iter().find(|f| f.folder == name) {
		return build_install_with_meta(path, meta);
	}
	if !path.join("Interface").join("AddOns").exists() && !looks_like_client(path) {
		return None;
	}
	let label = name.trim_matches('_').replace('_', " ");
	Some(WowInstall {
		id: path_id(path),
		label: format!("WoW ({label})"),
		flavor: label.clone(),
		flavor_key: name.trim_matches('_').to_string(),
		install_path: path.display().to_string(),
		addons_path: path.join("Interface").join("AddOns").display().to_string(),
		game_version: read_game_version(path),
		game_version_type_id: None,
	})
}

fn build_install_with_meta(path: &Path, meta: &FlavorMeta) -> Option<WowInstall> {
	if !path.exists() {
		return None;
	}
	if !looks_like_client(path) && !path.join("Interface").join("AddOns").exists() {
		return None;
	}
	let addons_path = path.join("Interface").join("AddOns");
	let _ = fs::create_dir_all(&addons_path);
	Some(WowInstall {
		id: path_id(path),
		label: meta.label.to_string(),
		flavor: meta.label.to_string(),
		flavor_key: meta.flavor_key.to_string(),
		install_path: path.display().to_string(),
		addons_path: addons_path.display().to_string(),
		game_version: read_game_version(path),
		game_version_type_id: Some(meta.game_version_type_id),
	})
}

fn looks_like_client(path: &Path) -> bool {
	path.join("Wow.exe").exists()
		|| path.join("WowClassic.exe").exists()
		|| path.join("World of Warcraft Launcher.exe").exists()
		|| path.join(".build.info").exists()
		|| path.join("Interface").exists()
}

fn is_flavor_dir(path: &Path) -> bool {
	let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
	name.starts_with('_') && name.ends_with('_')
}

fn path_id(path: &Path) -> String {
	use std::collections::hash_map::DefaultHasher;
	use std::hash::{Hash, Hasher};
	let mut hasher = DefaultHasher::new();
	path.display().to_string().to_lowercase().hash(&mut hasher);
	format!("wow-{:x}", hasher.finish())
}

fn read_game_version(path: &Path) -> Option<String> {
	let build_info = path.join(".build.info");
	if let Ok(content) = fs::read_to_string(build_info) {
		for line in content.lines().skip(1) {
			let cols: Vec<_> = line.split('|').collect();
			if cols.len() > 5 {
				let version = cols[5].trim();
				if !version.is_empty() {
					return Some(version.to_string());
				}
			}
		}
	}
	None
}

fn candidate_roots() -> Vec<PathBuf> {
	let mut roots = Vec::new();

	#[cfg(windows)]
	{
		let drives = ["C", "D", "E", "F"];
		for drive in drives {
			roots.push(PathBuf::from(format!(
				r"{drive}:\Program Files (x86)\World of Warcraft"
			)));
			roots.push(PathBuf::from(format!(
				r"{drive}:\Program Files\World of Warcraft"
			)));
			roots.push(PathBuf::from(format!(r"{drive}:\Games\World of Warcraft")));
			roots.push(PathBuf::from(format!(r"{drive}:\Battle.net\Games\World of Warcraft")));
		}
		if let Ok(home) = std::env::var("USERPROFILE") {
			roots.push(PathBuf::from(home).join("World of Warcraft"));
		}
	}

	#[cfg(not(windows))]
	{
		if let Some(home) = dirs::home_dir() {
			roots.push(home.join("Games/World of Warcraft"));
			roots.push(home.join(".wine/drive_c/Program Files (x86)/World of Warcraft"));
			roots.push(home.join(".wine/drive_c/Program Files/World of Warcraft"));
			roots.push(home.join("wow-mock"));
		}
		roots.push(PathBuf::from("/opt/world-of-warcraft"));
	}

	roots
}
