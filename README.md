# WoW Addon Manager

基于 **Tauri 2 + React + TypeScript** 的魔兽世界插件管理器。

## 功能

- 浏览页：仿 CurseForge 深色布局（左侧过滤器、右侧列表、顶部搜索、底部分页）
- 按已检测到的魔兽客户端分标签页管理插件
- 自动扫描魔兽安装目录（注册表 / Battle.net Agent / 常见盘符），也可在设置中手动添加
- 扫描 `Interface/AddOns`，在目录内维护 `wow-addon-manager.json`
- 已安装卡片显示安装日期，支持删除；仅检测到新版本时显示更新

## CurseForge API 申请

当前无官方 Key，暂用网页搜索 GET：

`https://www.curseforge.com/wow/search?class=addons&...`

该站点有 Cloudflare 防护，失败时会自动回退演示数据。下载也会尝试网页直链，失败则写入占位插件目录便于联调。

正式接入请申请官方 API Key：

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

## 目录说明

- `src/` 前端界面
- `src-tauri/src/curseforge/` 搜索与下载（网页 GET + mock）
- `src-tauri/src/wow/` 客户端检测、插件扫描、安装/更新/删除
