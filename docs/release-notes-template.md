# Release notes template

The draft release body is written by hand from this template. `gh release create
--generate-notes` lists pull requests, and this repository commits straight to
`main`, so the generated body comes back empty. Paste a filled template over it
before publishing.

## How to use

1. Copy the skeleton below into a file, fill it in, then apply it with
   `gh release edit v<version> --notes-file <file>`.
2. Replace every `<...>`: version, installer name, size, SHA-256, bullets.
3. Read the checksum from the file that is actually uploaded:
   `sha256sum release-assets/SAS-Launcher-<version>-x64-Setup.exe`.
4. Build the collapsed commit list with
   `git log --reverse --format=%h%x09%s v<previous>..v<version>`, then prefix
   every line with `` - `<hash>` ``.
5. Keep the release as a draft, and a pre-release while the version is still
   below 1.0, until the real-machine check passes.

## Skeleton

````markdown
**简体中文** | [English](#english)

## 简体中文

> [!WARNING]
> **<一句话说明破坏性变更；没有就删掉整块>**

安装包：`SAS-Launcher-<version>-x64-Setup.exe`（<size> MB）· SHA-256 `<sha256>`

### 安装

1. <升级前必须先做的事>
2. <安装步骤>
3. <旧数据或旧版本的留档位置>

回退：<怎么退回上一版，数据怎么恢复>

### 本次变更

- <一条一个用户可感知的变化，最多 6 条>

### 已知限制

- <本版确实做不到的事，最多 4 条>

> [!IMPORTANT]
> 安装包未签名。Windows SmartScreen 会拦截首次运行：只有在信任本次构建时才选「更多信息」→「仍要运行」。

完整提交列表见文末。

## English

> [!WARNING]
> **<the same breaking change in one sentence; drop the whole block when there is none>**

Installer: `SAS-Launcher-<version>-x64-Setup.exe` (<size> MB) · SHA-256 `<sha256>`

### Install

1. <what has to happen before upgrading>
2. <install steps>
3. <where the old data or the previous version stays>

Rolling back: <how to go back, and how the data is recovered>

### In this release

- <one user-visible change per bullet, at most 6>

### Known limitations

- <what this release genuinely cannot do, at most 4>

> [!IMPORTANT]
> This installer is unsigned. Windows SmartScreen will warn when you run it: choose "More info" and then "Run anyway" only if you trust this build.

The full commit list is at the end.

[简体中文](#简体中文) | **English**

---

**Full Changelog**: https://github.com/wtwang1998/sas-launcher/compare/v<previous>...v<version>

<details>
<summary>Commits in this release / 本次发布的提交（<count>）</summary>

- `<hash>` <subject>

</details>
````

## Formatting rules

- Two complete halves, not an interleaved translation. A reader who picked one
  language should never have to read the other.
- The switcher is the first line: bold the half that comes first, link the other
  one with its heading anchor (`#english`, `#简体中文`). Do not hand-write
  `<a id="...">`; GitHub rewrites custom ids to `user-content-*`, so the link
  stops matching.
- Inside a half, sections are `###` and the half itself is `##`. Never use `#`:
  the release page already prints the version as its title.
- Wrap flags, paths, issue codes and file names in backticks. No emoji, no
  colour, no tables: the release page is narrow on mobile.
- At most 6 change bullets and 4 limitation bullets per half; the full detail
  belongs in the collapsed commit list.
- Keep `**Full Changelog**` visible and the commit list collapsed in
  `<details>`.
- Both halves follow the same order: warning, installer identity, install,
  changes, limitations, unsigned note.
- Delete a block instead of leaving `<...>` behind: an unfilled placeholder ships
  as a broken note.

The published v0.2.0 release is the reference example.
