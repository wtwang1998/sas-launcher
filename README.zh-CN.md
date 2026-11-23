# SAS Launcher

在同一台机器上并行运行多个 SAS 9.4 项目，每个项目一套独立的会话环境。

![平台](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%20x64-0078D4)
![最新版本](https://img.shields.io/github/v/release/wtwang1998/sas-launcher)
![下载量](https://img.shields.io/github/downloads/wtwang1998/sas-launcher/total)
![许可证](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)
[![CI](https://github.com/wtwang1998/sas-launcher/actions/workflows/ci.yml/badge.svg)](https://github.com/wtwang1998/sas-launcher/actions/workflows/ci.yml)

[English](./README.md) | **简体中文**

## 为什么

SAS 会话把偏好存在同一份用户设置（`SASUSER`）里。开第二个项目时，它继承了上一个项目的状态——同样的编码、同样的 Preferences、同样的逻辑库定义——而后关闭的窗口会把先关闭的覆盖掉。

SAS Launcher 给每个项目一份自己的配置，并为每次启动准备一个独立、可写的 profile 目录，会话退出即删除。多个项目因此可以同时运行，互不干扰。

![项目页：三个项目及其配置胶囊](docs/images/overview-projects.png)

## 功能

**三种配置来源**

- 跟随安装的默认配置；选用某个出厂 NLS 配置（`zh`、`u8`、`en`……，每个 `nls\<code>\sasv9.cfg` 一条，通常 20 条以上）；或从其中任一个衍生出用户配置。
- 用户配置是一份 3 行的薄壳 `sasv9.cfg`——`-CONFIG` 指向出厂配置文件，加上编码与区域——并可附一份 4 个 SAS catalog 文件的快照。出厂文件的内容不会被复制，所以 SAS 升级是自动继承，而不是逐一修补。
- 编码与区域可以在表单里改，也可以打开一个专用 SAS 会话调整界面偏好，退出后把 catalog 存回该配置。

![配置管理页：出厂配置行，以及一份带快照的用户配置](docs/images/overview-configurations.png)

**每个窗口一套环境**

- 每次启动都有独立的、可写的 `-SASUSER` 目录，会话退出即删除。
- 可以批量启动常用项目；已在运行的项目会自动跳过。

**能据以排查的诊断**

- 每条记录保留解析链（薄壳 cfg → 出厂 cfg → 生效的编码、区域与 `SASUSER`）、真实启动参数、失败的阶段和退出码。
- SAS 日志通过 `-ALTLOG` 抓取，写入的是副本：Log 窗口照旧可用。只保留最近 20 条。

![会话诊断页：解析链与日志摘要](docs/images/overview-diagnostics.png)

## 系统要求

> [!IMPORTANT]
> 本机必须已安装并授权 SAS Foundation 9.4。SAS Launcher 只负责启动它，不附带 SAS 本体。

- Windows 10 或 11，x64
- SAS Foundation 9.4（已在 TS1M7 验证）
- WebView2 运行时——Windows 11 自带；Windows 10 可能需安装 Evergreen Runtime

## 安装

1. 从 [Releases](https://github.com/wtwang1998/sas-launcher/releases) 下载 `SAS-Launcher-0.2.0-x64-Setup.exe`。
2. 安装包只装给当前用户，落在 `%LOCALAPPDATA%\SAS Launcher`，不需要管理员权限。
3. 升级与卸载沿用该目录。

> [!NOTE]
> 安装包未做代码签名，Windows SmartScreen 会提示“Windows 已保护你的电脑”。这是未签名构建的正常现象：选择**更多信息** → **仍要运行**。请只从本仓库的 Releases 页面下载。

### 数据与卸载

项目、配置与诊断记录保存在 `%LOCALAPPDATA%\com.saslauncher.desktop`，与程序装在哪里无关。卸载只删除程序文件，数据会保留；想从干净状态开始，手动删除该目录即可。

## 快速开始

1. 启动应用，它会自动找到已安装的 SAS 9.4；找不到时，横幅里可以手动选择 `sas.exe`。
2. 在**配置管理**里浏览出厂配置，或衍生一个用户配置并调整它的编码与区域。
3. 在**项目**里新建项目：名称、三选一的配置来源、可选备注。
4. 需要改某个配置的 SAS 界面偏好时，选「打开 SAS 调整设置」，改完退出 SAS 再保存。
5. 启动不顺利时，打开**会话诊断**查看解析链、启动参数与日志。

## 已知限制

- SAS **增强编辑器**的字体、颜色与按键保存在 Windows 注册表（`HKCU`）。SAS 没有提供改变其位置的选项，因此这部分无法按配置隔离或保存。这是 SAS 的限制，不是缺少的功能。
- 0.2 之前版本的数据不做迁移：旧目录会被归档到应用数据目录下的 `_old-<日期>`，应用从干净状态开始。

## 从源码构建

需要 Rust MSVC 工具链（edition 2024）、Visual Studio C++ Build Tools、Node.js 与 WebView2 运行时。仓库用 `rust-toolchain.toml` 固定 Rust 1.99.1、用 `.nvmrc` 固定 Node 24.22.3，`rustup` 与 `nvm` 会自动选用。

```powershell
winget install --id Rustlang.Rustup
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

git clone https://github.com/wtwang1998/sas-launcher.git
cd sas-launcher
npm ci
npm run tauri dev
```

首次编译 Rust 依赖约 3–10 分钟。改 `src/**` 走热更新；改 `src-tauri/**` 会重新编译并重启窗口。

| 命令 | 用途 |
| --- | --- |
| `npm run check` | 全部门禁：格式化、lint、clippy、类型、两套测试（约 12 秒） |
| `npm run test` / `npm run test:rust` | 前端（vitest）/ 后端（cargo test） |
| `npm run build` | 类型检查，然后构建前端 |
| `npm run tauri build` | 编译应用并打 NSIS 安装包到 `src-tauri/target/release/bundle/nsis/` |

用 AI agent 参与开发？先读 [AGENTS.md](AGENTS.md)：里面写的是测试兜不住的那些不变量。

## 持续集成

每个 pull request 和每次推送到 `main`，都会在 `windows-latest` 上跑与你本地相同的门禁，分三个并行 job：`web`（格式化、lint、类型、前端测试、构建）、`rust`（cargo fmt、clippy、后端测试，以及 `Cargo.lock` 是否与清单一致）、`commitlint`。

推送 `v<version>` tag 会触发出发布流程：tag 与 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 三处版本不一致就拒绝构建；通过后构建未签名安装包，放在 draft release 里等人审阅。

欢迎贡献——见 [CONTRIBUTING.zh-CN.md](CONTRIBUTING.zh-CN.md)。

## 许可证

以 [GPL-3.0-or-later](LICENSE) 发布。
