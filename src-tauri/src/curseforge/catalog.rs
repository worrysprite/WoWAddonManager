use crate::types::{FilterCatalog, FilterOption, FlavorOption};

pub fn filter_catalog() -> FilterCatalog {
	FilterCatalog {
		categories: vec![
			opt("action-bars", "Action Bars"),
			opt("achievements", "Achievements"),
			opt("artwork", "Artwork"),
			opt("auction-economy", "Auction & Economy"),
			opt("audio-video", "Audio & Video"),
			opt("bags-inventory", "Bags & Inventory"),
			opt("boss-encounters", "Boss Encounters"),
			opt("buffs-debuffs", "Buffs & Debuffs"),
			opt("chat-communication", "Chat & Communication"),
			opt("class", "Class"),
			opt("combat", "Combat"),
			opt("companions", "Companions"),
			opt("data-export", "Data Export"),
			opt("development-tools", "Development Tools"),
			opt("guild", "Guild"),
			opt("huds", "HUDs"),
			opt("map-minimap", "Map & Minimap"),
			opt("miscellaneous", "Miscellaneous"),
			opt("plugins", "Plugins"),
			opt("professions", "Professions"),
			opt("quests-leveling", "Quests & Leveling"),
			opt("raid-frames", "Raid Frames"),
			opt("roleplay", "Roleplay"),
			opt("tooltips", "Tooltips"),
			opt("unit-frames", "Unit Frames"),
		],
		flavors: vec![
			flavor("retail", "Retail", 517),
			flavor("classic", "Classic", 67408),
			flavor("classic-tbc", "Classic TBC", 73246),
			flavor("forever", "Forever", 88568),
			flavor("mop-classic", "MoP Classic", 79434),
			flavor("cataclysm", "Cataclysm Classic", 77522),
			flavor("wotlk", "WotLK Classic", 73713),
		],
		game_versions: vec![
			opt("12.1.7", "12.1.7"),
			opt("12.1.5", "12.1.5"),
			opt("12.0.5", "12.0.5"),
			opt("11.2.7", "11.2.7"),
			opt("5.5.3", "5.5.3"),
			opt("1.15.8", "1.15.8"),
		],
	}
}

fn opt(id: &str, label: &str) -> FilterOption {
	FilterOption {
		id: id.to_string(),
		label: label.to_string(),
		value: id.to_string(),
	}
}

fn flavor(id: &str, label: &str, game_version_type_id: i64) -> FlavorOption {
	FlavorOption {
		id: id.to_string(),
		label: label.to_string(),
		game_version_type_id,
	}
}

pub fn flavor_label(game_version_type_id: i64) -> Option<&'static str> {
	match game_version_type_id {
		517 => Some("Retail"),
		67408 => Some("Classic"),
		73246 => Some("Classic TBC"),
		73713 => Some("WotLK Classic"),
		77522 => Some("Cataclysm Classic"),
		79434 => Some("MoP Classic"),
		88568 => Some("Forever"),
		_ => None,
	}
}

/// CurseForge WoW addon category ids (classId 5 / CF2WowAddonCategory).
pub fn category_id_for_slug(slug: &str) -> Option<i64> {
	match slug.trim().to_ascii_lowercase().as_str() {
		"action-bars" => Some(1018),
		"achievements" => Some(1067),
		"artwork" => Some(1006),
		"auction-economy" => Some(1002),
		"audio-video" => Some(1003),
		"bags-inventory" => Some(1009),
		"boss-encounters" => Some(1014),
		"buffs-debuffs" => Some(1005),
		"chat-communication" => Some(1001),
		"class" => Some(1020),
		"combat" => Some(1019),
		"companions" => Some(1085),
		"data-export" => Some(1007),
		"development-tools" => Some(1031),
		"guild" => Some(1008),
		"huds" => Some(1039),
		"map-minimap" => Some(1011),
		"miscellaneous" => Some(1017),
		"plugins" => Some(1063),
		"professions" => Some(1015),
		"quests-leveling" => Some(1013),
		"raid-frames" => Some(1037),
		"roleplay" => Some(1060),
		"tooltips" => Some(1055),
		"unit-frames" => Some(1016),
		_ => None,
	}
}

pub fn resolve_category_ids(categories: &[String]) -> Vec<i64> {
	let mut ids = Vec::new();
	for slug in categories {
		if let Some(id) = category_id_for_slug(slug) {
			if !ids.contains(&id) {
				ids.push(id);
			}
		}
	}
	ids
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn maps_known_category_slugs() {
		assert_eq!(category_id_for_slug("action-bars"), Some(1018));
		assert_eq!(category_id_for_slug("Combat"), Some(1019));
		assert_eq!(category_id_for_slug("unknown"), None);
	}

	#[test]
	fn resolve_dedupes_ids() {
		let ids = resolve_category_ids(&[
			"action-bars".into(),
			"Action-Bars".into(),
			"combat".into(),
		]);
		assert_eq!(ids, vec![1018, 1019]);
	}
}
