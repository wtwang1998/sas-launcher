# SAS Launcher

多个 SAS 项目往往需要不同的会话环境（编码、区域设置、启动配置），同时打开又会因共用用户设置而互相读写冲突。SAS Launcher 为每个项目准备独立的会话环境，让它们可以同时运行、互不干扰。

![平台](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%20x64-0078D4)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB)
![许可证](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)

## 功能

- 每个项目记住自己的启动配置（`sas.exe` + `sasv9.cfg`），编码、区域设置和 SAS 界面语言随项目固定。
- 扫描 `nls` 目录发现原厂配置；需要定制时复制成自定义配置，改的是应用数据目录里的副本，SAS 安装目录保持只读。
- 每个 SAS 窗口使用一份独立、可写的用户设置目录，退出后自动回收，多项目并行时互不覆盖。
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
| `cargo test --manifest-path src-tauri/Cargo.toml` | 后端单元测试 |
| `npm run package:windows` | 打安装包，输出到 `src-tauri/target/release/bundle/windows/` |

## 使用

1. 首次启动自动查找已安装的 SAS 9.4 和已有用户设置，找不到时可手动选择 `sas.exe`。
2. 在「启动配置」查看本机原厂组合，或复制一份来创建自定义配置。
3. 在「常用项目」或「临时启动」选好编码、区域设置和配置后启动；进入 SAS 后照常运行项目的 `setup.sas`。

## 安装

要求 Windows 10/11 x64，且本机已安装 SAS Foundation 9.4。从 [Releases](https://github.com/wtwang1998/sas-launcher/releases) 下载 `SAS-Launcher-0.1.0-x64-Setup.exe`；首次安装默认仅为当前用户装到 `%LOCALAPPDATA%\SAS Launcher`，升级与卸载沿用该范围。应用数据在 `%LOCALAPPDATA%\com.saslauncher.desktop`。

## 许可证

以 [GPL-3.0-or-later](LICENSE) 发布。
