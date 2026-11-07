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
	pub interface: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompatibilityIssue {
	pub reason: String,
}

/// Detect TOC markers that mean this addon is not for the current client.
pub fn client_unsupported_reason(meta: &TocMeta, game_version: Option<&str>) -> Option<CompatibilityIssue> {
	let title_raw = meta.title_raw.as_deref().unwrap_or("");
	let notes_raw = meta.notes_raw.as_deref().unwrap_or("");
	let title_l = title_raw.to_ascii_lowercase();
	let notes_l = notes_raw.to_ascii_lowercase();

	let title_plain = clean_toc_text(title_raw);

	if title_l.contains("game client not supported")
		|| title_l.contains("client not supported")
		|| title_l.contains("not compatible")
		|| is_compat_warning(&title_plain)
	{
		return Some(CompatibilityIssue {
			reason: "当前游戏客户端不支持该插件（TOC 标记为 game client not supported）".into(),
		});
	}

	if notes_l.contains("only supports")
		|| notes_l.contains("not supported")
		|| notes_l.contains("wrong client")
		|| notes_raw.contains("仅支持")
		|| notes_raw.contains("不支持")
	{
		return Some(CompatibilityIssue {
			reason: "该插件声明仅支持其他版本客户端，与当前安装不匹配".into(),
		});
	}

	if let (Some(toc_if), Some(gv)) = (meta.interface.as_deref(), game_version) {
		if let Some(reason) = interface_mismatch_reason(toc_if, gv) {
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

fn interface_mismatch_reason(toc_interface: &str, game_version: &str) -> Option<String> {
	let toc_if = toc_interface
		.split(|c: char| !c.is_ascii_digit())
		.find(|p| !p.is_empty())
		.and_then(|p| p.parse::<u32>().ok())?;
	let expected = game_version_to_interface(game_version)?;

	let toc_major = toc_if / 10000;
	let exp_major = expected / 10000;
	if toc_major == exp_major {
		return None;
	}

	Some(format!(
		"插件 Interface={toc_if} 与当前客户端版本 {game_version}（约 {expected}）不匹配"
	))
}

pub fn preferred_toc(dir: &Path, folder: &str) -> Option<PathBuf> {
	let preferred = dir.join(format!("{folder}.toc"));
	if preferred.exists() {
		return Some(preferred);
	}
	fs::read_dir(dir)
		.ok()?
		.flatten()
		.map(|e| e.path())
		.find(|p| p.extension().and_then(|s| s.to_str()) == Some("toc"))
}

pub fn parse_toc_file(path: &Path) -> TocMeta {
	let content = fs::read_to_string(path).unwrap_or_default();
	let mut meta = TocMeta::default();
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
			meta.interface = Some(rest.trim().to_string());
		}
	}
	meta
}

pub fn read_folder_toc(addons_path: &Path, folder: &str) -> Option<TocMeta> {
	let dir = addons_path.join(folder);
	let toc = preferred_toc(&dir, folder)?;
	Some(parse_toc_file(&toc))
}

#[cfg(test)]
mod tests {
	use super::*;

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
			..TocMeta::default()
		};
		assert!(client_unsupported_reason(&meta, Some("12.1.5.70783")).is_none());
	}

	#[test]
	fn game_version_to_interface_works() {
		assert_eq!(game_version_to_interface("12.1.5.70783"), Some(120105));
		assert_eq!(game_version_to_interface("1.15.7"), Some(11507));
	}
}
