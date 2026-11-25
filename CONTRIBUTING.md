# Contributing to SAS Launcher

Thanks for taking the time to contribute. This file covers what you need to
build the app, what has to pass before a pull request can merge, and how a
release is cut. The invariants a test cannot catch live in
[AGENTS.md](AGENTS.md); read it before touching launch arguments, the data
directory or the installer.

**English** | [简体中文](./CONTRIBUTING.zh-CN.md)

## Before you start

- Windows 10 or 11, x64. The app launches SAS Foundation 9.4 and cannot be
  developed usefully on another platform.
- Rust 1.99.1: `rust-toolchain.toml` pins it, so `rustup` installs the right
  version on the first `cargo` command.
- Node 24.22.3: `.nvmrc` pins it, so `nvm use` (or `fnm use`) selects it.
- Visual Studio C++ Build Tools with the MSVC workload, and the WebView2
  Runtime.
- SAS Foundation 9.4 if you want to exercise a real launch. The test suites do
  not need it: every test uses `tempfile` directories and an inert fixture.

```powershell
git clone https://github.com/wtwang1998/sas-launcher.git
cd sas-launcher
npm ci
npm run tauri dev
```

Use `npm ci`, not `npm install`: the second rewrites `package-lock.json`.

## The gate

```powershell
npm run check        # formatting, lint, clippy, types, both test suites
```

| Command | Purpose |
| --- | --- |
| `npm run check` | every gate, about 15 s |
| `npm run test` | frontend tests (vitest, jsdom, fake backend) |
| `npm run test:rust` | backend tests (cargo test) |
| `npm run format` | rewrite formatting with biome and cargo fmt |
| `npm run lint` | biome lint plus knip |
| `npm run lint:rust` | clippy with `-D warnings` |
| `npm run typecheck` | `tsc --noEmit` |

Formatting must keep CRLF: the repository stores CRLF and both formatters are
configured for it. Do not add a `.gitattributes` that changes that.

## Tests

Work test-first. Write the failing test, watch it fail for the right reason,
then implement the smallest change that passes it.

Tests live at five seams:

1. Pure functions in `src/lib/` and `src-tauri/src/*` — table-driven tests.
2. Pages in `src/pages/` — Testing Library against `mockIPC`; assert what the
   user sees and what the app submits, never internal state.
3. The IPC contract in `src/api.test.ts` — command names and argument keys.
4. Rust modules — `#[cfg(test)] mod tests` with real `tempfile` directories.
5. The release guard in `scripts/` — keeps the tag and the three version
   fields honest.

Do not introduce traits or mock file systems to make something testable. The
existing tests use real files, real temporary directories and, where the code
spawns a process, a real `powershell.exe` child.

Style, configuration and documentation changes do not need tests.

## Commits

- Conventional commits, English imperative mood: `fix(ui): keep the banner on
  one line`.
- Body lines stay under 100 characters; the commit-msg hook enforces it.
- One logical change per commit. Keep formatting separate from behaviour.
- No AI attribution and no `Co-authored-by` trailers.
- `README.md` and `README.zh-CN.md` change in the same commit; the English one
  wins when they disagree.

## Pull requests

- `main` only accepts squash merges, so **the pull request title becomes the
  commit message on `main`**. Write it as a conventional commit.
- Three checks must pass: `web`, `rust` and `commitlint`. They run the same
  commands as `npm run check`, on `windows-latest`.
- Do not use `[skip ci]`: the checks are the reason `main` stays green.
- Describe the behaviour that changed, not the files you edited.

Commands passing is not the same as the change working. If you touched a
launch, the installer or anything visual, run it on a machine with SAS 9.4
installed and say in the pull request what you ran and what you saw.

## Releases

Maintainers only. A release starts with a tag:

1. Bump the version in `package.json`, `src-tauri/Cargo.toml` and
   `src-tauri/tauri.conf.json` in one commit on `main`.
2. Write `docs/release-notes/v<version>.md` from
   `docs/release-notes-template.md` and commit it with the bump. The workflow
   publishes that body as the draft, and the file name must match the tag.
3. Tag that commit `v<version>` and push the tag.
4. The `Release` workflow refuses to build unless the tag matches all three
   files, then uploads the installer to a **draft** release. A tag without a
   notes file still builds: it falls back to `--generate-notes`, which lists
   pull requests, and `main` receives none, so that body is empty.
5. Install the draft on a clean Windows machine, check the upgrade from the
   previous version, then publish.

The installer is not code-signed, so SmartScreen warns before it runs. The
release notes say so.

## Questions

Open an issue: a bug report with the diagnostics record, or a feature request
with the workflow it would unblock.
