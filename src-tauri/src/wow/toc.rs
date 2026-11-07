use regex::Regex;
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
	// Collapse leftover pipes used as plain separators after stripping.
	out = out.replace("||", "|");
	out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Prefer the plain addon name before a colored warning suffix.
/// e.g. `Questie|cFFFF0000 game client not supported|r` -> `Questie`
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

/// Compatibility warnings from TOC Notes are not useful as addon descriptions.
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn strips_questie_unsupported_title() {
		let raw = "Questie|cFFFF0000 game client not supported|r";
		assert_eq!(clean_toc_title(raw), "Questie");
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
}
