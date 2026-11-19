# SAS Launcher

Windows 上的 SAS Foundation 9.4 桌面启动器。按编码、SAS 区域设置和启动配置打开独立的 SAS 窗口。

## 使用

1. 首次启动会查找 SAS 9.4 和原有用户设置。找不到 SAS 时可手动选择 `sas.exe`。
2. 在「启动配置」中查看本机原厂组合，或复制原厂配置来创建自己的配置。自定义的 sasv9.cfg 存在应用数据目录，可重新命名、更换原厂基础配置或删除。
3. 新建项目或使用「临时启动」，依次选择编码、SAS 区域设置和具体的启动配置。配置选项单独显示 SAS 界面语言。常用项目可勾选多个并点击「启动所选」。已在运行的项目会被批量操作跳过。
4. 打开 SAS 后，按原来的方式运行项目的 `setup.sas`。

每次启动使用一份单独、可写的用户设置目录。没有保存个人设置时，SAS 会在空白用户目录中生成初始设置。SAS 退出后，启动器回收该目录。请把要保留的程序、数据和报告写到项目目录。SAS 主窗口布局和 Display Manager 快捷键等设置可以在「启动配置」的「SAS 个人设置」区更新。模板包含 SAS 用户目录中的个人设置文件。增强编辑器的选项由 Windows 用户共享。[SAS 增强编辑器文档](https://support.sas.com/documentation/cdl/en/hostwin/69955/HTML/default/editor_enhanced.htm)

安装包面向 Windows 10/11 x64，需要本机安装 SAS Foundation 9.4。首次安装默认仅为当前用户安装到 AppData 下的 SAS Launcher 文件夹，也可以选择 Program Files 下的 SAS Launcher 文件夹，后者需要管理员权限。英文产品名发布后，升级沿用 SAS Launcher 的安装范围和目录。测试版的中文安装记录由 Windows 单独管理。应用和安装器支持简体中文与英文。应用数据位于 `%LOCALAPPDATA%\com.saslauncher.desktop`。

## 开发

依赖 Rust MSVC 工具链、Visual Studio C++ Build Tools、Node.js 22.12+ 和 WebView2。提交的安装包图标已有成品，重新生成图标时才需要 Python 与 Pillow。

```powershell
npm ci
npm run tauri dev
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run package:windows
```

安装包输出在 `src-tauri/target/release/bundle/windows/`。
