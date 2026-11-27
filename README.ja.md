# SAS Launcher

同じマシン上で複数の SAS 9.4 プロジェクトを並行して実行し、プロジェクトごとに独立したセッション環境を持たせます。

![プラットフォーム](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%20x64-0078D4)
![最新リリース](https://img.shields.io/github/v/release/wtwang1998/sas-launcher?include_prereleases)
![ダウンロード数](https://img.shields.io/github/downloads/wtwang1998/sas-launcher/total)
![ライセンス](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)
[![CI](https://github.com/wtwang1998/sas-launcher/actions/workflows/ci.yml/badge.svg)](https://github.com/wtwang1998/sas-launcher/actions/workflows/ci.yml)

[English](./README.md) | [简体中文](./README.zh-CN.md) | [繁體中文](./README.zh-Hant.md) | **日本語** | [한국어](./README.ko.md) | [Deutsch](./README.de.md) | [Français](./README.fr.md) | [Español](./README.es.md)

## なぜ作ったか

SAS 自体は複数ウィンドウを開けます。問題は多重起動ではなく、すべてのウィンドウが同じユーザー設定フォルダー（`SASUSER`）を共有することです。フォント、色、ウィンドウ状態、`PROC TEMPLATE` の既定テンプレートストアがこの 1 つのフォルダーに入っており、SAS はその更新アクセスに排他を要求します（[RSASUSER システムオプション](https://support.sas.com/documentation/cdl/en/hostwin/69955/HTML/default/n0npwfu9o7rvtun1f8f4ztpug83z.htm)）。

そのため 2 つ目のウィンドウは Work ライブラリ内のコピーにフォールバックし、見た目は既定のまま、変更は終了時に失われます。

> `NOTE: SASUSER.PROFILE を開けません。WORK.PROFILE を開きます。`

レポート出力ではもっと直接的です。プロジェクト固有の ODS RTF テンプレートがこのフォルダーに書き込めず、`PROC TEMPLATE` がその場でエラーになり、以後は既定スタイルしか使えません。そのウィンドウではプログラムは動いても、必要なレポートは出せません。公式の `ODS PATH` はテンプレートだけを扱い、環境設定は扱いません。「プロジェクトごとにショートカットを 1 つ」という手もありますが、プロジェクトが増えるとアイコンだらけになります。

SAS Launcher はプロジェクトごとに独自の構成を用意し、起動ごとに独立した書き込み可能な `SASUSER` フォルダーを用意して、セッション終了時に削除します。複数プロジェクトが同時にレポートを出せるようになり、互いに干渉しません。

![プロジェクトページ: 3 つのプロジェクトと構成チップ](docs/images/overview-projects.png)

## 機能

**3 種類の構成ソース**

- インストール済みの既定構成を使う。出荷時の NLS 構成（`zh`、`u8`、`en` など、`nls\\<code>\\sasv9.cfg` ごとに 1 件、通常 20 件以上）から選ぶ。またはそのいずれかからユーザー構成を派生させる。
- ユーザー構成は 3 行の薄い `sasv9.cfg` です。`-CONFIG` で出荷時の構成ファイルを指し、エンコーディングとロケールを加え、保存した 4 つの SAS 設定ファイルを添付できます。出荷時のファイル内容はコピーされないため、SAS のアップグレードは自動的に継承され、個別に手直しする必要はありません。
- エンコーディングとロケールはフォームで変更でき、専用の SAS セッションを開いてインターフェース設定を調整し、終了時にその catalog を構成へ保存することもできます。

![構成管理ページ: 出荷時構成の行と、設定ファイル付きのユーザー構成](docs/images/overview-configurations.png)

**ウィンドウごとに 1 つの環境**

- 起動ごとに独立した書き込み可能な `-SASUSER` フォルダーを用意し、セッション終了時に削除します。
- プロジェクトは既定でインターフェース設定と環境設定を持ち帰ります。ウィンドウ配置、ファンクションキー、メニュー、Explorer オプション、ライブラリ割り当てなどで、セッション終了時に終了コードに関係なく変更を回収します。ODS テンプレートは手動で選択する必要があります。
- よく使うプロジェクトをまとめて起動できます。すでに実行中のプロジェクトは自動的にスキップされます。

**プロジェクト固有の起動コード**

- プロジェクトに専用の autoexec ファイルを指定できます。起動のたびに SAS はそれを先に実行してから入力を受け付けます。グローバル変数、raw/分析データ/TFL のフォルダー、マクロライブラリをプロジェクトで 1 回設定すれば、セッションごとに繰り返す必要はありません。
- `-APPEND AUTOEXEC` によりサイトの autoexec チェーンの後ろに追加されるため、インストール先や社内の `autoexec.sas` が先に実行されます。ファイルが存在しない場合は、パスが欠けたセッションを開くのではなく起動を拒否します。
- 「セッションにスタートアップファイルのパスを渡す」を有効にすると、スクリプトから `&SASLAUNCHER_STARTUP_FILE` を参照できます。ランチャーはプロジェクトファイルの前にごく短い prelude を実行してこれを定義します。SAS はウィンドウセッションの起動時に `SAS_EXECFILEPATH` をリセットし、非 ASCII パスは UTF-8 セッションでも環境変数の層を通らないため、パスはマクロ変数で渡します。`%sysget(SAS_EXECFILEPATH)` だけを読むスクリプトは、空のパスで入るのではなく起動を拒否されます。
- すでに `%sysget(SAS_EXECFILEPATH)` で書かれた初期化スクリプトは、その拒否から直接直せます。「スタートアップファイルを移行」を押すと、既知の代入を `%symexist` 判定へ書き換え、元のファイルを `<名前>.bak` として残し、そのスイッチを有効にして再起動します。ファイル内の他の内容には一切触れません。参照箇所が書き換え範囲にないスクリプトは、中途半端に書き換えるのではなく拒否されます。
- 対象範囲: Windows 上の SAS 9.4 Foundation セッションで、本ランチャーから起動したもの。スクリプトはウィンドウセッションの autoexec チェーンに載るため、Enterprise Guide、SAS Studio、SAS Grid、Linux/Unix 上の SAS には効果がありません。プロジェクトのディレクトリ階層も仮定しません。パスはプロジェクトが絶対パスで指定し、ファイル名は任意です。書き換えはファイルをテキストとしてデコードしないため、GBK のスクリプトは書き換え後も GBK のままです。スクリプトがパスからプロジェクトルートを推定している場合、その階層数はスクリプト自身の約束であり、移行はその推定に触れません。
- 移行が認識する書き方は 1 つだけです。`%let` 代入で、右辺全体が `%sysget(SAS_EXECFILEPATH)` であるもの。`&_SASPROGRAMFILE`、`%sysfunc(getoption(sysin))`、別ファイルから読み込んだパスなど他の特定方法は、変更も警告もしません。

スクリプト内で `SAS_EXECFILEPATH`、`_SASPROGRAMFILE`、`getoption(sysin)` を検索すれば、どのケースか判断できます。

| 起動スクリプトでの書き方 | 対処 |
| --- | --- |
| `%let <名前> = %sysget(SAS_EXECFILEPATH);` | 「スタートアップファイルを移行」を押す |
| `&_SASPROGRAMFILE`、`%sysfunc(getoption(sysin))`、別ファイルからパスを読む | 下の形に手で書き換える |
| パスを直書きしている、またはスクリプトが自分を特定しない | 変更不要 |

どのケースも最終的には同じコードになり、移行ボタンが書き出すのもそれです。`&SASLAUNCHER_STARTUP_FILE` の値はスタートアップファイル自身のパスで、セッション内でプログラムを投入したときの `SAS_EXECFILEPATH` の値に等しくなります。

```sas
%if %symexist(SASLAUNCHER_STARTUP_FILE) %then %do;
    %let <name> = &SASLAUNCHER_STARTUP_FILE;
%end;
%else %do;
    %let <name> = %sysget(SAS_EXECFILEPATH);  /* the original fallback */
%end;
```


**原因を追える診断**

- 各レコードは解決チェーン（薄い cfg → 出荷時 cfg → 実際に効いているエンコーディング、ロケール、`SASUSER`）、実際の起動引数、失敗した段階、終了コードを保持します。
- SAS ログは `-ALTLOG` で取得し、書き込まれるのはコピーです。Log ウィンドウはそのまま使えます。保持するのは最新 20 件までです。

![セッション診断ページ: 解決チェーンとログ要約](docs/images/overview-diagnostics.png)

## 動作環境

> [!IMPORTANT]
> 本マシンに SAS Foundation 9.4 がインストールされ、ライセンスされている必要があります。SAS Launcher は起動のみを担当し、SAS 本体は同梱しません。

- Windows 10 または 11、x64
- SAS Foundation 9.4（TS1M7 で検証済み）
- WebView2 ランタイム。Windows 11 は同梱、Windows 10 では Evergreen Runtime のインストールが必要な場合があります。ネイティブのオーバーレイスクロールバーには 125 以降が必要で、それ以前はクラシックなスクロールバーに戻ります

## インストール

1. [Releases](https://github.com/wtwang1998/sas-launcher/releases) から `SAS-Launcher-0.5.0-x64-Setup.exe` をダウンロードします。
2. インストーラーは現在のユーザーにだけインストールし、`%LOCALAPPDATA%\\SAS Launcher` に置きます。管理者権限は不要です。
3. アップグレードとアンインストールもこのフォルダーを使います。

> [!NOTE]
> インストーラーはコード署名されていないため、Windows SmartScreen が「Windows によって PC が保護されました」と表示します。これは未署名ビルドでは正常な動作です。**詳細情報** → **実行** を選んでください。ダウンロードは本リポジトリの Releases ページからのみ行ってください。

### データとアンインストール

プロジェクト、構成、診断記録は `%LOCALAPPDATA%\\com.saslauncher.desktop` に保存され、プログラムのインストール先とは無関係です。アンインストールはプログラムファイルだけを削除し、データは残ります。まっさらな状態から始めたい場合は、そのフォルダーを手動で削除してください。

## 設定

**設定**にはアプリ単位のスイッチがまとまっており、すべて即座に反映されます。保存ボタンはありません。

- **テーマ**——システムに従う、またはダーク/ライトを強制。各色値は `light-dark()` のペアで、タイトルバー、ネイティブのドロップダウン、スクロールバーも一緒に切り替わります。比較的新しい WebView2 ランタイムが必要です。
- **アプリの言語**——简体中文、繁體中文、English、日本語、한국어、Deutsch、Français、Español、またはシステムに従う。インストーラーには別に 23 の UI 言語があり、選択内容を記憶します。
- **SAS プログラムのパス**——検出された `sas.exe` を表示します。手入力、参照、「自動検出」で手動指定をやめて再スキャンできます。
- **サインイン時に自動起動**——現在のユーザーの `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run` に書き込みます。パスにスペースがあっても正しく起動します。
- **起動時にタスクトレイへ最小化**——サインイン時の自動起動のときだけ静かに起動します。「サインイン時に自動起動」を有効にするとこの行が現れ、自動起動を切っても値は保持され、再度有効にすると元の動作に戻ります。
- **閉じるときにタスクトレイへ最小化**——既定で有効です。ウィンドウを閉じると隠れるだけで、トレイアイコンに「ウィンドウを表示」と「終了」があります。無効にすると閉じた時点で終了しますが、SAS セッションが動いている場合は先に確認します。プログラムを再度起動すると、隠れていたウィンドウが呼び戻されます。

## クイックスタート


1. アプリを起動すると、インストール済みの SAS 9.4 を自動で見つけます。見つからない場合はバナーから `sas.exe` を手動で選べます。
2. **構成管理**で出荷時構成を眺めるか、ユーザー構成を派生させてエンコーディングとロケールを調整します。
3. **プロジェクト**で新規作成します。名前、3 択の構成ソース、ユーザー設定のソース（`My SAS Files` のインターフェース設定を引き継ぐ、いずれかの構成が保存したセッション設定を引き継ぐ、クリーンな環境）、任意のメモ、任意のスタートアップコードファイル。
4. ある構成の SAS インターフェース設定を変えたいときは「SAS を開いて設定を調整」を選び、終了してから保存します。
5. 起動がうまくいかないときは**セッション診断**を開き、解決チェーン、起動引数、ログを確認します。
6. **設定**でテーマ、言語、SAS プログラムのパス、閉じるときと起動時の動作を変更します。

## 既知の制限

- SAS **拡張エディター**のフォント、色、キー割り当ては Windows レジストリ（`HKCU`）に保存されます。SAS はその場所を変えるオプションを提供していないため、この部分は構成ごとに分離も保存もできません。これは SAS 側の制限であり、機能不足ではありません。
- 0.2 より前のバージョンのデータは移行しません。古いフォルダーはアプリのデータフォルダー内の `_old-<日付>` にアーカイブされ、アプリはまっさらな状態から始まります。

## ソースからビルド

Rust の MSVC ツールチェーン（edition 2024）、Visual Studio C++ Build Tools、Node.js、WebView2 ランタイムが必要です。リポジトリは `rust-toolchain.toml` で Rust 1.99.1 を、`.nvmrc` で Node 24.22.3 を固定しており、`rustup` と `nvm` が自動的に選択します。

```powershell
winget install --id Rustlang.Rustup
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

git clone https://github.com/wtwang1998/sas-launcher.git
cd sas-launcher
npm ci
npm run tauri dev
```

Rust 依存の初回ビルドは約 3〜10 分です。`src/**` の変更はホットリロード、`src-tauri/**` の変更は再ビルドとウィンドウの再起動になります。

| コマンド | 用途 |
| --- | --- |
| `npm run check` | すべてのゲート: 整形、lint、clippy、型、2 種類のテスト（約 12 秒） |
| `npm run test` / `npm run test:rust` | フロントエンド（vitest）/ バックエンド（cargo test） |
| `npm run build` | 型チェックの後にフロントエンドをビルド |
| `npm run tauri build` | アプリをコンパイルし、NSIS インストーラーを `src-tauri/target/release/bundle/nsis/` に出力 |

AI エージェントと一緒に開発しますか。まず [AGENTS.md](AGENTS.md) を読んでください。テストでは守りきれない不変条件が書かれています。

## 継続的インテグレーション

pull request ごと、`main` への push ごとに、`windows-latest` 上でローカルと同じゲートを 3 つの並列ジョブで実行します。`web`（整形、lint、型、フロントエンドテスト、ビルド）、`rust`（cargo fmt、clippy、バックエンドテスト、`Cargo.lock` とマニフェストの一致）、`commitlint` です。

`v<version>` タグを push するとリリースが始まります。タグと `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` の 3 か所のバージョンが一致しない場合はビルドを拒否します。通過すると未署名インストーラーをビルドし、draft release に置いて人の確認を待ちます。

コントリビューションを歓迎します。[CONTRIBUTING.md](CONTRIBUTING.md) をご覧ください。

## ライセンス

[GPL-3.0-or-later](LICENSE) で公開しています。
