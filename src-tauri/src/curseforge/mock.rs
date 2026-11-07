use crate::types::{AddonSummary, SearchParams, SearchResult};

pub fn search(params: &SearchParams) -> SearchResult {
	let page = params.page.unwrap_or(1).max(1);
	let page_size = params.page_size.unwrap_or(20).clamp(1, 50);
	let search = params
		.search
		.as_deref()
		.unwrap_or("")
		.trim()
		.to_lowercase();
	let categories = params.categories.clone().unwrap_or_default();
	let flavors = params.game_version_type_ids.clone().unwrap_or_default();

	let mut items: Vec<AddonSummary> = demo_addons()
		.into_iter()
		.filter(|item| {
			if search.is_empty() {
				true
			} else {
				item.name.to_lowercase().contains(&search)
					|| item.summary.to_lowercase().contains(&search)
					|| item.author.to_lowercase().contains(&search)
					|| item.slug.to_lowercase().contains(&search)
			}
		})
		.filter(|item| {
			if categories.is_empty() {
				true
			} else {
				categories.iter().any(|c| {
					item.categories
						.iter()
						.any(|ic| ic.eq_ignore_ascii_case(c) || slugify(ic) == *c)
				})
			}
		})
		.filter(|item| {
			if flavors.is_empty() {
				true
			} else {
				flavors.iter().any(|id| {
					let label = crate::curseforge::catalog::flavor_label(*id).unwrap_or("");
					item.flavors.iter().any(|f| f.eq_ignore_ascii_case(label))
				})
			}
		})
		.collect();

	let total = items.len() as u32;
	let start = ((page - 1) * page_size) as usize;
	if start >= items.len() {
		items.clear();
	} else {
		let end = (start + page_size as usize).min(items.len());
		items = items[start..end].to_vec();
	}

	SearchResult {
		items,
		page,
		page_size,
		total,
		source: "mock".to_string(),
		message: None,
	}
}

fn slugify(value: &str) -> String {
	value
		.to_lowercase()
		.chars()
		.map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
		.collect::<String>()
		.split('-')
		.filter(|s| !s.is_empty())
		.collect::<Vec<_>>()
		.join("-")
}

fn demo_addons() -> Vec<AddonSummary> {
	vec![
		addon(
			445235,
			"moveany",
			"MoveAny (Edit Mode, Windows and Ui)",
			"D4KiR",
			"Move windows, unitframes and more. Customize your UI freely.",
			"HUDs",
			&["HUDs", "Action Bars", "Unit Frames"],
			&["Forever", "Retail"],
			22_300_000,
			"2026-11-07",
			"12.1.5",
		),
		addon(
			88372,
			"maxdps-rotation-helper",
			"MaxDps Rotation Helper",
			"Kingofdune",
			"Rotation helper that highlights spells you should cast next.",
			"Combat",
			&["Combat", "Class"],
			&["Forever", "Retail"],
			12_100_000,
			"2026-11-06",
			"12.1.5",
		),
		addon(
			455546,
			"plater-nameplates",
			"Plater Nameplates",
			"Tercioo",
			"Highly customizable nameplate framework for all content.",
			"HUDs",
			&["HUDs", "Combat"],
			&["Retail", "MoP Classic"],
			18_400_000,
			"2026-11-05",
			"12.1.7",
		),
		addon(
			306333,
			"details",
			"Details! Damage Meter",
			"Tercioo",
			"The most complete damage meter for World of Warcraft.",
			"Combat",
			&["Combat", "Data Export"],
			&["Retail", "Forever", "MoP Classic"],
			95_200_000,
			"2026-11-04",
			"12.1.7",
		),
		addon(
			6124,
			"deadly-boss-mods",
			"Deadly Boss Mods",
			"DeadlyBossMods",
			"Boss encounter timers, alerts and strategies.",
			"Boss Encounters",
			&["Boss Encounters"],
			&["Retail", "Classic", "MoP Classic"],
			210_000_000,
			"2026-11-03",
			"12.1.5",
		),
		addon(
			66675,
			"bigwigs",
			"BigWigs Bossmods",
			"Funkydude",
			"Modular boss encounter addons with precise timers.",
			"Boss Encounters",
			&["Boss Encounters"],
			&["Retail", "Forever"],
			48_700_000,
			"2026-11-02",
			"12.1.5",
		),
		addon(
			4383,
			"weak-auras-2",
			"WeakAuras 2",
			"Stanzilla",
			"A powerful framework for customizable status displays.",
			"Buffs & Debuffs",
			&["Buffs & Debuffs", "Combat"],
			&["Retail", "Classic", "Forever"],
			160_000_000,
			"2026-11-01",
			"12.1.7",
		),
		addon(
			269263,
			"all-the-things",
			"All The Things",
			"DagoCodes",
			"Track collections, appearances, mounts and more.",
			"Achievements",
			&["Achievements", "Miscellaneous"],
			&["Retail"],
			28_900_000,
			"2026-10-30",
			"12.1.5",
		),
		addon(
			33440,
			"auctionator",
			"Auctionator",
			"Borjamacare",
			"Simplifies buying and selling at the auction house.",
			"Auction & Economy",
			&["Auction & Economy"],
			&["Retail", "Classic", "MoP Classic"],
			72_500_000,
			"2026-10-28",
			"12.1.5",
		),
		addon(
			50461,
			"handynotes",
			"HandyNotes",
			"Nevcairiel",
			"Place and share notes on your world map and minimap.",
			"Map & Minimap",
			&["Map & Minimap"],
			&["Retail", "Classic"],
			35_100_000,
			"2026-10-26",
			"12.1.5",
		),
		addon(
			35833,
			"bartender4",
			"Bartender4",
			"Nevcairiel",
			"Action bar addon with extensive customization options.",
			"Action Bars",
			&["Action Bars"],
			&["Retail", "Forever", "MoP Classic"],
			88_300_000,
			"2026-10-24",
			"12.1.7",
		),
		addon(
			4705,
			"omnicc",
			"OmniCC",
			"Tuller",
			"Adds cooldown text to action buttons and items.",
			"Action Bars",
			&["Action Bars", "Combat"],
			&["Retail", "Classic"],
			61_200_000,
			"2026-10-22",
			"12.1.5",
		),
		addon(
			111814,
			"simulationcraft",
			"SimulationCraft",
			"navv",
			"Export your character for SimulationCraft analysis.",
			"Data Export",
			&["Data Export", "Combat"],
			&["Retail"],
			19_800_000,
			"2026-10-20",
			"12.1.5",
		),
		addon(
			94845,
			"raiderio",
			"Raider.IO Mythic Plus and Raid Progress",
			"AspyrBox",
			"Show Mythic+ and raid progress in tooltips and LFG.",
			"Tooltips",
			&["Tooltips", "Guild"],
			&["Retail"],
			41_600_000,
			"2026-10-18",
			"12.1.7",
		),
		addon(
			20430,
			"tomtom",
			"TomTom",
			"Ludovicus",
			"Coordinates, waypoints and crazy-arrow navigation.",
			"Map & Minimap",
			&["Map & Minimap"],
			&["Retail", "Classic", "MoP Classic"],
			55_400_000,
			"2026-10-16",
			"12.1.5",
		),
		addon(
			23422,
			"dominos",
			"Dominos",
			"Tuller",
			"A main actionbar replacement with movable bars.",
			"Action Bars",
			&["Action Bars"],
			&["Retail", "Classic"],
			47_900_000,
			"2026-10-14",
			"12.1.5",
		),
		addon(
			13824,
			"elvui",
			"ElvUI",
			"Elv",
			"A complete replacement for the standard World of Warcraft interface.",
			"HUDs",
			&["HUDs", "Unit Frames", "Action Bars"],
			&["Retail", "Classic", "MoP Classic"],
			120_000_000,
			"2026-10-12",
			"12.1.7",
		),
		addon(
			63659,
			"premade-groups-filter",
			"Premade Groups Filter",
			"Maldivia",
			"Filter premade groups by score, roles and more.",
			"Guild",
			&["Guild", "Miscellaneous"],
			&["Retail"],
			22_700_000,
			"2026-10-10",
			"12.1.5",
		),
		addon(
			45580,
			"immersion",
			"Immersion",
			"MunkDev",
			"Immersive questing with cinematic dialogue UI.",
			"Quests & Leveling",
			&["Quests & Leveling", "Roleplay"],
			&["Retail", "Classic"],
			16_300_000,
			"2026-10-08",
			"12.1.5",
		),
		addon(
			32274,
			"threat-classic2",
			"ThreatClassic2",
			"DennisWG",
			"Simple threat meter designed for classic flavors.",
			"Combat",
			&["Combat"],
			&["Classic", "MoP Classic", "Classic TBC"],
			9_400_000,
			"2026-10-06",
			"5.5.3",
		),
	]
}

fn addon(
	id: i64,
	slug: &str,
	name: &str,
	author: &str,
	summary: &str,
	primary: &str,
	categories: &[&str],
	flavors: &[&str],
	download_count: i64,
	updated_at: &str,
	latest_game_version: &str,
) -> AddonSummary {
	AddonSummary {
		id,
		slug: slug.to_string(),
		name: name.to_string(),
		author: author.to_string(),
		summary: summary.to_string(),
		thumbnail_url: format!(
			"https://media.forgecdn.net/avatars/thumbnails/0/0/256/256/{}.png",
			id
		),
		download_count,
		updated_at: updated_at.to_string(),
		latest_game_version: latest_game_version.to_string(),
		categories: {
			let mut cats = vec![primary.to_string()];
			for c in categories {
				if !cats.iter().any(|x| x == c) {
					cats.push((*c).to_string());
				}
			}
			cats
		},
		flavors: flavors.iter().map(|s| (*s).to_string()).collect(),
		website_url: format!("https://www.curseforge.com/wow/addons/{}", slug),
		latest_file_id: Some(id * 10 + 1),
	}
}
