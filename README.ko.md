# SAS Launcher

한 대의 컴퓨터에서 여러 SAS 9.4 프로젝트를 동시에 실행하고, 프로젝트마다 독립된 세션 환경을 제공합니다.

![플랫폼](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%20x64-0078D4)
![최신 릴리스](https://img.shields.io/github/v/release/wtwang1998/sas-launcher?include_prereleases)
![다운로드 수](https://img.shields.io/github/downloads/wtwang1998/sas-launcher/total)
![라이선스](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)
[![CI](https://github.com/wtwang1998/sas-launcher/actions/workflows/ci.yml/badge.svg)](https://github.com/wtwang1998/sas-launcher/actions/workflows/ci.yml)

[English](./README.md) | [简体中文](./README.zh-CN.md) | [繁體中文](./README.zh-Hant.md) | [日本語](./README.ja.md) | **한국어** | [Deutsch](./README.de.md) | [Français](./README.fr.md) | [Español](./README.es.md)

## 왜 만들었나

SAS 자체는 창을 여러 개 열 수 있습니다. 문제는 다중 실행이 아니라 모든 창이 같은 사용자 설정 폴더(`SASUSER`)를 공유한다는 점입니다. 글꼴, 색상, 창 상태, `PROC TEMPLATE`의 기본 템플릿 저장소가 이 폴더 하나에 들어 있고, SAS는 이 폴더의 갱신 접근에 배타를 요구합니다([RSASUSER 시스템 옵션](https://support.sas.com/documentation/cdl/en/hostwin/69955/HTML/default/n0npwfu9o7rvtun1f8f4ztpug83z.htm)).

그래서 두 번째 창은 Work 라이브러리의 복사본으로 물러서고, 모양은 모두 기본값이며 변경은 종료할 때 사라집니다.

> `NOTE: SASUSER.PROFILE을 열 수 없습니다. WORK.PROFILE을 엽니다.`

보고서를 내보낼 때는 더 직접적입니다. 프로젝트 전용 ODS RTF 템플릿을 이 폴더에 쓸 수 없어 `PROC TEMPLATE`가 그 자리에서 오류가 나고, 이후에는 기본 스타일만 쓸 수 있습니다. 그 창에서는 프로그램이 돌아가도 필요한 보고서는 나오지 않습니다. 공식 `ODS PATH`는 템플릿만 다루고 기본 설정은 다루지 않습니다. "프로젝트마다 바로 가기 하나"도 가능하지만 프로젝트가 늘면 아이콘이 잔뜩 쌓입니다.

SAS Launcher는 프로젝트마다 자체 구성을 주고, 실행할 때마다 독립적이고 쓰기 가능한 `SASUSER` 폴더를 준비해 세션이 끝나면 삭제합니다. 그래서 여러 프로젝트가 동시에 보고서를 내도 서로 간섭하지 않습니다.

![프로젝트 페이지: 세 개의 프로젝트와 구성 배지](docs/images/overview-projects.png)

## 기능

**세 가지 구성 원본**

- 설치된 기본 구성을 따릅니다. 출하 시 NLS 구성(`zh`, `u8`, `en` 등, `nls\\<code>\\sasv9.cfg`마다 하나, 보통 20개 이상) 중 하나를 고르거나 그중 하나에서 사용자 구성을 파생합니다.
- 사용자 구성은 3줄짜리 얇은 `sasv9.cfg`입니다. `-CONFIG`가 출하 시 구성 파일을 가리키고 인코딩과 로캘을 더하며, 저장해 둔 SAS 설정 파일 4개를 붙일 수 있습니다. 출하 시 파일 내용은 복사되지 않으므로 SAS 업그레이드는 자동으로 상속되고 하나씩 손볼 필요가 없습니다.
- 인코딩과 로캘은 폼에서 바꿀 수 있고, 전용 SAS 세션을 열어 인터페이스 기본 설정을 조정한 뒤 종료할 때 그 catalog를 구성에 저장할 수도 있습니다.

![구성 관리 페이지: 출하 시 구성 행과 설정 파일이 붙은 사용자 구성](docs/images/overview-configurations.png)

**창마다 하나의 환경**

- 실행할 때마다 독립적이고 쓰기 가능한 `-SASUSER` 폴더를 주고 세션이 끝나면 삭제합니다.
- 프로젝트는 기본적으로 인터페이스 설정과 환경 설정을 가져갑니다. 창 배치, 기능 키, 메뉴, Explorer 옵션, 라이브러리 할당 등이며 세션이 끝날 때 종료 코드와 관계없이 변경을 되돌려 받습니다. ODS 템플릿은 직접 선택해야 합니다.
- 자주 쓰는 프로젝트를 한꺼번에 실행할 수 있고, 이미 실행 중인 프로젝트는 자동으로 건너뜁니다.

**프로젝트 전용 시작 코드**

- 프로젝트에 자체 autoexec 파일을 지정할 수 있습니다. 실행할 때마다 SAS가 그것을 먼저 실행한 뒤 입력을 받습니다. 전역 변수, raw/분석 데이터/TFL 폴더, 매크로 라이브러리를 프로젝트에서 한 번만 설정하면 세션마다 되풀이할 필요가 없습니다.
- `-APPEND AUTOEXEC`로 사이트 autoexec 체인 뒤에 붙기 때문에 설치 폴더나 회사 자체 `autoexec.sas`가 먼저 실행됩니다. 파일이 없으면 경로가 빠진 세션을 여는 대신 실행을 거부합니다.
- "세션에 시작 파일 경로 제공"을 켜면 스크립트에서 `&SASLAUNCHER_STARTUP_FILE`을 읽을 수 있습니다. 실행기는 프로젝트 파일 전에 아주 작은 prelude를 실행해 그것을 정의합니다. SAS는 창 세션 시작 단계에서 `SAS_EXECFILEPATH`를 재설정하고, 비 ASCII 경로는 UTF-8 세션에서도 환경 변수 계층을 통과하지 못하므로 경로는 매크로 변수로 전달합니다. `%sysget(SAS_EXECFILEPATH)`만 읽는 스크립트는 빈 경로로 들어오는 대신 실행을 거부합니다.
- 이미 `%sysget(SAS_EXECFILEPATH)`로 작성된 초기화 스크립트는 그 거부에서 바로 고칠 수 있습니다. "시작 파일 마이그레이션"을 누르면 실행기가 알려진 대입을 `%symexist` 판정으로 바꾸고, 원본을 `<이름>.bak`으로 남기고, 그 스위치를 켜고 다시 실행합니다. 파일의 다른 내용은 건드리지 않습니다. 참조 위치가 변환 범위 밖인 스크립트는 절반만 고치는 대신 거부합니다.
- 적용 범위: Windows의 SAS 9.4 Foundation 세션이며 이 실행기로 시작한 경우입니다. 스크립트가 창 세션의 autoexec 체인에 걸리므로 Enterprise Guide, SAS Studio, SAS Grid, Linux/Unix의 SAS에는 효과가 없습니다. 프로젝트의 디렉터리 계층도 가정하지 않습니다. 경로는 프로젝트가 절대 경로로 주고 파일 이름은 자유이며, 변환은 파일을 텍스트로 디코딩하지 않으므로 GBK 스크립트는 변환 후에도 GBK입니다. 스크립트가 경로에서 프로젝트 루트를 추정한다면 그 단계 수는 스크립트 자체의 약속이고 마이그레이션은 그 추정을 건드리지 않습니다.
- 마이그레이션이 인식하는 표기는 하나뿐입니다. `%let` 대입이고 오른쪽 전체가 `%sysget(SAS_EXECFILEPATH)`인 경우입니다. `&_SASPROGRAMFILE`, `%sysfunc(getoption(sysin))`, 다른 파일에서 읽어 온 경로 같은 다른 방식은 고치지도 알리지도 않습니다.

스크립트에서 `SAS_EXECFILEPATH`, `_SASPROGRAMFILE`, `getoption(sysin)`을 검색하면 어느 경우인지 알 수 있습니다.

| 시작 스크립트의 표기 | 처리 |
| --- | --- |
| `%let <이름> = %sysget(SAS_EXECFILEPATH);` | "시작 파일 마이그레이션" 클릭 |
| `&_SASPROGRAMFILE`, `%sysfunc(getoption(sysin))`, 다른 파일에서 읽은 경로 | 아래 형태로 직접 수정 |
| 경로를 하드코딩했거나 스크립트가 자신을 찾지 않음 | 변경 불필요 |

모든 경우 결국 같은 코드가 되고, 마이그레이션 버튼이 써 내는 것도 그것입니다. `&SASLAUNCHER_STARTUP_FILE`의 값은 시작 파일 자체의 경로이며, 세션에서 프로그램을 제출할 때의 `SAS_EXECFILEPATH` 값과 같습니다.

```sas
%if %symexist(SASLAUNCHER_STARTUP_FILE) %then %do;
    %let <name> = &SASLAUNCHER_STARTUP_FILE;
%end;
%else %do;
    %let <name> = %sysget(SAS_EXECFILEPATH);  /* the original fallback */
%end;
```


**원인을 추적할 수 있는 진단**

- 각 기록은 해석 체인(얇은 cfg → 출하 시 cfg → 실제 적용된 인코딩, 로캘, `SASUSER`), 실제 시작 인수, 실패한 단계와 종료 코드를 보관합니다.
- SAS 로그는 `-ALTLOG`로 수집하며 기록되는 것은 복사본입니다. Log 창은 그대로 쓸 수 있습니다. 최근 20개만 보관합니다.

![세션 진단 페이지: 해석 체인과 로그 요약](docs/images/overview-diagnostics.png)

## 요구 사항

> [!IMPORTANT]
> 이 컴퓨터에 SAS Foundation 9.4가 설치되어 있고 라이선스가 있어야 합니다. SAS Launcher는 실행만 담당하며 SAS 본체를 포함하지 않습니다.

- Windows 10 또는 11, x64
- SAS Foundation 9.4(TS1M7에서 검증)
- WebView2 런타임. Windows 11은 기본 포함, Windows 10은 Evergreen Runtime 설치가 필요할 수 있습니다. 네이티브 오버레이 스크롤바에는 125 이상이 필요하고 그 이전 버전은 클래식 스크롤바로 돌아갑니다

## 설치

1. [Releases](https://github.com/wtwang1998/sas-launcher/releases)에서 `SAS-Launcher-0.5.0-x64-Setup.exe`를 내려받습니다.
2. 설치 프로그램은 현재 사용자에게만 설치하고 `%LOCALAPPDATA%\\SAS Launcher`에 둡니다. 관리자 권한은 필요 없습니다.
3. 업그레이드와 제거도 이 폴더를 사용합니다.

> [!NOTE]
> 설치 프로그램은 코드 서명이 없어 Windows SmartScreen이 "Windows에서 PC를 보호했습니다"를 표시합니다. 이는 서명되지 않은 빌드의 정상 동작입니다. **추가 정보** → **그래도 실행**을 선택하세요. 내려받기는 이 저장소의 Releases 페이지에서만 하세요.

### 데이터 및 제거

프로젝트, 구성, 진단 기록은 `%LOCALAPPDATA%\\com.saslauncher.desktop`에 저장되며 프로그램 설치 위치와 무관합니다. 제거는 프로그램 파일만 지우고 데이터는 남깁니다. 깨끗한 상태에서 시작하려면 그 폴더를 직접 삭제하세요.

## 설정

**설정**에는 앱 수준 스위치가 모여 있고 모두 즉시 적용됩니다. 저장 버튼은 없습니다.

- **테마**——시스템을 따르거나 어둡게/밝게 강제합니다. 색 값마다 `light-dark()` 한 쌍이며 제목 표시줄, 네이티브 드롭다운, 스크롤바가 함께 따라갑니다. 비교적 최신 WebView2 런타임이 필요합니다.
- **앱 언어**——简体中文, 繁體中文, English, 日本語, 한국어, Deutsch, Français, Español 중 선택하거나 시스템을 따릅니다. 설치 프로그램에는 별도로 23개 UI 언어가 있고 선택을 기억합니다.
- **SAS 프로그램 경로**——감지된 `sas.exe`를 표시합니다. 직접 입력, 찾아보기, "자동 감지"로 수동 지정을 포기하고 다시 검색할 수 있습니다.
- **로그인 시 자동 실행**——현재 사용자의 `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run`에 기록하며 경로에 공백이 있어도 정상 실행됩니다.
- **시작 시 트레이로 최소화**——로그인 자동 실행 때만 조용히 시작합니다. "로그인 시 자동 실행"을 켜야 이 행이 나타나고, 자동 실행을 꺼도 값은 유지되며 다시 켜면 원래 동작으로 돌아옵니다.
- **닫을 때 트레이로 최소화**——기본으로 켜져 있습니다. 창을 닫으면 숨기기만 하고 트레이 아이콘에 "창 표시"와 "종료"가 있습니다. 끄면 닫을 때 종료되지만 SAS 세션이 돌고 있으면 먼저 확인합니다. 프로그램을 다시 실행하면 숨겨진 창이 되돌아옵니다.

## 빠른 시작


1. 앱을 실행하면 설치된 SAS 9.4를 자동으로 찾습니다. 찾지 못하면 배너에서 `sas.exe`를 직접 고를 수 있습니다.
2. **구성 관리**에서 출하 시 구성을 살펴보거나 사용자 구성을 파생해 인코딩과 로캘을 조정합니다.
3. **프로젝트**에서 새로 만듭니다. 이름, 세 가지 중 하나인 구성 원본, 사용자 기본 설정 원본(`My SAS Files`의 인터페이스 설정 따르기, 어떤 구성이 저장한 세션 설정 따르기, 깨끗한 환경), 선택 메모, 선택 시작 코드 파일.
4. 어떤 구성의 SAS 인터페이스 기본 설정을 바꾸려면 "SAS를 열어 설정 조정"을 선택하고, 끝난 뒤 SAS를 종료하고 저장합니다.
5. 실행이 매끄럽지 않으면 **세션 진단**을 열어 해석 체인, 시작 인수, 로그를 봅니다.
6. **설정**에서 테마, 언어, SAS 프로그램 경로, 닫을 때와 시작할 때의 동작을 바꿉니다.

## 알려진 제한

- SAS **확장 편집기**의 글꼴, 색상, 키 할당은 Windows 레지스트리(`HKCU`)에 저장됩니다. SAS가 그 위치를 바꾸는 옵션을 제공하지 않으므로 이 부분은 구성별로 분리하거나 저장할 수 없습니다. 이는 SAS의 제한이며 기능 부족이 아닙니다.
- 0.2 이전 버전의 데이터는 마이그레이션하지 않습니다. 이전 폴더는 앱 데이터 폴더 안의 `_old-<날짜>`로 보관되고 앱은 깨끗한 상태에서 시작합니다.

## 소스에서 빌드

Rust MSVC 도구 체인(edition 2024), Visual Studio C++ Build Tools, Node.js, WebView2 런타임이 필요합니다. 저장소는 `rust-toolchain.toml`로 Rust 1.99.1을, `.nvmrc`로 Node 24.22.3을 고정하며 `rustup`과 `nvm`이 자동으로 선택합니다.

```powershell
winget install --id Rustlang.Rustup
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

git clone https://github.com/wtwang1998/sas-launcher.git
cd sas-launcher
npm ci
npm run tauri dev
```

Rust 의존성의 첫 빌드는 약 3~10분 걸립니다. `src/**` 변경은 핫 리로드, `src-tauri/**` 변경은 다시 컴파일하고 창을 재시작합니다.

| 명령 | 용도 |
| --- | --- |
| `npm run check` | 모든 게이트: 포맷, lint, clippy, 타입, 두 테스트 묶음(약 12초) |
| `npm run test` / `npm run test:rust` | 프런트엔드(vitest) / 백엔드(cargo test) |
| `npm run build` | 타입 검사 후 프런트엔드 빌드 |
| `npm run tauri build` | 앱을 컴파일하고 NSIS 설치 프로그램을 `src-tauri/target/release/bundle/nsis/`에 출력 |

AI 에이전트와 함께 개발하나요. 먼저 [AGENTS.md](AGENTS.md)를 읽으세요. 테스트로 지킬 수 없는 불변 조건이 적혀 있습니다.

## 지속적 통합

모든 pull request와 `main`으로의 push마다 `windows-latest`에서 로컬과 같은 게이트를 세 개의 병렬 job으로 실행합니다. `web`(포맷, lint, 타입, 프런트엔드 테스트, 빌드), `rust`(cargo fmt, clippy, 백엔드 테스트, `Cargo.lock`과 매니페스트 일치), `commitlint`입니다.

`v<version>` 태그를 push하면 릴리스가 시작됩니다. 태그와 `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` 세 곳의 버전이 다르면 빌드를 거부합니다. 통과하면 서명되지 않은 설치 프로그램을 빌드해 draft release에 두고 사람의 검토를 기다립니다.

기여를 환영합니다. [CONTRIBUTING.md](CONTRIBUTING.md)를 보세요.

## 라이선스

[GPL-3.0-or-later](LICENSE)로 배포합니다.
