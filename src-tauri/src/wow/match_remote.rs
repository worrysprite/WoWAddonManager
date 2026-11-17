//! Shared helpers for matching a local addon folder/title to a CurseForge summary.

/// Case-insensitive equality, or "Name • subtitle" / "Name - subtitle" style prefix.
pub fn names_loosely_equal(a: &str, b: &str) -> bool {
	let a = a.trim().to_ascii_lowercase();
	let b = b.trim().to_ascii_lowercase();
	if a.is_empty() || b.is_empty() {
		return false;
	}
	if a == b {
		return true;
	}
	starts_with_word_break(&a, &b) || starts_with_word_break(&b, &a)
}

fn starts_with_word_break(full: &str, prefix: &str) -> bool {
	if !full.starts_with(prefix) {
		return false;
	}
	full.get(prefix.len()..)
		.and_then(|rest| rest.chars().next())
		.is_some_and(|c| !c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn matches_decorated_curseforge_title() {
		assert!(names_loosely_equal(
			"Postbox",
			"Postbox • A Modern Mailbox & Mail Manager"
		));
		assert!(names_loosely_equal("PostBox", "postbox"));
		assert!(!names_loosely_equal("Postbox", "postboxmail"));
		assert!(!names_loosely_equal("Post", "Postbox"));
	}
}
