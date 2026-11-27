//! Compare local TOC / manifest version labels with CurseForge file display names.
//!
//! Manual installs have no `file_id`, so update detection must fall back to these
//! human version strings (e.g. TOC `1.4.0` vs CF `PostBox 1.4.0`).

use std::cmp::Ordering;

/// True when `raw` looks like an ISO-8601 timestamp (not a version label).
pub fn looks_like_iso_timestamp(raw: &str) -> bool {
	let s = raw.trim();
	if s.len() < 10 {
		return false;
	}
	let bytes = s.as_bytes();
	// YYYY-MM-DD…
	bytes[0].is_ascii_digit()
		&& bytes[1].is_ascii_digit()
		&& bytes[2].is_ascii_digit()
		&& bytes[3].is_ascii_digit()
		&& bytes[4] == b'-'
		&& bytes[5].is_ascii_digit()
		&& bytes[6].is_ascii_digit()
		&& bytes[7] == b'-'
		&& bytes[8].is_ascii_digit()
		&& bytes[9].is_ascii_digit()
}

fn strip_archive_ext(name: &str) -> &str {
	let lower = name.to_ascii_lowercase();
	for ext in [".tar.gz", ".tgz", ".zip", ".rar", ".7z"] {
		if lower.ends_with(ext) {
			return &name[..name.len() - ext.len()];
		}
	}
	name
}

/// Pull dotted numeric parts from labels like `1.4.0`, `v12.6.2`, `PostBox 1.4.0`.
pub fn extract_version_parts(raw: &str) -> Option<Vec<u64>> {
	let s = strip_archive_ext(raw.trim());
	if s.is_empty() || looks_like_iso_timestamp(s) {
		return None;
	}

	let mut best: Option<Vec<u64>> = None;
	let bytes = s.as_bytes();
	let mut i = 0;
	while i < bytes.len() {
		if !bytes[i].is_ascii_digit() {
			i += 1;
			continue;
		}
		let start = i;
		while i < bytes.len() && bytes[i].is_ascii_digit() {
			i += 1;
		}
		let mut parts = Vec::new();
		if let Ok(n) = s[start..i].parse::<u64>() {
			parts.push(n);
		} else {
			continue;
		}
		while i < bytes.len() && bytes[i] == b'.' {
			let dot = i;
			i += 1;
			let seg_start = i;
			while i < bytes.len() && bytes[i].is_ascii_digit() {
				i += 1;
			}
			if seg_start == i {
				i = dot;
				break;
			}
			if let Ok(n) = s[seg_start..i].parse::<u64>() {
				parts.push(n);
			} else {
				i = dot;
				break;
			}
		}
		let better = match &best {
			None => true,
			Some(prev) => parts.len() > prev.len()
				|| (parts.len() == prev.len() && parts > *prev),
		};
		if better {
			best = Some(parts);
		}
	}
	best
}

fn cmp_parts(a: &[u64], b: &[u64]) -> Ordering {
	let n = a.len().max(b.len());
	for i in 0..n {
		let left = a.get(i).copied().unwrap_or(0);
		let right = b.get(i).copied().unwrap_or(0);
		match left.cmp(&right) {
			Ordering::Equal => {}
			other => return other,
		}
	}
	Ordering::Equal
}

/// Compare two version labels. `None` when either side has no extractable version.
pub fn compare_addon_versions(local: &str, remote: &str) -> Option<Ordering> {
	let a = extract_version_parts(local)?;
	let b = extract_version_parts(remote)?;
	Some(cmp_parts(&a, &b))
}

/// `Some(true)` when local covers remote (equal or newer); `Some(false)` when older;
/// `None` when versions cannot be compared.
pub fn local_covers_remote(local: Option<&str>, remote: Option<&str>) -> Option<bool> {
	let local = local.map(str::trim).filter(|s| !s.is_empty())?;
	let remote = remote.map(str::trim).filter(|s| !s.is_empty())?;
	match compare_addon_versions(local, remote)? {
		Ordering::Less => Some(false),
		Ordering::Equal | Ordering::Greater => Some(true),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn extracts_plain_and_prefixed() {
		assert_eq!(extract_version_parts("1.4.0"), Some(vec![1, 4, 0]));
		assert_eq!(extract_version_parts("v12.6.2"), Some(vec![12, 6, 2]));
		assert_eq!(
			extract_version_parts("PostBox 1.4.0"),
			Some(vec![1, 4, 0])
		);
		assert_eq!(
			extract_version_parts("PostBox-1.4.0.zip"),
			Some(vec![1, 4, 0])
		);
		assert_eq!(extract_version_parts("r837"), Some(vec![837]));
		assert_eq!(
			extract_version_parts("DBM-Vanilla_SoD-r837.zip"),
			Some(vec![837])
		);
	}

	#[test]
	fn dbm_revision_covers_cf_display_name() {
		assert_eq!(
			local_covers_remote(Some("r837"), Some("DBM-Vanilla_SoD-r837")),
			Some(true)
		);
		assert_eq!(
			local_covers_remote(
				Some("DBM-Vanilla_SoD-r837.zip"),
				Some("DBM-Vanilla_SoD-r837")
			),
			Some(true)
		);
		assert_eq!(
			local_covers_remote(Some("r836"), Some("DBM-Vanilla_SoD-r837")),
			Some(false)
		);
	}

	#[test]
	fn rejects_iso_timestamps() {
		assert!(extract_version_parts("2026-11-19T15:16:18.08Z").is_none());
		assert!(looks_like_iso_timestamp("2026-11-19T15:16:18.08Z"));
	}

	#[test]
	fn local_covers_when_equal_or_newer() {
		assert_eq!(
			local_covers_remote(Some("1.4.0"), Some("PostBox 1.4.0")),
			Some(true)
		);
		assert_eq!(
			local_covers_remote(Some("1.5.0"), Some("PostBox 1.4.0")),
			Some(true)
		);
		assert_eq!(
			local_covers_remote(Some("1.3.0"), Some("PostBox 1.4.0")),
			Some(false)
		);
		assert_eq!(local_covers_remote(Some("1.4.0"), None), None);
		assert_eq!(local_covers_remote(None, Some("1.4.0")), None);
	}

	#[test]
	fn pads_shorter_version() {
		assert_eq!(
			compare_addon_versions("1.4", "1.4.0"),
			Some(Ordering::Equal)
		);
		assert_eq!(
			compare_addon_versions("1.4.1", "1.4"),
			Some(Ordering::Greater)
		);
	}
}
