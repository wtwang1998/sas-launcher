# SAS Launcher

多个 SAS 项目往往需要不同的会话环境（编码、区域设置、界面语言、个人偏好），同时打开又会因共用用户设置而互相读写冲突。SAS Launcher 为每个项目准备独立的会话环境，让它们可以同时运行、互不干扰。

![平台](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%20x64-0078D4)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB)
![许可证](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)

> AI 编码 agent 请先读 [AGENTS.md](AGENTS.md)。

## 功能

- **三种配置来源**：跟随默认配置（SAS 安装的默认语言环境）、内置配置（`nls` 下 23 个出厂语言配置，如「简体中文 · UTF-8」）、用户配置（从任一处衍生，可改编码/区域）。
- **用户配置 = 薄壳 cfg + 个人设置快照**：配置文件只有 3 行（指向出厂 cfg + 编码 + 区域），SAS 升级后自动继承修复；个人设置以 `profile.sas7bcat`、`profile2.sas7bcat`、`regstry.sas7bitm`、`templat.sas7bitm` 快照保存。
- **两种修改方式**：表单修改改编码/区域；「打开 SAS 调整设置」用一个专用会话改界面偏好（Preferences、Results 选项卡、Explorer 配置等），退出后保存回该配置。
- **每个 SAS 窗口一份独立、可写的用户设置目录**（`-SASUSER`），退出后自动回收，多项目并行时互不覆盖。
- **会话诊断**：解析链（薄壳 cfg → 出厂 cfg → 生效的编码/区域/SASUSER）、真实启动参数、失败阶段、退出码，以及 `-ALTLOG` 抓到的 SAS 日志副本（Log 窗口不受影响），保留最近 20 条。
- 常用项目可批量启动，已在运行的项目自动跳过。

## 开发

依赖 Rust MSVC 工具链、Visual Studio C++ Build Tools、Node.js 22.12+ 和 WebView2。

```powershell
winget install --id Rustlang.Rustup
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

git clone https://github.com/wtwang1998/sas-launcher.git
cd sas-launcher
npm ci
npm run tauri dev
```

首次编译 Rust 依赖约 3–10 分钟。改 `src/**` 走 Vite HMR；改 `src-tauri/**` 会自动重新编译并重启窗口。

| 命令 | 用途 |
| --- | --- |
| `npm run build` | 类型检查 + 前端构建 |
| `npm run test` | 前端测试（vitest + Testing Library） |
| `cargo test --manifest-path src-tauri/Cargo.toml` | 后端单元测试 |
| `npm run package:windows` | 打安装包，输出到 `src-tauri/target/release/bundle/windows/` |

## 使用

1. 首次启动自动查找已安装的 SAS 9.4；找不到时在横幅里手动选择 `sas.exe`。
2. 在「配置管理」查看 23 个出厂配置，或用「从本配置衍生」建一个用户配置再改编码/区域。
3. 在「项目」新建项目：填名称、三选一配置来源、备注，然后保存或直接启动。
4. 需要调 SAS 界面偏好时，在配置行菜单里点「打开 SAS 调整设置」，改完退出 SAS 再保存。
5. 启动不顺利时去「会话诊断」看解析链、启动参数与日志。

## 已知限制

- SAS 的**增强编辑器**字体/颜色/按键保存在 Windows 注册表（`HKCU`），SAS 没有改变其位置的选项，因此这部分**不能**按配置隔离或保存。
- v0.2 不迁移旧版本数据：旧目录会被归档到 `<应用数据目录>\_old-<日期>`，然后从干净状态开始。

## 安装

要求 Windows 10/11 x64，且本机已安装 SAS Foundation 9.4。从 [Releases](https://github.com/wtwang1998/sas-launcher/releases) 下载 `SAS-Launcher-0.2.0-x64-Setup.exe`；首次安装默认仅为当前用户装到 `%LOCALAPPDATA%\SAS Launcher`，升级与卸载沿用该范围。应用数据在同目录的 `com.saslauncher.desktop`。

## 许可证

以 [GPL-3.0-or-later](LICENSE) 发布。
