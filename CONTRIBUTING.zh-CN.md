# 为 SAS Launcher 做贡献

感谢你愿意花时间。本文说明怎么把应用跑起来、合并前必须通过什么、以及一次发布是怎么产生的。测试抓不住的约束在
[AGENTS.md](AGENTS.md)：动启动参数、数据目录或安装包之前先读它。

[English](./CONTRIBUTING.md) | **简体中文**

## 开始之前

- Windows 10 或 11 x64。应用要启动 SAS Foundation 9.4，换平台基本没法开发。
- Rust 1.99.1：由 `rust-toolchain.toml` 固定，`rustup` 会在第一次 `cargo` 时自动装好。
- Node 24.22.3：由 `.nvmrc` 固定，`nvm use`（或 `fnm use`）会选中它。
- Visual Studio C++ Build Tools 的 MSVC 工作负载，以及 WebView2 Runtime。
- 想真的启动一次 SAS，需要本机装有 SAS Foundation 9.4。测试套件不需要：所有测试都用 `tempfile`
  临时目录和一个不会真正运行的假 `sas.exe`。

```powershell
git clone https://github.com/wtwang1998/sas-launcher.git
cd sas-launcher
npm ci
npm run tauri dev
```

用 `npm ci`，不要用 `npm install`：后者会改写 `package-lock.json`。

## 门禁

```powershell
npm run check        # 格式、lint、clippy、类型、两套测试
```

| 命令 | 作用 |
| --- | --- |
| `npm run check` | 全部门禁，约 15 秒 |
| `npm run test` | 前端测试（vitest + jsdom + 假后端） |
| `npm run test:rust` | 后端测试（cargo test） |
| `npm run format` | biome 与 cargo fmt 重写格式 |
| `npm run lint` | biome lint 与 knip |
| `npm run lint:rust` | 带 `-D warnings` 的 clippy |
| `npm run typecheck` | `tsc --noEmit` |

格式化必须保持 CRLF：仓库以 CRLF 存盘，两个格式化工具都为此配置。不要加会改变这一点的 `.gitattributes`。

## 测试

先写测试。让测试因为正确的原因失败，再写刚好让它通过的最小实现。

测试落在五条缝上：

1. `src/lib/` 与 `src-tauri/src/*` 里的纯函数 —— 表驱动测试。
2. `src/pages/` 的页面 —— 用 Testing Library 对着 `mockIPC`，断言用户看见什么、应用提交了什么，绝不碰内部状态。
3. `src/api.test.ts` 里的 IPC 契约 —— 命令名与参数键。
4. Rust 模块 —— `#[cfg(test)] mod tests`，用真实的 `tempfile` 临时目录。
5. `scripts/` 里的发布 guard —— 保证 tag 与三个版本字段一致。

不要为了让代码可测而引入 trait 或假文件系统。现有测试直接用真实文件、真实临时目录，需要子进程时就用真实的
`powershell.exe`。

样式、配置、文档改动不写测试。

## 提交

- conventional commits，英文祈使句：`fix(ui): keep the banner on one line`。
- 正文每行不超过 100 字符，commit-msg 钩子会拦。
- 一次逻辑变更一次 commit，格式化与行为改动分开。
- 不加 AI 署名，也不加 `Co-authored-by`。
- `README.md` 与 `README.zh-CN.md` 在同一次提交里一起改；两者冲突时以英文版为准。

## Pull request

- `main` 只接受 squash 合并，所以**PR 标题会成为 `main` 上的 commit message**，请按 conventional commit 写。
- 三个检查必须通过：`web`、`rust`、`commitlint`。它们在 `windows-latest` 上跑与 `npm run check` 相同的命令。
- 不要用 `[skip ci]`：这些检查就是 `main` 一直绿的原因。
- 描述改变了什么行为，而不是改了哪些文件。

命令全绿不等于改动可用。碰到启动、安装包或任何视觉改动，请在有 SAS 9.4 的机器上真跑一遍，并在 PR 里说明你跑了什么、看到什么。

## 发布

仅维护者。一次发布从一个 tag 开始：

1. 在 `main` 上用一次提交同时改 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 的版本号。
2. 给那个提交打 `v<version>` tag 并推送。
3. `Release` workflow 会先校验 tag 与三个文件一致，不符就不构建；通过后把安装包传到一个 **draft** release。
4. 按 `docs/release-notes-template.md` 填写 draft 正文。`--generate-notes` 只列 PR，而 `main` 不收 PR，自动生成的正文是空的。
5. 在一台干净的 Windows 机器上安装这个 draft，检查从上一版本的升级路径，再手动发布。

安装包没有代码签名，运行前 SmartScreen 会警告，release 说明里已经写明。

## 提问

开 issue：bug 请附上诊断记录，feature 请说明它能让哪个流程变顺。
