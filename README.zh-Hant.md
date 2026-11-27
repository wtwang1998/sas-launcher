# SAS Launcher

在同一台機器上並行執行多個 SAS 9.4 專案，每個專案擁有自己的工作階段環境。

![平台](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%20x64-0078D4)
![最新版本](https://img.shields.io/github/v/release/wtwang1998/sas-launcher?include_prereleases)
![下載次數](https://img.shields.io/github/downloads/wtwang1998/sas-launcher/total)
![授權條款](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)
[![CI](https://github.com/wtwang1998/sas-launcher/actions/workflows/ci.yml/badge.svg)](https://github.com/wtwang1998/sas-launcher/actions/workflows/ci.yml)

[English](./README.md) | [简体中文](./README.zh-CN.md) | **繁體中文** | [日本語](./README.ja.md) | [한국어](./README.ko.md) | [Deutsch](./README.de.md) | [Français](./README.fr.md) | [Español](./README.es.md)

## 為什麼

SAS 可以同時開啟多個視窗，真正的限制不是多開，而是這些視窗共用同一份使用者設定資料夾（`SASUSER`）——字型、顏色、視窗狀態與 `PROC TEMPLATE` 的預設樣板儲存區都在這個資料夾裡，而 SAS 對它的更新存取要求獨佔（[RSASUSER 系統選項](https://support.sas.com/documentation/cdl/en/hostwin/69955/HTML/default/n0npwfu9o7rvtun1f8f4ztpug83z.htm)）。

於是第二個視窗退回 Work 邏輯庫裡的副本，外觀全是預設值，變更在結束時就丟失：

> `NOTE: 無法開啟 SASUSER.PROFILE。將改為開啟 WORK.PROFILE。`

產出報表時更直接：專案專屬的 ODS RTF 樣板寫不進該資料夾，`PROC TEMPLATE` 當場報錯，之後只能用預設樣式——那個視窗能執行程式，卻產不出你要的報表。官方的 `ODS PATH` 只處理樣板、不處理偏好設定；「每個專案一個捷徑」確實可行，但專案一多就散成一堆圖示。

SAS Launcher 給每個專案一份自己的組態，並為每次啟動準備獨立、可寫的 `SASUSER` 資料夾，工作階段結束即刪除。多個專案因此可以同時產出報表，互不干擾。

![專案頁：三個專案及其組態標籤](docs/images/overview-projects.png)

## 功能

**三種組態來源**

- 跟隨安裝的預設組態；選用某個出廠 NLS 組態（`zh`、`u8`、`en`……，每個 `nls\\<code>\\sasv9.cfg` 一筆，通常 20 筆以上）；或從其中任一個衍生出使用者組態。
- 使用者組態是一份 3 行的薄殼 `sasv9.cfg`——`-CONFIG` 指向出廠組態檔，加上編碼與地區設定——並可附上它儲存的 4 個 SAS 設定檔。出廠檔案的內容不會被複製，因此 SAS 升級是自動繼承，而不是逐一修補。
- 編碼與地區設定可以在表單裡修改，也可以開啟專用的 SAS 工作階段調整介面偏好，結束後把 catalog 存回該組態。

![組態管理頁：出廠組態列，以及一份帶設定檔的使用者組態](docs/images/overview-configurations.png)

**每個視窗一套環境**

- 每次啟動都有獨立、可寫的 `-SASUSER` 資料夾，工作階段結束即刪除。
- 專案預設帶走介面設定與環境設定——視窗配置、功能鍵、功能表、Explorer 選項與程式庫指派——工作階段結束時把變更收回，無論結束碼是多少。ODS 樣板仍需手動勾選。
- 可以批次啟動常用專案；已在執行的專案會自動跳過。

**專案專屬的啟動程式碼**

- 為專案指定它自己的 autoexec 檔案，每次啟動時 SAS 先執行它，再接受任何輸入：全域變數、原始資料/分析資料/TFL 資料夾、巨集程式庫只需在專案裡設定一次，不必每個工作階段重來。
- 它透過 `-APPEND AUTOEXEC` 附加在站台 autoexec 鏈之後，安裝目錄或公司自己的 `autoexec.sas` 仍然先執行。檔案不存在時直接拒絕啟動，而不是開一個路徑殘缺的工作階段。
- 開啟「向工作階段提供啟動檔路徑」後，指令碼裡可以讀 `&SASLAUNCHER_STARTUP_FILE`。啟動器會在專案檔案之前先跑一段極小的 prelude 定義它：SAS 在視窗工作階段啟動階段會重設 `SAS_EXECFILEPATH`，而非 ASCII 路徑在 UTF-8 工作階段裡也過不了環境變數這一層，所以路徑改用巨集變數交付。只讀 `%sysget(SAS_EXECFILEPATH)` 的指令碼會被拒絕啟動，而不是帶著空路徑進來。
- 已經按 `%sysget(SAS_EXECFILEPATH)` 寫好的初始化指令碼，可以直接從那次拒絕裡修好：按「移轉啟動檔」，啟動器會把已知的指派改寫成 `%symexist` 判斷、把原檔案保留為 `<名稱>.bak`、開啟該開關並重新啟動。檔案裡其他內容一律不動；參照位置不在改寫範圍內的指令碼會被拒絕，而不是改一半。
- 適用範圍：Windows 上的 SAS 9.4 Foundation 工作階段，由本啟動器啟動。指令碼掛在視窗工作階段的 autoexec 鏈上，因此這個選項對 Enterprise Guide、SAS Studio、SAS Grid 以及 Linux/Unix 上的 SAS 都沒有作用。它也不假設專案的目錄層級：路徑由專案以絕對路徑給出，檔名不限，改寫不把檔案解碼成文字，GBK 指令碼改完仍是 GBK。指令碼若從路徑推算專案根目錄，層數是指令碼自己的約定，移轉不動這段推算。
- 移轉只認一種寫法：`%let` 指派，右側整體是 `%sysget(SAS_EXECFILEPATH)`。其他定位方式它既不改也不提示，例如 `&_SASPROGRAMFILE`、`%sysfunc(getoption(sysin))`，或從別的檔案讀進來的路徑。

在指令碼裡搜 `SAS_EXECFILEPATH`、`_SASPROGRAMFILE`、`getoption(sysin)`，就能判斷是哪種情況：

| 啟動指令碼裡的寫法 | 處理方式 |
| --- | --- |
| `%let <名稱> = %sysget(SAS_EXECFILEPATH);` | 按「移轉啟動檔」 |
| `&_SASPROGRAMFILE`、`%sysfunc(getoption(sysin))`、路徑從另一個檔案讀入 | 照下面的樣式手動修改 |
| 寫死的路徑，或指令碼不定位自己 | 不用改 |

這幾種情況最終都是同一段程式碼，移轉按鈕寫出的就是它。`&SASLAUNCHER_STARTUP_FILE` 的值是啟動檔本身的路徑，等於在工作階段裡提交一個程式時 `SAS_EXECFILEPATH` 的值：

```sas
%if %symexist(SASLAUNCHER_STARTUP_FILE) %then %do;
    %let <name> = &SASLAUNCHER_STARTUP_FILE;
%end;
%else %do;
    %let <name> = %sysget(SAS_EXECFILEPATH);  /* the original fallback */
%end;
```


**可據以排查的診斷**

- 每筆記錄保留解析鏈（薄殼 cfg → 出廠 cfg → 生效的編碼、地區設定與 `SASUSER`）、真實啟動參數、失敗的階段與結束碼。
- SAS 日誌透過 `-ALTLOG` 擷取，寫入的是副本：Log 視窗照舊可用。只保留最近 20 筆。

![工作階段診斷頁：解析鏈與日誌摘要](docs/images/overview-diagnostics.png)

## 系統需求

> [!IMPORTANT]
> 本機必須已安裝並授權 SAS Foundation 9.4。SAS Launcher 只負責啟動它，不附帶 SAS 本體。

- Windows 10 或 11，x64
- SAS Foundation 9.4（已在 TS1M7 驗證）
- WebView2 執行階段——Windows 11 內建；Windows 10 可能需安裝 Evergreen Runtime。原生浮動捲軸需要 125 或更新版本，舊版會退回傳統捲軸

## 安裝

1. 從 [Releases](https://github.com/wtwang1998/sas-launcher/releases) 下載 `SAS-Launcher-0.4.0-x64-Setup.exe`。
2. 安裝程式只裝給目前使用者，落在 `%LOCALAPPDATA%\\SAS Launcher`，不需要系統管理員權限。
3. 升級與解除安裝沿用該資料夾。

> [!NOTE]
> 安裝程式未做程式碼簽章，Windows SmartScreen 會提示「Windows 已保護您的電腦」。這是未簽章組建的正常現象：選擇**更多資訊** → **仍要執行**。請只從本儲存庫的 Releases 頁面下載。

### 資料與解除安裝

專案、組態與診斷記錄保存在 `%LOCALAPPDATA%\\com.saslauncher.desktop`，與程式裝在哪裡無關。解除安裝只刪除程式檔案，資料會保留；想從乾淨狀態開始，手動刪除該資料夾即可。

## 設定

**設定**集中了應用程式層級的開關，全部立即生效，沒有儲存按鈕。

- **主題**——跟隨系統，或強制深色/淺色。每個色值都是一對 `light-dark()`，標題列、原生下拉選單與捲軸會一起跟隨；這需要較新的 WebView2 執行階段。
- **應用程式語言**——简体中文、繁體中文、English、日本語、한국어、Deutsch、Français、Español，或跟隨系統。安裝程式另有 23 種介面語言，並會記住你的選擇。
- **SAS 程式路徑**——顯示偵測到的 `sas.exe`；可以手動輸入、瀏覽，或用「自動偵測」放棄手動指定並重新掃描。
- **登入時自動啟動**——寫入目前使用者的 `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run`，路徑含空格也能正常啟動。
- **啟動時最小化到系統匣**——只有登入自啟那一次會靜默啟動；開啟「登入時自動啟動」後這一列才出現，關閉自啟時它的值仍然保留，重新開啟即恢復原行為。
- **關閉時最小化到系統匣**——預設開啟：關閉視窗只是隱藏（系統匣圖示提供「顯示視窗」與「結束」）。關掉後關閉即結束，但仍有 SAS 工作階段在執行時會先確認；再次啟動程式會把隱藏的視窗喚回。

## 快速開始


1. 啟動應用程式，它會自動找到已安裝的 SAS 9.4；找不到時，橫幅裡可以手動選擇 `sas.exe`。
2. 在**組態管理**裡瀏覽出廠組態，或衍生一個使用者組態並調整它的編碼與地區設定。
3. 在**專案**裡新增專案：名稱、三選一的組態來源、使用者偏好來源（沿用 `My SAS Files` 裡的介面設定、沿用某個組態儲存的工作階段設定，或乾淨環境）、選用備註，以及選用的啟動程式碼檔案。
4. 需要改某個組態的 SAS 介面偏好時，選「開啟 SAS 調整設定」，改完結束 SAS 再儲存。
5. 啟動不順利時，開啟**工作階段診斷**查看解析鏈、啟動參數與日誌。
6. 在**設定**裡改主題、語言、SAS 程式路徑，以及關閉視窗與登入啟動的行為。

## 已知限制

- SAS **增強編輯器**的字型、顏色與按鍵保存在 Windows 登錄檔（`HKCU`）。SAS 沒有提供改變其位置的選項，因此這部分無法按組態隔離或儲存。這是 SAS 的限制，不是缺少的功能。
- 0.2 之前版本的資料不做移轉：舊資料夾會被封存到應用程式資料夾下的 `_old-<日期>`，應用程式從乾淨狀態開始。

## 從原始碼建置

需要 Rust MSVC 工具鏈（edition 2024）、Visual Studio C++ Build Tools、Node.js 與 WebView2 執行階段。儲存庫用 `rust-toolchain.toml` 固定 Rust 1.99.1、用 `.nvmrc` 固定 Node 24.22.3，`rustup` 與 `nvm` 會自動選用。

```powershell
winget install --id Rustlang.Rustup
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

git clone https://github.com/wtwang1998/sas-launcher.git
cd sas-launcher
npm ci
npm run tauri dev
```

首次編譯 Rust 相依約 3–10 分鐘。改 `src/**` 走熱更新；改 `src-tauri/**` 會重新編譯並重啟視窗。

| 指令 | 用途 |
| --- | --- |
| `npm run check` | 全部門檻：格式化、lint、clippy、型別、兩套測試（約 12 秒） |
| `npm run test` / `npm run test:rust` | 前端（vitest）/ 後端（cargo test） |
| `npm run build` | 型別檢查，然後建置前端 |
| `npm run tauri build` | 編譯應用程式並打包 NSIS 安裝程式到 `src-tauri/target/release/bundle/nsis/` |

用 AI agent 參與開發？先讀 [AGENTS.md](AGENTS.md)：裡面寫的是測試攔不住的那些不變條件。

## 持續整合

每個 pull request 和每次推送到 `main`，都會在 `windows-latest` 上跑與你本機相同的門檻，分三個平行 job：`web`（格式化、lint、型別、前端測試、建置）、`rust`（cargo fmt、clippy、後端測試，以及 `Cargo.lock` 是否與清單一致）、`commitlint`。

推送 `v<version>` tag 會觸發發行流程：tag 與 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 三處版本不一致就拒絕建置；通過後建置未簽章安裝程式，放在 draft release 裡等人審閱。

歡迎貢獻——見 [CONTRIBUTING.md](CONTRIBUTING.md)。

## 授權條款

以 [GPL-3.0-or-later](LICENSE) 發佈。
