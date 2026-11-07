use crate::types::WowInstall;
use std::collections::{BTreeMap, BTreeSet};
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

const CLIENT_EXES: &[&str] = &[
	"Wow.exe",
	"WowClassic.exe",
	"WowB.exe",
	"WowT.exe",
	"WowClassicT.exe",
	"WowClassicB.exe",
	"Wow-64.exe",
	"WowClassic-64.exe",
	"Wow-ARM64.exe",
	"World of Warcraft Launcher.exe",
];

#[derive(Debug, Clone, Default)]
pub struct DetectionReport {
	pub installs: Vec<WowInstall>,
	pub invalid_roots: Vec<String>,
}

pub fn detect_installs() -> Vec<WowInstall> {
	detect_with_extra_roots(&[]).installs
}

pub fn detect_with_extra_roots(extra_roots: &[String]) -> DetectionReport {
	let mut found: BTreeMap<String, WowInstall> = BTreeMap::new();
	let mut scanned = BTreeSet::new();
	let mut invalid = Vec::new();

	for root in candidate_roots() {
		let key = normalize_path_key(&root);
		if !scanned.insert(key) {
			continue;
		}
		collect_from_root(&root, &mut found);
	}

	for root in extra_roots {
		let path = PathBuf::from(root);
		let key = normalize_path_key(&path);
		scanned.insert(key);
		match install_from_path(root) {
			Ok(items) if !items.is_empty() => {
				for item in items {
					found.insert(item.id.clone(), item);
				}
			}
			_ => {
				if path.exists() {
					invalid.push(root.clone());
				} else {
					invalid.push(format!("{root}（路径不存在）"));
				}
			}
		}
	}

	let mut installs: Vec<_> = found.into_values().collect();
	installs.sort_by(|a, b| a.label.cmp(&b.label));

	let _ = scanned;
	DetectionReport {
		installs,
		invalid_roots: invalid,
	}
}

pub fn install_from_path(path: &str) -> Result<Vec<WowInstall>, String> {
	let path = PathBuf::from(path);
	if !path.exists() {
		return Err("路径不存在".into());
	}

	let mut found: BTreeMap<String, WowInstall> = BTreeMap::new();

	// Selected a flavor folder directly (_retail_, etc.).
	if is_flavor_dir(&path) {
		if let Some(install) = build_install(&path) {
			found.insert(install.id.clone(), install);
		}
		// Also try parent as WoW root.
		if let Some(parent) = path.parent() {
			collect_from_root(parent, &mut found);
		}
	}

	// Selected World of Warcraft root.
	collect_from_root(&path, &mut found);

	// Selected parent containing "World of Warcraft".
	if found.is_empty() {
		if let Ok(entries) = fs::read_dir(&path) {
			for entry in entries.flatten() {
				let child = entry.path();
				if child.is_dir() && is_wow_root(&child) {
					collect_from_root(&child, &mut found);
				}
			}
		}
	}

	// Selected an executable inside a client folder.
	if found.is_empty() && path.is_file() {
		if let Some(parent) = path.parent() {
			if is_flavor_dir(parent) {
				if let Some(install) = build_install(parent) {
					found.insert(install.id.clone(), install);
				}
			}
			collect_from_root(parent, &mut found);
			if let Some(grand) = parent.parent() {
				collect_from_root(grand, &mut found);
			}
		}
	}

	if found.is_empty() {
		Err("未在该路径下识别到魔兽世界客户端。请选择「World of Warcraft」根目录，或其中的 _retail_ / _classic_ 等版本目录。".into())
	} else {
		Ok(found.into_values().collect())
	}
}

fn collect_from_root(root: &Path, found: &mut BTreeMap<String, WowInstall>) {
	if !root.exists() {
		return;
	}

	// If user pointed at a flavor dir, treat its parent as root too.
	let root = if is_flavor_dir(root) {
		root.parent().unwrap_or(root)
	} else {
		root
	};

	for meta in FLAVORS {
		let flavor_path = root.join(meta.folder);
		if let Some(install) = build_install_with_meta(&flavor_path, meta, root) {
			found.insert(install.id.clone(), install);
		}
	}

	// Unknown _*_ folders that look like clients.
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
		let root = path.parent().unwrap_or(path);
		return build_install_with_meta(path, meta, root);
	}
	if !looks_like_client(path) {
		return None;
	}
	let label = name.trim_matches('_').replace('_', " ");
	let root = path.parent().unwrap_or(path);
	Some(WowInstall {
		id: path_id(path),
		label: format!("WoW ({label})"),
		flavor: label.clone(),
		flavor_key: name.trim_matches('_').to_string(),
		install_path: path.display().to_string(),
		addons_path: path.join("Interface").join("AddOns").display().to_string(),
		game_version: read_game_version(path, root),
		game_version_type_id: None,
	})
}

fn build_install_with_meta(path: &Path, meta: &FlavorMeta, root: &Path) -> Option<WowInstall> {
	if !path.exists() {
		return None;
	}
	if !looks_like_client(path) {
		return None;
	}
	let addons_path = path.join("Interface").join("AddOns");
	Some(WowInstall {
		id: path_id(path),
		label: meta.label.to_string(),
		flavor: meta.label.to_string(),
		flavor_key: meta.flavor_key.to_string(),
		install_path: path.display().to_string(),
		addons_path: addons_path.display().to_string(),
		game_version: read_game_version(path, root),
		game_version_type_id: Some(meta.game_version_type_id),
	})
}

fn looks_like_client(path: &Path) -> bool {
	if CLIENT_EXES.iter().any(|exe| path.join(exe).exists()) {
		return true;
	}
	if path.join(".flavor.info").exists() {
		return true;
	}
	// Common after first launch: Interface exists even if AddOns empty.
	if path.join("Interface").exists() && (path.join("WTF").exists() || path.join("Cache").exists())
	{
		return true;
	}
	// Bare but valid client data layout.
	path.join("Interface").join("AddOns").exists()
}

fn is_wow_root(path: &Path) -> bool {
	if !path.is_dir() {
		return false;
	}
	if path.join(".build.info").exists() {
		return true;
	}
	if path.join("Data").exists()
		&& FLAVORS
			.iter()
			.any(|f| path.join(f.folder).exists() && looks_like_client(&path.join(f.folder)))
	{
		return true;
	}
	FLAVORS
		.iter()
		.any(|f| path.join(f.folder).exists() && looks_like_client(&path.join(f.folder)))
}

fn is_flavor_dir(path: &Path) -> bool {
	let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
	name.starts_with('_') && name.ends_with('_')
}

fn path_id(path: &Path) -> String {
	use std::collections::hash_map::DefaultHasher;
	use std::hash::{Hash, Hasher};
	let mut hasher = DefaultHasher::new();
	normalize_path_key(path).hash(&mut hasher);
	format!("wow-{:x}", hasher.finish())
}

fn normalize_path_key(path: &Path) -> String {
	path.display().to_string().replace('\\', "/").to_lowercase()
}

fn read_game_version(client_path: &Path, root: &Path) -> Option<String> {
	for candidate in [
		client_path.join(".build.info"),
		root.join(".build.info"),
		client_path
			.parent()
			.map(|p| p.join(".build.info"))
			.unwrap_or_default(),
	] {
		if let Some(version) = parse_build_info_version(&candidate) {
			return Some(version);
		}
	}
	None
}

fn parse_build_info_version(path: &Path) -> Option<String> {
	let content = fs::read_to_string(path).ok()?;
	let mut lines = content.lines();
	let header = lines.next()?;
	let cols: Vec<_> = header.split('|').map(|s| s.trim().to_lowercase()).collect();
	let version_idx = cols.iter().position(|c| c.starts_with("version"))?;
	for line in lines {
		let fields: Vec<_> = line.split('|').collect();
		if let Some(version) = fields.get(version_idx).map(|s| s.trim()).filter(|s| !s.is_empty())
		{
			return Some(version.to_string());
		}
	}
	None
}

fn candidate_roots() -> Vec<PathBuf> {
	let mut roots = BTreeSet::new();

	for path in registry_install_paths() {
		push_root_candidate(&mut roots, path);
	}
	for path in battlenet_config_paths() {
		push_root_candidate(&mut roots, path);
	}
	for path in agent_product_db_paths() {
		push_root_candidate(&mut roots, path);
	}
	for path in well_known_paths() {
		push_root_candidate(&mut roots, path);
	}
	for path in drive_scan_wow_folders() {
		push_root_candidate(&mut roots, path);
	}

	roots.into_iter().collect()
}

fn push_root_candidate(out: &mut BTreeSet<PathBuf>, path: PathBuf) {
	if !path.exists() {
		return;
	}
	if is_flavor_dir(&path) {
		if let Some(parent) = path.parent() {
			out.insert(parent.to_path_buf());
		}
		return;
	}
	if is_wow_root(&path) {
		out.insert(path);
	}
}

fn well_known_paths() -> Vec<PathBuf> {
	let mut paths = Vec::new();

	#[cfg(windows)]
	{
		let drives = ["C", "D", "E", "F", "G", "H"];
		let suffixes = [
			r"Program Files (x86)\World of Warcraft",
			r"Program Files\World of Warcraft",
			r"Games\World of Warcraft",
			r"Battle.net\Games\World of Warcraft",
			r"Blizzard\World of Warcraft",
			r"Program Files (x86)\Battle.net\Games\World of Warcraft",
		];
		for drive in drives {
			for suffix in suffixes {
				paths.push(PathBuf::from(format!(r"{drive}:\{suffix}")));
			}
		}
		if let Ok(home) = std::env::var("USERPROFILE") {
			paths.push(PathBuf::from(&home).join(r"Games\World of Warcraft"));
			paths.push(PathBuf::from(home).join("World of Warcraft"));
		}
		if let Ok(program_files) = std::env::var("ProgramFiles(x86)") {
			paths.push(PathBuf::from(program_files).join("World of Warcraft"));
		}
		if let Ok(program_files) = std::env::var("ProgramFiles") {
			paths.push(PathBuf::from(program_files).join("World of Warcraft"));
		}
	}

	#[cfg(not(windows))]
	{
		if let Some(home) = dirs::home_dir() {
			paths.push(home.join("Games/World of Warcraft"));
			paths.push(home.join(".wine/drive_c/Program Files (x86)/World of Warcraft"));
			paths.push(home.join(".wine/drive_c/Program Files/World of Warcraft"));
			paths.push(home.join("wow-mock"));
		}
		paths.push(PathBuf::from("/opt/world-of-warcraft"));
	}

	paths
}

fn drive_scan_wow_folders() -> Vec<PathBuf> {
	#[cfg(windows)]
	{
		let mut found = Vec::new();
		for drive in b'C'..=b'Z' {
			let letter = drive as char;
			let root = PathBuf::from(format!(r"{letter}:\"));
			if !root.exists() {
				continue;
			}
			// Shallow scan: drive root and a few common parents.
			scan_dir_for_wow_name(&root, 0, 1, &mut found);
			for child_name in ["Games", "Battle.net", "Blizzard", "Program Files", "Program Files (x86)"]
			{
				let child = root.join(child_name);
				if child.is_dir() {
					scan_dir_for_wow_name(&child, 0, 2, &mut found);
				}
			}
		}
		found
	}
	#[cfg(not(windows))]
	{
		Vec::new()
	}
}

#[cfg(windows)]
fn scan_dir_for_wow_name(dir: &Path, depth: usize, max_depth: usize, out: &mut Vec<PathBuf>) {
	if depth > max_depth {
		return;
	}
	let Ok(entries) = fs::read_dir(dir) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		if !path.is_dir() {
			continue;
		}
		let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
		if name.eq_ignore_ascii_case("World of Warcraft")
			|| name.eq_ignore_ascii_case("World of Warcraft Public Test")
		{
			out.push(path.clone());
		} else if depth < max_depth {
			scan_dir_for_wow_name(&path, depth + 1, max_depth, out);
		}
	}
}

fn registry_install_paths() -> Vec<PathBuf> {
	#[cfg(windows)]
	{
		use winreg::enums::*;
		use winreg::RegKey;
		let mut paths = Vec::new();
		let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
		for sub in [
			r"SOFTWARE\WOW6432Node\Blizzard Entertainment\World of Warcraft",
			r"SOFTWARE\Blizzard Entertainment\World of Warcraft",
		] {
			if let Ok(key) = hklm.open_subkey(sub) {
				for value_name in ["InstallPath", "InstallPath64", "GamePath"] {
					if let Ok(value) = key.get_value::<String, _>(value_name) {
						let path = PathBuf::from(value.trim_end_matches(['\\', '/']));
						// GamePath may point to Launcher.exe
						if path.is_file() {
							if let Some(parent) = path.parent() {
								paths.push(parent.to_path_buf());
							}
						} else {
							paths.push(path);
						}
					}
				}
			}
		}
		paths
	}
	#[cfg(not(windows))]
	{
		Vec::new()
	}
}

fn battlenet_config_paths() -> Vec<PathBuf> {
	let mut paths = Vec::new();
	let Some(config_dir) = dirs::config_dir() else {
		return paths;
	};
	// Windows: %AppData%\Battle.net\Battle.net.config
	let candidates = [
		config_dir.join("Battle.net").join("Battle.net.config"),
		config_dir
			.join("Battle.net")
			.join("Battle.net.config.json"),
	];
	for candidate in candidates {
		if let Ok(content) = fs::read_to_string(&candidate) {
			extract_wow_paths_from_text(&content, &mut paths);
		}
	}
	paths
}

fn agent_product_db_paths() -> Vec<PathBuf> {
	let mut paths = Vec::new();
	let mut db_files = Vec::new();

	#[cfg(windows)]
	{
		if let Ok(program_data) = std::env::var("ProgramData") {
			db_files.push(PathBuf::from(program_data).join(r"Battle.net\Agent\product.db"));
		}
		db_files.push(PathBuf::from(r"C:\ProgramData\Battle.net\Agent\product.db"));
	}
	#[cfg(not(windows))]
	{
		if let Some(home) = dirs::home_dir() {
			db_files.push(
				home.join(".wine/drive_c/ProgramData/Battle.net/Agent/product.db"),
			);
		}
	}

	for db in db_files {
		if let Ok(bytes) = fs::read(&db) {
			extract_wow_paths_from_bytes(&bytes, &mut paths);
		}
	}
	paths
}

fn extract_wow_paths_from_text(text: &str, out: &mut Vec<PathBuf>) {
	for raw in text.split(|c: char| c == '"' || c == '\'' || c.is_whitespace() || c == ',') {
		push_if_wow_path(raw, out);
	}
}

fn extract_wow_paths_from_bytes(bytes: &[u8], out: &mut Vec<PathBuf>) {
	// UTF-8-ish printable runs.
	let mut current = String::new();
	for &b in bytes {
		if (0x20..=0x7e).contains(&b) || b == b'\\' || b == b'/' {
			current.push(b as char);
		} else if !current.is_empty() {
			push_if_wow_path(&current, out);
			current.clear();
		}
	}
	if !current.is_empty() {
		push_if_wow_path(&current, out);
	}

	// UTF-16LE runs.
	let mut utf16 = String::new();
	let mut i = 0;
	while i + 1 < bytes.len() {
		let code = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
		if (0x20..=0x7e).contains(&code) || code == b'\\' as u16 || code == b'/' as u16 {
			if let Some(ch) = char::from_u32(code as u32) {
				utf16.push(ch);
			}
			i += 2;
		} else {
			if !utf16.is_empty() {
				push_if_wow_path(&utf16, out);
				utf16.clear();
			}
			i += 2;
		}
	}
	if !utf16.is_empty() {
		push_if_wow_path(&utf16, out);
	}
}

fn push_if_wow_path(raw: &str, out: &mut Vec<PathBuf>) {
	let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
	if trimmed.len() < 8 {
		return;
	}
	let lower = trimmed.to_lowercase().replace('/', "\\");
	if !(lower.contains(r"\world of warcraft") || lower.ends_with("world of warcraft")) {
		return;
	}
	let mut path = PathBuf::from(trimmed.replace('/', "\\"));
	// Strip trailing exe / flavor.
	if path.is_file() || CLIENT_EXES.iter().any(|exe| lower.ends_with(&exe.to_lowercase())) {
		if let Some(parent) = path.parent() {
			path = parent.to_path_buf();
		}
	}
	if is_flavor_dir(&path) {
		if let Some(parent) = path.parent() {
			path = parent.to_path_buf();
		}
	}
	// Truncate to the World of Warcraft root segment.
	if let Some(root) = truncate_to_wow_root(&path) {
		out.push(root);
	}
}

fn truncate_to_wow_root(path: &Path) -> Option<PathBuf> {
	let mut current = path.to_path_buf();
	loop {
		let name = current.file_name()?.to_str()?.to_lowercase();
		if name == "world of warcraft" || name == "world of warcraft public test" {
			return Some(current);
		}
		current = current.parent()?.to_path_buf();
	}
}
