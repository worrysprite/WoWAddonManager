# Windows 预编译包

当前版本：**0.4.2**

- `WoW-Addon-Manager_0.4.2_x64-setup.exe`：安装包（推荐，支持应用内自动更新）
- `WoW-Addon-Manager.exe`：免安装便携版（需手动下载替换）

首次运行若被 SmartScreen 拦截，选择“仍要运行”。需已安装 WebView2（Win10/11 通常自带）。

## 自动更新

- **安装包**：启动后会静默检查 GitHub Releases；有新版本时后台下载，提示「重启以安装」。
- **便携版**：仅提示新版本并打开 [Releases](https://github.com/worrysprite/WoWAddonManager/releases/latest) 下载页。
- 发版：推送 `v*` 标签触发 `.github/workflows/release.yml`，生成 NSIS 安装包、签名与 `latest.json`。
- 仓库 Secrets 需配置 `TAURI_SIGNING_PRIVATE_KEY`（可选 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`）。公钥已写入 `src-tauri/tauri.conf.json`。

下载：

- 安装包：https://github.com/worrysprite/WoWAddonManager/raw/main/releases/WoW-Addon-Manager_0.4.2_x64-setup.exe
- 便携版：https://github.com/worrysprite/WoWAddonManager/raw/main/releases/WoW-Addon-Manager.exe
- 最新 Release：https://github.com/worrysprite/WoWAddonManager/releases/latest

## 0.4.2 更新
- 日志从启动起始终按 INFO 记录（设置页仅查看）；级别可持久化
- CurseForge API 搜索补充诊断日志；空结果时自动重试；Key 粘贴杂质清理
- 备份/恢复弹窗不再显示「引擎：mirror」

## 0.4.1 更新
- 备份/恢复改为内置本地镜像同步，不再调用 Cygwin/系统 rsync（修复 Windows 盘符路径报错）
- 未变更文件跳过复制；浏览页移除演示数据回退

## 0.4.0 更新
- 备份 / 恢复不再闪出终端窗口（Windows 无控制台后台同步）
- 弹窗实时流式显示 rsync / 内置镜像同步进度
- 恢复成功后若列表扫描失败，不再掩盖已成功的恢复结果

## 0.3.0 更新
- 每个客户端可在设置「已检测到的客户端」中单独配置备份目录
- 手动安装合集先解压到暂存区，避免覆盖/删除已有同名插件
- Forever Camelot TOC / Interface 16001 合集兼容（已合入）

## 0.2.6 更新
- 客户端页支持备份 / 恢复 Interface 与 WTF（rsync 镜像同步；无 rsync 时用同等逻辑）
- 设置页可配备份路径；未设置时点击备份会跳转到该设置

## 0.2.5 更新
- 修复 Forever 插件（Interface 16001）被 Retail 12.x 版本号误拒（PrettyTooltip / ForeverForge 等）
- 已安装列表的更新/删除按钮移到卡片右侧

## 0.2.4 更新
- 修复 Questie 等多 TOC 插件误判不支持：按客户端选择 `*_Forever.toc` 等专用 TOC
- 浏览页增加 CurseForge 版权提示（可点开官网）
- 浏览页铺满窗口宽度；安装按钮仅显示插件支持的本地客户端

## 0.2.3 更新
- TOC 颜色码清理；不兼容客户端安装失败且不出现在列表
- 删除确认改用原生对话框，避免确认框弹出时列表空白
- 移除顶栏重复扫描；版本页可打开 Interface 目录
- 改语言不再误清空 API Key

## 0.2.2 更新
- 修复切换魔兽版本时插件列表串台 / 残留
- 设置页可查看、调整级别并保存运行日志（默认 WARN）
- 增加多语言（简体中文 / English），默认跟随系统语言
