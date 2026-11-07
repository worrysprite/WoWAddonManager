use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Strip WoW UI escape sequences (`|cAARRGGBB`, `|r`, textures, etc.).
pub fn strip_wow_escapes(input: &str) -> String {
	static COLOR: OnceLock<Regex> = OnceLock::new();
	static TEXTURE: OnceLock<Regex> = OnceLock::new();
	static HYPERLINK: OnceLock<Regex> = OnceLock::new();

	let color = COLOR.get_or_init(|| Regex::new(r"(?i)\|c[0-9a-f]{6,8}").expect("color regex"));
	let texture =
		TEXTURE.get_or_init(|| Regex::new(r"(?i)\|T[^|]*\|t").expect("texture regex"));
	let hyperlink =
		HYPERLINK.get_or_init(|| Regex::new(r"(?i)\|H[^|]*\|h(.*?)\|h").expect("link regex"));

	let mut out = hyperlink.replace_all(input, "$1").into_owned();
	out = texture.replace_all(&out, "").into_owned();
	out = color.replace_all(&out, "").into_owned();
	out = out.replace("|r", "").replace("|R", "");
	out = out.replace("||", "|");
	out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Prefer the plain addon name before a colored warning suffix.
pub fn clean_toc_title(input: &str) -> String {
	let trimmed = input.trim();
	if let Some(idx) = trimmed.find("|c").or_else(|| trimmed.find("|C")) {
		let prefix = trimmed[..idx].trim();
		if !prefix.is_empty() {
			return strip_wow_escapes(prefix);
		}
	}
	strip_wow_escapes(trimmed)
}

pub fn clean_toc_text(input: &str) -> String {
	strip_wow_escapes(input.trim())
}

pub fn is_compat_warning(text: &str) -> bool {
	let lower = text.to_ascii_lowercase();
	lower.contains("not supported")
		|| lower.contains("only supports")
		|| lower.contains("unsupported")
		|| lower.contains("wrong client")
		|| text.contains("不支持")
		|| text.contains("仅支持")
}

pub fn clean_toc_notes(input: &str) -> Option<String> {
	let cleaned = clean_toc_text(input);
	if cleaned.is_empty() || is_compat_warning(&cleaned) {
		None
	} else {
		Some(cleaned)
	}
}

#[derive(Debug, Clone, Default)]
pub struct TocMeta {
	pub title: Option<String>,
	pub title_raw: Option<String>,
	pub author: Option<String>,
	pub notes: Option<String>,
	pub notes_raw: Option<String>,
	pub version: Option<String>,
	/// First/raw Interface field (for display).
	pub interface: Option<String>,
	/// All Interface numbers declared in the TOC (multi-value / multi-line).
	pub interfaces: Vec<u32>,
	/// TOC file stem suffix after `{Folder}_`, e.g. `Forever` for `Questie_Forever.toc`.
	pub flavor_suffix: Option<String>,
	pub path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CompatibilityIssue {
	pub reason: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ClientContext<'a> {
	pub flavor_key: Option<&'a str>,
	pub game_version: Option<&'a str>,
}

impl<'a> ClientContext<'a> {
	pub fn from_install(flavor_key: &'a str, game_version: Option<&'a str>) -> Self {
		Self {
			flavor_key: Some(flavor_key),
			game_version,
		}
	}
}

/// Detect TOC markers that mean this single TOC is not for the current client.
pub fn client_unsupported_reason(meta: &TocMeta, game_version: Option<&str>) -> Option<CompatibilityIssue> {
	let title_raw = meta.title_raw.as_deref().unwrap_or("");
	let notes_raw = meta.notes_raw.as_deref().unwrap_or("");
	let title_l = title_raw.to_ascii_lowercase();
	let notes_l = notes_raw.to_ascii_lowercase();
	let title_plain = clean_toc_text(title_raw).to_ascii_lowercase();

	// Prefer explicit wrong-client title markers (e.g. Questie stub TOC).
	if title_l.contains("game client not supported")
		|| title_l.contains("client not supported")
		|| title_plain.contains("game client not supported")
		|| title_plain.contains("client not supported")
	{
		return Some(CompatibilityIssue {
			reason: "当前游戏客户端不支持该插件（TOC 标记为 game client not supported）".into(),
		});
	}

	// Notes: only clear client-gate phrasing, not generic "unsupported feature".
	if notes_l.contains("game client not supported")
		|| notes_l.contains("wrong client")
		|| notes_l.contains("only supports classic")
		|| notes_l.contains("only supports retail")
		|| notes_l.contains("only supports season of discovery")
		|| notes_l.contains("only supports wrath")
		|| notes_l.contains("only supports cataclysm")
		|| notes_raw.contains("仅支持经典")
		|| notes_raw.contains("仅支持正式服")
	{
		return Some(CompatibilityIssue {
			reason: "该插件声明仅支持其他版本客户端，与当前安装不匹配".into(),
		});
	}

	if let Some(gv) = game_version {
		if let Some(reason) = interface_mismatch_reason(meta, gv) {
			return Some(CompatibilityIssue { reason });
		}
	}

	None
}

/// Convert WoW game version like `12.1.5.70783` / `1.15.7` into TOC Interface number.
pub fn game_version_to_interface(game_version: &str) -> Option<u32> {
	let mut parts = game_version
		.split(|c: char| !c.is_ascii_digit())
		.filter(|p| !p.is_empty())
		.filter_map(|p| p.parse::<u32>().ok());
	let major = parts.next()?;
	let minor = parts.next().unwrap_or(0);
	let patch = parts.next().unwrap_or(0);
	Some(major * 10000 + minor * 100 + patch)
}

fn parse_interface_numbers(raw: &str) -> Vec<u32> {
	raw.split(|c: char| !c.is_ascii_digit())
		.filter(|p| !p.is_empty())
		.filter_map(|p| p.parse::<u32>().ok())
		.filter(|&n| n >= 100) // ignore tiny tokens / Interface: 00000 stubs
		.collect()
}

fn interface_mismatch_reason(meta: &TocMeta, game_version: &str) -> Option<String> {
	let expected = game_version_to_interface(game_version)?;
	let exp_major = expected / 10000;

	let mut interfaces = meta.interfaces.clone();
	if interfaces.is_empty() {
		if let Some(raw) = meta.interface.as_deref() {
			interfaces = parse_interface_numbers(raw);
		}
	}
	if interfaces.is_empty() {
		return None;
	}

	if interfaces.iter().any(|n| n / 10000 == exp_major) {
		return None;
	}

	Some(format!(
		"插件 Interface={:?} 与当前客户端版本 {game_version}（约 {expected}）不匹配",
		interfaces
	))
}

fn interfaces_match_game_version(meta: &TocMeta, game_version: Option<&str>) -> bool {
	let Some(gv) = game_version else {
		return false;
	};
	let Some(expected) = game_version_to_interface(gv) else {
		return false;
	};
	let exp_major = expected / 10000;
	!meta.interfaces.is_empty() && meta.interfaces.iter().any(|n| n / 10000 == exp_major)
}

/// TOC filename suffixes commonly used for each flavor key.
fn flavor_toc_suffixes(flavor_key: &str) -> &'static [&'static str] {
	match flavor_key {
		"forever" => &["Forever"],
		"classic" => &["Vanilla", "Classic", "Era", "ClassicEra"],
		"mop-classic" | "classic-beta" | "classic-ptr" => &["Mists", "MoP", "Mist", "Pandaria"],
		"classic-tbc" => &["TBC", "BurningCrusade", "BCC", "Burning"],
		"cataclysm" => &["Cata", "Cataclysm"],
		"wotlk" => &["Wrath", "WotLK", "WLK", "Lich"],
		"retail" | "ptr" | "xptr" => &["Mainline", "Retail", "Standard"],
		_ => &[],
	}
}

fn toc_suffix_matches_flavor(suffix: &str, flavor_key: &str) -> bool {
	let suffix_l = suffix.to_ascii_lowercase();
	flavor_toc_suffixes(flavor_key)
		.iter()
		.any(|alias| suffix_l == alias.to_ascii_lowercase())
}

/// Root-level `{Folder}.toc` / `{Folder}_*.toc` only (skip Libs nested TOCs).
pub fn list_addon_tocs(dir: &Path, folder: &str) -> Vec<PathBuf> {
	let Ok(entries) = fs::read_dir(dir) else {
		return Vec::new();
	};
	let folder_l = folder.to_ascii_lowercase();
	let mut tocs: Vec<PathBuf> = entries
		.flatten()
		.map(|e| e.path())
		.filter(|p| {
			p.extension().and_then(|s| s.to_str()) == Some("toc")
				&& p.file_stem()
					.and_then(|s| s.to_str())
					.map(|stem| {
						let stem_l = stem.to_ascii_lowercase();
						stem_l == folder_l || stem_l.starts_with(&format!("{folder_l}_"))
					})
					.unwrap_or(false)
		})
		.collect();
	tocs.sort();
	tocs
}

fn toc_flavor_suffix(path: &Path, folder: &str) -> Option<String> {
	let stem = path.file_stem()?.to_str()?;
	let folder_l = folder.to_ascii_lowercase();
	let stem_l = stem.to_ascii_lowercase();
	if stem_l == folder_l {
		return None;
	}
	let prefix_l = format!("{folder_l}_");
	if !stem_l.starts_with(&prefix_l) {
		return None;
	}
	let suffix = &stem[folder.len() + 1..];
	(!suffix.is_empty()).then(|| suffix.to_string())
}

fn score_toc_for_client(meta: &TocMeta, ctx: &ClientContext<'_>) -> i32 {
	// Explicit stub / wrong-client TOC should never win when alternatives exist.
	if client_unsupported_reason(meta, None).is_some() {
		return -1000;
	}

	let mut score = 10; // real TOC baseline
	if let (Some(suffix), Some(flavor_key)) = (meta.flavor_suffix.as_deref(), ctx.flavor_key) {
		if toc_suffix_matches_flavor(suffix, flavor_key) {
			score += 100;
		}
	} else if meta.flavor_suffix.is_none() {
		// Plain Addon.toc — ok for single-TOC addons, weak for multi-TOC packs.
		score += 5;
	}

	if interfaces_match_game_version(meta, ctx.game_version) {
		score += 50;
	} else if ctx.game_version.is_some() && !meta.interfaces.is_empty() {
		// Has interfaces but none match this client.
		score -= 20;
	}

	score
}

/// Pick the best TOC for this client. Skips Questie-style unsupported stubs when a
/// flavor-specific TOC (e.g. `Questie_Forever.toc`) is available.
pub fn preferred_toc(dir: &Path, folder: &str) -> Option<PathBuf> {
	preferred_toc_for_client(dir, folder, &ClientContext::default())
}

pub fn preferred_toc_for_client(
	dir: &Path,
	folder: &str,
	ctx: &ClientContext<'_>,
) -> Option<PathBuf> {
	let tocs = list_addon_tocs(dir, folder);
	if tocs.is_empty() {
		// Fallback: any .toc in the folder root.
		return fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).find(|p| {
			p.extension().and_then(|s| s.to_str()) == Some("toc")
		});
	}

	let mut best: Option<(i32, PathBuf)> = None;
	for path in tocs {
		let mut meta = parse_toc_file(&path);
		meta.flavor_suffix = toc_flavor_suffix(&path, folder);
		meta.path = Some(path.clone());
		let score = score_toc_for_client(&meta, ctx);
		match &best {
			Some((best_score, _)) if *best_score >= score => {}
			_ => best = Some((score, path)),
		}
	}
	best.map(|(_, path)| path)
}

pub fn parse_toc_file(path: &Path) -> TocMeta {
	let content = fs::read_to_string(path).unwrap_or_default();
	let mut meta = TocMeta::default();
	meta.path = Some(path.to_path_buf());
	for line in content.lines() {
		let line = line.trim();
		if let Some(rest) = line.strip_prefix("## Title:") {
			meta.title_raw = Some(rest.trim().to_string());
			let cleaned = clean_toc_title(rest);
			if !cleaned.is_empty() {
				meta.title = Some(cleaned);
			}
		} else if let Some(rest) = line.strip_prefix("## Author:") {
			let cleaned = clean_toc_text(rest);
			if !cleaned.is_empty() {
				meta.author = Some(cleaned);
			}
		} else if let Some(rest) = line.strip_prefix("## Notes:") {
			meta.notes_raw = Some(rest.trim().to_string());
			meta.notes = clean_toc_notes(rest);
		} else if let Some(rest) = line.strip_prefix("## Version:") {
			let cleaned = clean_toc_text(rest);
			if !cleaned.is_empty() {
				meta.version = Some(cleaned);
			}
		} else if let Some(rest) = line.strip_prefix("## Interface:") {
			if meta.interface.is_none() {
				meta.interface = Some(rest.trim().to_string());
			}
			meta.interfaces.extend(parse_interface_numbers(rest));
		}
	}
	meta
}

pub fn read_folder_toc(addons_path: &Path, folder: &str) -> Option<TocMeta> {
	read_folder_toc_for_client(addons_path, folder, &ClientContext::default())
}

pub fn read_folder_toc_for_client(
	addons_path: &Path,
	folder: &str,
	ctx: &ClientContext<'_>,
) -> Option<TocMeta> {
	let dir = addons_path.join(folder);
	let toc = preferred_toc_for_client(&dir, folder, ctx)?;
	let mut meta = parse_toc_file(&toc);
	meta.flavor_suffix = toc_flavor_suffix(&toc, folder);
	Some(meta)
}

/// True unsupported only when no TOC is usable for this client.
pub fn folder_unsupported_reason(
	addons_path: &Path,
	folder: &str,
	ctx: &ClientContext<'_>,
) -> Option<CompatibilityIssue> {
	let dir = addons_path.join(folder);
	let tocs = list_addon_tocs(&dir, folder);
	if tocs.is_empty() {
		return None;
	}

	let metas: Vec<TocMeta> = tocs
		.into_iter()
		.map(|path| {
			let mut meta = parse_toc_file(&path);
			meta.flavor_suffix = toc_flavor_suffix(&path, folder);
			meta
		})
		.collect();

	let real_flavor_tocs: Vec<&TocMeta> = metas
		.iter()
		.filter(|m| {
			m.flavor_suffix.is_some() && client_unsupported_reason(m, None).is_none()
		})
		.collect();

	// Multi-TOC packs (Questie_Forever / _Vanilla / …): require an explicit
	// matching flavor TOC. Do not accept another flavor just because Interface
	// majors overlap (e.g. Forever vs Vanilla both in the 1.x range).
	if !real_flavor_tocs.is_empty() {
		if let Some(flavor_key) = ctx.flavor_key {
			if real_flavor_tocs.iter().any(|m| {
				m.flavor_suffix
					.as_deref()
					.is_some_and(|suffix| toc_suffix_matches_flavor(suffix, flavor_key))
			}) {
				return None;
			}
			return Some(CompatibilityIssue {
				reason: format!("未找到适用于 {flavor_key} 客户端的 TOC"),
			});
		}
	}

	let mut best_score = i32::MIN;
	let mut best_meta: Option<&TocMeta> = None;
	for meta in &metas {
		let score = score_toc_for_client(meta, ctx);
		if score > best_score {
			best_score = score;
			best_meta = Some(meta);
		}
	}

	if best_score >= 0 {
		if let Some(meta) = best_meta {
			if let Some(issue) = client_unsupported_reason(meta, ctx.game_version) {
				return Some(issue);
			}
		}
		return None;
	}

	// No usable TOC. Prefer a clear stub reason if present.
	if let Some(stub) = metas
		.iter()
		.find(|m| client_unsupported_reason(m, None).is_some())
	{
		return client_unsupported_reason(stub, ctx.game_version)
			.or_else(|| client_unsupported_reason(stub, None));
	}

	best_meta
		.and_then(|m| client_unsupported_reason(m, ctx.game_version))
		.or_else(|| {
			Some(CompatibilityIssue {
				reason: "未找到适用于当前客户端的 TOC".into(),
			})
		})
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::{SystemTime, UNIX_EPOCH};

	#[test]
	fn strips_questie_unsupported_title() {
		let raw = "Questie|cFFFF0000 game client not supported|r";
		assert_eq!(clean_toc_title(raw), "Questie");
		let meta = TocMeta {
			title_raw: Some(raw.into()),
			notes_raw: Some("Questie only supports Classic official servers.".into()),
			..TocMeta::default()
		};
		assert!(client_unsupported_reason(&meta, Some("12.1.5.70783")).is_some());
	}

	#[test]
	fn strips_color_from_notes() {
		let raw = "|cFFFF0000Questie only supports Classic official servers.|r";
		assert_eq!(
			clean_toc_text(raw),
			"Questie only supports Classic official servers."
		);
		assert!(clean_toc_notes(raw).is_none());
	}

	#[test]
	fn detects_interface_major_mismatch() {
		let meta = TocMeta {
			title: Some("Questie".into()),
			interface: Some("11507".into()),
			interfaces: vec![11507],
			..TocMeta::default()
		};
		let issue = client_unsupported_reason(&meta, Some("12.1.5.70783"));
		assert!(issue.is_some());
	}

	#[test]
	fn allows_matching_interface_major() {
		let meta = TocMeta {
			title: Some("SomeAddon".into()),
			interface: Some("120100".into()),
			interfaces: vec![120100],
			..TocMeta::default()
		};
		assert!(client_unsupported_reason(&meta, Some("12.1.5.70783")).is_none());
	}

	#[test]
	fn allows_when_any_interface_major_matches() {
		let meta = TocMeta {
			title: Some("Multi".into()),
			interface: Some("11507,120100".into()),
			interfaces: vec![11507, 120100],
			..TocMeta::default()
		};
		assert!(client_unsupported_reason(&meta, Some("12.1.5.70783")).is_none());
	}

	#[test]
	fn notes_feature_unsupported_is_not_client_gate() {
		let meta = TocMeta {
			title: Some("HandyNotes".into()),
			notes_raw: Some("Some features are not supported on this build.".into()),
			interface: Some("120100".into()),
			interfaces: vec![120100],
			..TocMeta::default()
		};
		assert!(client_unsupported_reason(&meta, Some("12.1.5.70783")).is_none());
	}

	#[test]
	fn game_version_to_interface_works() {
		assert_eq!(game_version_to_interface("12.1.5.70783"), Some(120105));
		assert_eq!(game_version_to_interface("1.15.7"), Some(11507));
	}

	fn temp_questie_dir() -> PathBuf {
		let dir = std::env::temp_dir().join(format!(
			"wam-questie-toc-{}-{}",
			std::process::id(),
			SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.unwrap()
				.as_nanos()
		));
		let _ = fs::remove_dir_all(&dir);
		let addon = dir.join("Questie");
		fs::create_dir_all(&addon).unwrap();
		fs::write(
			addon.join("Questie.toc"),
			"## Interface: 00000\n## Title: Questie|cFFFF0000 game client not supported|r\n## Notes: Questie only supports Classic official servers.\n",
		)
		.unwrap();
		fs::write(
			addon.join("Questie_Forever.toc"),
			"## Interface: 16001\n## Title: Questie|cFF00FF00 v12.6.2|r\n## Notes: A standalone Classic QuestHelper\n## Version: 12.6.2\n",
		)
		.unwrap();
		fs::write(
			addon.join("Questie_Vanilla.toc"),
			"## Interface: 11508, 11509\n## Title: Questie|cFF00FF00 v12.6.2|r\n## Notes: A standalone Classic QuestHelper\n## Version: 12.6.2\n",
		)
		.unwrap();
		dir
	}

	#[test]
	fn prefers_forever_toc_over_unsupported_stub() {
		let dir = temp_questie_dir();
		let ctx = ClientContext::from_install("forever", Some("1.16.0"));
		let meta = read_folder_toc_for_client(&dir, "Questie", &ctx).unwrap();
		assert_eq!(meta.flavor_suffix.as_deref(), Some("Forever"));
		assert!(meta.title.as_deref().unwrap_or("").contains("Questie"));
		assert!(folder_unsupported_reason(&dir, "Questie", &ctx).is_none());
		let _ = fs::remove_dir_all(dir);
	}

	#[test]
	fn rejects_retail_when_only_stub_matches() {
		let dir = temp_questie_dir();
		let ctx = ClientContext::from_install("retail", Some("12.1.5.70783"));
		let issue = folder_unsupported_reason(&dir, "Questie", &ctx);
		assert!(issue.is_some());
		let _ = fs::remove_dir_all(dir);
	}

	#[test]
	fn prefers_vanilla_toc_for_classic_era() {
		let dir = temp_questie_dir();
		let ctx = ClientContext::from_install("classic", Some("1.15.7"));
		let meta = read_folder_toc_for_client(&dir, "Questie", &ctx).unwrap();
		assert_eq!(meta.flavor_suffix.as_deref(), Some("Vanilla"));
		assert!(folder_unsupported_reason(&dir, "Questie", &ctx).is_none());
		let _ = fs::remove_dir_all(dir);
	}

	#[test]
	fn rejects_other_flavor_even_if_interface_major_overlaps() {
		let dir = std::env::temp_dir().join(format!(
			"wam-questie-no-forever-{}-{}",
			std::process::id(),
			SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.unwrap()
				.as_nanos()
		));
		let _ = fs::remove_dir_all(&dir);
		let addon = dir.join("Questie");
		fs::create_dir_all(&addon).unwrap();
		fs::write(
			addon.join("Questie.toc"),
			"## Interface: 00000\n## Title: Questie|cFFFF0000 game client not supported|r\n## Notes: Questie only supports Classic official servers.\n",
		)
		.unwrap();
		fs::write(
			addon.join("Questie_Vanilla.toc"),
			"## Interface: 11508, 11509\n## Title: Questie|cFF00FF00 v12.6.2|r\n## Notes: A standalone Classic QuestHelper\n## Version: 12.6.2\n",
		)
		.unwrap();
		let ctx = ClientContext::from_install("forever", Some("1.16.0"));
		assert!(folder_unsupported_reason(&dir, "Questie", &ctx).is_some());
		let _ = fs::remove_dir_all(dir);
	}
}
