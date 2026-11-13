import type { LocaleCode } from "./messages";

const CATEGORY_ZH: Record<string, string> = {
	"action-bars": "动作条",
	achievements: "成就",
	artwork: "美术",
	"auction-economy": "拍卖与经济",
	"audio-video": "音频与视频",
	"bags-inventory": "背包与物品栏",
	"boss-encounters": "首领战",
	"buffs-debuffs": "增益与减益",
	"chat-communication": "聊天与交流",
	class: "职业",
	combat: "战斗",
	companions: "同伴",
	"data-export": "数据导出",
	"development-tools": "开发工具",
	guild: "公会",
	huds: "HUD",
	"map-minimap": "地图与小地图",
	miscellaneous: "杂项",
	plugins: "插件扩展",
	professions: "专业",
	"quests-leveling": "任务与升级",
	"raid-frames": "团队框体",
	roleplay: "角色扮演",
	tooltips: "鼠标提示",
	"unit-frames": "单位框体",
};

/** CurseForge filter flavors + detected client flavor keys. */
const FLAVOR_ZH: Record<string, string> = {
	retail: "正式服",
	classic: "经典旧世",
	"classic-tbc": "经典燃烧的远征",
	forever: "无限",
	"mop-classic": "熊猫人之谜经典版",
	cataclysm: "大地的裂变经典版",
	wotlk: "巫妖王之怒经典版",
	"classic-beta": "经典版 Beta",
	"classic-ptr": "经典版 PTR",
	ptr: "PTR",
	xptr: "xPTR",
};

const FLAVOR_LABEL_TO_KEY: Record<string, string> = {
	Retail: "retail",
	Classic: "classic",
	"Classic TBC": "classic-tbc",
	Forever: "forever",
	"MoP Classic": "mop-classic",
	"Cataclysm Classic": "cataclysm",
	"WotLK Classic": "wotlk",
	"Classic Beta": "classic-beta",
	"Classic PTR": "classic-ptr",
	PTR: "ptr",
	xPTR: "xptr",
};

export function translateCategoryLabel(
	idOrValue: string,
	fallback: string,
	locale: LocaleCode,
): string {
	if (locale !== "zh-CN") return fallback;
	return CATEGORY_ZH[idOrValue] ?? fallback;
}

export function translateFlavorLabel(
	flavorKeyOrLabel: string,
	fallback: string,
	locale: LocaleCode,
): string {
	if (locale !== "zh-CN") return fallback;
	const key = FLAVOR_LABEL_TO_KEY[flavorKeyOrLabel] ?? flavorKeyOrLabel;
	return FLAVOR_ZH[key] ?? fallback;
}

export function translateInstallLabel(
	flavorKey: string,
	fallbackLabel: string,
	locale: LocaleCode,
): string {
	return translateFlavorLabel(flavorKey, fallbackLabel, locale);
}
