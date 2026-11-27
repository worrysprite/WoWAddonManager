# WoW Addon Manager

基于 **Tauri 2 + React + TypeScript** 的魔兽世界插件管理器（当前版本 **1.9.1**）。

预编译安装包 / 便携版与发版说明见 [`releases/README.md`](releases/README.md)；最新包：https://github.com/worrysprite/WoWAddonManager/releases/latest

## 功能

### 浏览与安装

- 浏览页：仿 CurseForge 深色布局（左侧分类 / 版本过滤、右侧列表、顶部搜索、底部分页）
- 按已检测到的魔兽客户端分标签页管理；Forever 客户端中文显示为「无限」
- 安装 / 更新按钮按插件支持的本地客户端显示；进行中显示下载→解压进度
- 点击卡片可打开应用内详情弹窗（API 截图 + 介绍，非嵌入网页）；介绍中的链接在系统浏览器打开
- 作者禁用 API 分发导致下载失败时，弹出说明对话框（可打开 CurseForge）；「全部更新」时会逐条追加多个插件

### 已安装：插件 / 配置管理

每个客户端的已安装页有两个页签：

- **插件**：路径显示到该客户端的 `Interface`；扫描、手动安装、更新 / 全部更新、备份 / 恢复（仅 `Interface`）、打开插件目录
- **配置管理**：路径显示到同客户端的 `WTF`；「{版本} 角色列表」标题（Forever = 「无限」）；备份 / 恢复（仅 `WTF`）；列出账号与角色，支持删除与复制配置

扫描 `Interface/AddOns` 后，磁盘结果立即出列表；CurseForge 更新检查与图标补全在后台进行。启动约 3 秒后检查一次已安装更新，之后每小时自动检查（需已配置 API Key）。

- **套件**：列表只显示父插件；安装 / 更新 / 删除作用于整个套件；优先以带 `X-Curse-Project-ID` 的文件夹为合集父项
- **全部更新**：最多 5 路并发，按钮显示剩余数量，各插件独立进度
- **手动安装**：可一次多选压缩包，按顺序安装；成功提示显示合集 / CurseForge 包名（「已安装：…」），已是最新时不显示 Update
- **备份 / 恢复**：插件页只同步 `Interface`，配置管理页只同步 `WTF`（内置镜像同步，弹窗显示进度）。设置里为每个客户端指定的备份路径是**父目录**（其下直接放 `Interface`、`WTF`）。未设置时点击备份会跳转设置
- **配置管理**：可复制账号 / 角色配置（对话框勾选类别，默认勾选 SavedVariables；含按键绑定等）；扫描账号时排除顶层 `SavedVariables`，避免误列为账号

### 客户端、设置与界面

- 自动扫描魔兽安装目录（注册表 / Battle.net Agent / 常见盘符），也可在设置中手动添加
- **CurseForge API Key** 仅在设置中由用户填写，**不会**打进安装包；有 Key 时走官方 API，无 Key 时回退网页 GET
- 设置页全宽双列布局（窄屏单列）；切换语言后成功提示使用目标语言
- 简体中文 / English（可跟随系统）；筛选菜单与客户端标签等已本地化
- 全应用禁用 WebView 右键菜单（搜索框等可编辑输入除外，以便剪切 / 粘贴）
- 安装版支持应用内自动更新（静默下载，重启安装）；便携版提示并打开 Releases 下载页

## CurseForge API 申请

未配置 Key 时暂用网页搜索 GET：

`https://www.curseforge.com/wow/search?class=addons&...`

该站点有 Cloudflare 防护，失败时会提示错误。下载也会尝试网页直链。正式接入请申请官方 API Key：

- 第三方申请表：https://forms.monday.com/forms/dce5ccb7afda9a1c21dab1a1aa1d84eb?r=use1
- API 文档：https://docs.curseforge.com/rest-api/
- Studios Console（游戏开发者）：https://console.curseforge.com/

> 注意：非游戏开发者请走第三方申请表。未申请 Key 就注册 Console 的账号可能会被删除。

拿到 Key 后，在应用右上角 **设置** 中填写即可。保存后搜索与下载会自动改用官方 API（`api.curseforge.com` + `x-api-key`）。

## 开发

```bash
npm install
npm run tauri dev
```

构建：

```bash
npm run tauri build
```

发版：推送 `v*` 标签触发 [`.github/workflows/release.yml`](.github/workflows/release.yml)，生成 NSIS 安装包、updater 元数据与便携 exe。细节见 [`releases/README.md`](releases/README.md)。

## 目录说明

- `src/` 前端界面（浏览 / 已安装 / 设置、i18n、详情弹窗等）
- `src-tauri/src/curseforge/` 搜索、详情与下载（网页 GET / 官方 API）
- `src-tauri/src/wow/` 客户端检测、插件扫描与套件折叠、安装/更新/删除、备份/恢复、WTF 配置管理
- `releases/` Windows 预编译说明与版本更新记录
