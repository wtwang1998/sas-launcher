# AGENTS.md

面向在本仓库工作的 AI 编码 agent；人类文档见 `README.md`。

**本文件是常驻上下文，每加一行都会挤掉别的约束的注意力。只写代码与测试表达不了的约束；能由测试锁住的不写在这里。**

事实基线：Windows 10/11 x64、SAS Foundation 9.4、Tauri 2 + React 19 + TypeScript(strict) + Rust edition 2024。
仓库以 **CRLF** 存盘且没有 `.gitattributes`，所有格式化工具必须保持 CRLF（见「硬约束」）。

## 命令

```bash
npm ci                    # 首次安装；用 ci 不用 install，后者会改写 lock
npm run check             # 全部门禁，约 12 秒 —— 提交前跑这个
npm run format            # biome format --write + cargo fmt
npm run lint              # biome lint + knip
npm run lint:rust         # clippy --all-targets -- -D warnings
npm run typecheck         # tsc --noEmit
npm run test              # vitest，约 5 秒
npm run test:rust         # cargo test，约 1 秒（增量已编译）
npm run build             # tsc --noEmit + vite build
npm run version:check -- v0.2.0   # 发布前自查：tag 与三处版本是否一致
npm run i18n:coverage             # 各语言覆盖率与缺失 key；`-- --update` 接受下降并改基线
```

- Rust 与 Node 版本钉死：`rust-toolchain.toml`（1.99.1 + clippy/rustfmt）、`.nvmrc`（24.22.3）、`package.json` 的 `engines`。升级它们是一次独立提交，别顺手改。
- 只跑一个前端测试：`npm run test -- src/api.test.ts`
- 只跑一个 Rust 测试：`npm run test:rust -- <测试名>`
- 所有 `cargo` 命令都要带 `--manifest-path src-tauri/Cargo.toml`：根目录不是 workspace，裸 `cargo` 会失败。
- `npm run tauri dev` 与 `npm run tauri build` **只在用户明确要求时**执行：首次编译 Rust 依赖 3–10 分钟，打包会写 `src-tauri/target/release/`。

## 目录地图

```text
src/                    React 外壳：App.tsx（路由 + 2.8s 轮询）、api.ts（唯一的 invoke 封装）、i18n/（英文事实源 + 三语言词典 + 不变量测试）
src/pages/              三个页面；只接收 props 与回调，不 import api
src/components/         可复用控件：自绘 listbox、确认对话框、表单字段
src/lib/                纯函数：标签、排序、默认名、argv 引号 —— 必须可表驱动测试
src/test/               harness.tsx（mockIPC 假后端 + renderApp）、fixtures.ts
src-tauri/src/          Rust：main.rs 命令层 / store.rs 持久化 / session.rs 会话 / view.rs DTO
src-tauri/capabilities/ 窗口权限白名单
src-tauri/windows/      NSIS 安装钩子（preserve-install-location.nsh）
scripts/                发布版本校验与 i18n 覆盖率（Node，配 vitest 表驱动测试）
docs/images/            README 截图（真机拍摄）
docs/i18n.md            界面语言与安装器语言的约定、门禁与手工验收清单
```

## 硬约束（违反后测试照样绿）

| 禁止 | 为什么 |
|---|---|
| 内联 style（`style={{…}}`） | CSP 是 `style-src 'self'`，打包后静默失效；改 `src/style.css` |
| `std::fs::canonicalize` 写进 cfg | 会带 `\\?\` 前缀，SAS 打不开；用 `dunce::canonicalize` |
| 把 NSIS 语言名写成 Tauri 没有译文的 | 构建照绿，安装器界面显示裸 `$(createDesktop)` 之类；只能取 Tauri 2.12 自带译文的 23 个，`src/lib/installer.test.ts` 会拦 |
| 把 `preserve-install-location.nsh` 存成带 BOM 或非 UTF-8 | makensis 以 `-INPUTCHARSET UTF8` 读它，BOM/GBK 让中文乱码 |
| 在安装钩子里用 `LangString` | 宏在 Section 内展开，makensis 报 `command LangString not valid in Section`；钩子文案只能运行时按 `$LANGUAGE` 选，英文兜底 |
| 改 Rust DTO 不同步 `src/api.ts` | 契约测试只校验命令名与参数键，字段漂移无人拦 |
| `src/pages/` 里 import `api` | 页面必须能在假后端下渲染 |
| 改 `identifier` / `installerHooks` / NSIS 安装范围 | 会孤立用户数据与升级路径 |
| 真实启动 SAS、动真实应用数据目录 | 测试一律 `tempfile` 临时目录；真实数据在 `%LOCALAPPDATA%\com.saslauncher.desktop` |
| 引 UI 库或设计系统 | 只用 `src/style.css` + `lucide-react` |
| 在 `capabilities/default.json` 加宽泛权限 | 只加当前功能必需的；不开 `fs`/`shell` 通配 |
| 让格式化改掉 CRLF | `biome.json` 的 `lineEnding: "crlf"` 与 `rustfmt.toml` 的 `newline_style = "Windows"` 不得改 |
| 把 `profbak` / `folder` / `gng` / `parms` 加进 profile 白名单 | 前三个是 SAS 的回退副本与 ASSIST、Graph-N-Go 产品资源，`parms` 是别台机器写的 ASSIST 参数，都不是用户偏好；`profile.rs` 的注释写着排除理由 |

已被测试锁死、无需在此重复：薄壳 cfg 恰好 3 行、启动参数顺序与 `-ENCODING`/`-LOCALE` 缺席、profile 白名单 4 个文件且分界面/环境/模板三层、项目偏好快照保留 2 版、诊断保留 20 条、运行中与待保存的设置会话不可被清空、Rust 发出的每个 issue 码都有三语文案、词典无未知/未用 key、译文占位符与英文源一致、非源语言覆盖率不低于 `src/i18n/coverage-baseline.json`。

## 测试

- 五条缝：纯函数（表驱动）/ 页面（RTL + `mockIPC`，只断言看见什么、提交什么）/ IPC 契约（`api.test.ts`）/ Rust（`#[cfg(test)] mod tests`）/ 发布 guard（`scripts/*.test.mjs`）。
- 新行为先写失败测试再实现。**不要**为测试引入 trait 或 mock 文件系统：现有测试直接建 `tempfile::tempdir()`，用真实文件与真实 `powershell.exe` 子进程。
- 能走到 `launch` 的测试必须用 `app_with_install(temp, &exe)` 钉住 exe：没钉时 `App::new` 会回退到 `discovery::find_installed()`，在真机上启动真正的 SAS。helper 里的断言会在路径跑出临时目录时当场 panic。
- 样式、纯配置、文档改动不写测试。
- 真实 SAS 启动、安装包、视觉：**命令全绿不等于完成**，必须在真机手工走一遍，并在答复里说明跑了什么、结果如何。
- 不要假定 clippy 警告或测试失败是既有的；先确认再下结论。

## 提交

- conventional commits，英文祈使句；`body-max-line-length` 为 100，commitlint 会拦。
- 一次逻辑变更一次 commit；格式化与行为改动分开提交。
- 不加 AI 署名或 `Co-authored-by` trailer。
- `README.md` 与 `README.zh-CN.md` 必须同一次改动里一起改；两者冲突时以 `README.md` 为准。
- commitlint 现在也在 CI 里跑：本地钩子被绕过（见 Known gaps 的 MSYS 条目）不等于通过。

## CI

- 三个 job，名字被分支保护引用，改名等于关掉保护：`web`（biome / knip / tsc / vitest / vite build）、`rust`（cargo fmt / clippy / cargo test / `Cargo.lock` 与清单一致）、`commitlint`（PR 标题 + 提交范围）。
- 本地复现 web：`npm run format:check:web && npm run lint && npm run typecheck && npm run test && npm run build`；复现 rust：`npm run format:check:rust && npm run lint:rust && npm run test:rust`。
- 不许用 `[skip ci]`、不许合并不等检查、不许为了变绿改 job 名或删 step。
- 发布：推 `v<version>` tag；tag 必须与 `package.json` / `Cargo.toml` / `tauri.conf.json` 完全一致，否则 `Release` 直接失败；产物是 draft release 里的未签名安装包，人审后手动发布。

## Known gaps（是现状，不是范例）

| 现象 | 处理 |
|---|---|
| `unexpected` 有文案但无生产者 | 它是前端未知码的兜底标题（`src/i18n/index.ts`），有意保留，别删 |
| 默认配置名生成在 `configuration.rs` 与 `src/lib/config.ts` 各一份 | 文案模板已统一到 `shared/labels.json`（Rust `include_str!`、前端 import），但两份实现仍在；前端的 `defaultConfigName` 没有 UI 调用，改逻辑仍要同时改两处 |
| `biome.json` 关掉了 `noDescendingSpecificity` 与 4 个文件上的 4 条 a11y 规则 | 修需人工验视觉/键盘，原因见 `overrides` |
| MSYS 下 git 直接 exec husky 钩子报 Permission denied | 用 `git -c core.hooksPath=` 提交，再手动 `npx commitlint --from HEAD~1 --to HEAD` |

代码与测试是唯一事实源：本文件与代码冲突时，先改本文件。
