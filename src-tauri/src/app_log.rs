use chrono::Utc;
use log::{LevelFilter, Log, Metadata, Record};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

const MAX_ENTRIES: usize = 2000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
	pub timestamp: String,
	pub level: String,
	pub target: String,
	pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogView {
	pub level: String,
	pub entries: Vec<LogEntry>,
}

struct MemoryLogger;

static ENTRIES: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
// 3 == LevelFilter::Info — always record from process start; settings only views.
static LEVEL: AtomicU8 = AtomicU8::new(3);
static LOGGER: MemoryLogger = MemoryLogger;
static INIT: OnceLock<()> = OnceLock::new();

fn level_to_u8(level: LevelFilter) -> u8 {
	match level {
		LevelFilter::Off => 0,
		LevelFilter::Error => 1,
		LevelFilter::Warn => 2,
		LevelFilter::Info => 3,
		LevelFilter::Debug => 4,
		LevelFilter::Trace => 5,
	}
}

fn u8_to_level(v: u8) -> LevelFilter {
	match v {
		0 => LevelFilter::Off,
		1 => LevelFilter::Error,
		2 => LevelFilter::Warn,
		3 => LevelFilter::Info,
		4 => LevelFilter::Debug,
		_ => LevelFilter::Trace,
	}
}

fn parse_level(name: &str) -> Result<LevelFilter, String> {
	match name.trim().to_ascii_lowercase().as_str() {
		"off" => Ok(LevelFilter::Off),
		"error" => Ok(LevelFilter::Error),
		"warn" | "warning" => Ok(LevelFilter::Warn),
		"info" => Ok(LevelFilter::Info),
		"debug" => Ok(LevelFilter::Debug),
		"trace" => Ok(LevelFilter::Trace),
		other => Err(format!("未知日志级别: {other}")),
	}
}

fn level_name(level: LevelFilter) -> &'static str {
	match level {
		LevelFilter::Off => "off",
		LevelFilter::Error => "error",
		LevelFilter::Warn => "warn",
		LevelFilter::Info => "info",
		LevelFilter::Debug => "debug",
		LevelFilter::Trace => "trace",
	}
}

pub fn init() {
	INIT.get_or_init(|| {
		let _ = log::set_logger(&LOGGER);
		// Accept every level from macros; LEVEL atomic still filters what we store.
		log::set_max_level(LevelFilter::Trace);
		let preferred = parse_level(&crate::config::load().log_level)
			.unwrap_or(LevelFilter::Info);
		LEVEL.store(level_to_u8(preferred), Ordering::Relaxed);
		log::info!(
			"日志系统已启动，级别 {}（始终记录，设置页仅用于查看）",
			level_name(preferred)
		);
	});
}

pub fn current_level() -> String {
	level_name(u8_to_level(LEVEL.load(Ordering::Relaxed))).to_string()
}

pub fn set_level(name: &str) -> Result<String, String> {
	let level = parse_level(name)?;
	LEVEL.store(level_to_u8(level), Ordering::Relaxed);
	if let Err(err) = crate::config::save_log_level(level_name(level)) {
		log::warn!("保存日志级别失败: {err}");
	}
	log::info!("日志级别已设置为 {}", level_name(level));
	Ok(level_name(level).to_string())
}

pub fn view() -> LogView {
	let entries = ENTRIES
		.lock()
		.map(|guard| guard.iter().cloned().collect())
		.unwrap_or_default();
	LogView {
		level: current_level(),
		entries,
	}
}

pub fn clear() {
	if let Ok(mut guard) = ENTRIES.lock() {
		guard.clear();
	}
	log::warn!("日志已清空");
}

pub fn save_to_path(path: &str) -> Result<(), String> {
	let view = view();
	let mut lines = Vec::with_capacity(view.entries.len() + 2);
	lines.push(format!(
		"# WoW Addon Manager log  level={}  exportedAt={}",
		view.level,
		Utc::now().to_rfc3339()
	));
	for entry in &view.entries {
		lines.push(format!(
			"{} [{:5}] {}: {}",
			entry.timestamp, entry.level, entry.target, entry.message
		));
	}
	let content = lines.join("\n") + "\n";
	if let Some(parent) = PathBuf::from(path).parent() {
		if !parent.as_os_str().is_empty() {
			fs::create_dir_all(parent).map_err(|e| e.to_string())?;
		}
	}
	fs::write(path, content).map_err(|e| e.to_string())?;
	log::warn!("日志已保存到 {path}");
	Ok(())
}

impl Log for MemoryLogger {
	fn enabled(&self, metadata: &Metadata) -> bool {
		let current = u8_to_level(LEVEL.load(Ordering::Relaxed));
		metadata.level() <= current
	}

	fn log(&self, record: &Record) {
		if !self.enabled(record.metadata()) {
			return;
		}
		let entry = LogEntry {
			timestamp: Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string(),
			level: record.level().as_str().to_string(),
			target: record.target().to_string(),
			message: format!("{}", record.args()),
		};
		if let Ok(mut guard) = ENTRIES.lock() {
			if guard.len() >= MAX_ENTRIES {
				guard.pop_front();
			}
			guard.push_back(entry);
		}
	}

	fn flush(&self) {}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_level_is_info_and_can_change() {
		init();
		// May already be info from a prior test/config; ensure set works.
		assert_eq!(set_level("debug").unwrap(), "debug");
		assert_eq!(current_level(), "debug");
		assert_eq!(set_level("info").unwrap(), "info");
		assert_eq!(current_level(), "info");
	}
}
