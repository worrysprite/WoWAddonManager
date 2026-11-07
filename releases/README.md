# Windows 预编译包

当前版本：**0.2.1**

- `WoW-Addon-Manager_0.2.1_x64-setup.exe`：安装包（推荐）
- `WoW-Addon-Manager.exe`：免安装便携版

首次运行若被 SmartScreen 拦截，选择“仍要运行”。需已安装 WebView2（Win10/11 通常自带）。

下载：

- 安装包：https://github.com/worrysprite/WoWAddonManager/raw/main/releases/WoW-Addon-Manager_0.2.1_x64-setup.exe
- 便携版：https://github.com/worrysprite/WoWAddonManager/raw/main/releases/WoW-Addon-Manager.exe

## 0.2.1 更新
- 修复启动时主线程扫盘卡死约一分钟
- 已安装插件列表缓存到 JSON，切换标签不再全量扫描
- 浏览页搜索结果本地缓存，减少重复 API 请求
