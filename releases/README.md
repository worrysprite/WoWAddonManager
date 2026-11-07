# Windows 预编译包

当前版本：**0.2.5**

- `WoW-Addon-Manager_0.2.5_x64-setup.exe`：安装包（推荐）
- `WoW-Addon-Manager.exe`：免安装便携版

首次运行若被 SmartScreen 拦截，选择“仍要运行”。需已安装 WebView2（Win10/11 通常自带）。

下载：

- 安装包：https://github.com/worrysprite/WoWAddonManager/raw/main/releases/WoW-Addon-Manager_0.2.5_x64-setup.exe
- 便携版：https://github.com/worrysprite/WoWAddonManager/raw/main/releases/WoW-Addon-Manager.exe

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
