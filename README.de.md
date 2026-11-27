# SAS Launcher

Mehrere SAS-9.4-Projekte gleichzeitig auf einem Rechner ausführen – jedes mit seiner eigenen Sitzungsumgebung.

![Plattform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011%20x64-0078D4)
![Neueste Version](https://img.shields.io/github/v/release/wtwang1998/sas-launcher?include_prereleases)
![Downloads](https://img.shields.io/github/downloads/wtwang1998/sas-launcher/total)
![Lizenz](https://img.shields.io/badge/license-GPL--3.0--or--later-blue)
[![CI](https://github.com/wtwang1998/sas-launcher/actions/workflows/ci.yml/badge.svg)](https://github.com/wtwang1998/sas-launcher/actions/workflows/ci.yml)

[English](./README.md) | [简体中文](./README.zh-CN.md) | [繁體中文](./README.zh-Hant.md) | [日本語](./README.ja.md) | [한국어](./README.ko.md) | **Deutsch** | [Français](./README.fr.md) | [Español](./README.es.md)

## Warum

SAS kann durchaus mehrere Fenster öffnen. Das Problem ist nicht die Mehrfachausführung, sondern dass alle Fenster dasselbe Benutzereinstellungsverzeichnis (`SASUSER`) teilen. Schriften, Farben, Fensterzustand und der Standardvorlagenspeicher von `PROC TEMPLATE` liegen in diesem einen Verzeichnis, und SAS verlangt für schreibende Zugriffe Exklusivität ([Systemoption RSASUSER](https://support.sas.com/documentation/cdl/en/hostwin/69955/HTML/default/n0npwfu9o7rvtun1f8f4ztpug83z.htm)).

Das zweite Fenster fällt deshalb auf eine Kopie in der Work-Bibliothek zurück, sieht überall nach Standard aus, und Änderungen sind beim Beenden verloren:

> `NOTE: SASUSER.PROFILE kann nicht geöffnet werden. WORK.PROFILE wird geöffnet.`

Beim Berichtsexport wird es direkter: Das projektspezifische ODS-RTF-Template lässt sich nicht in dieses Verzeichnis schreiben, `PROC TEMPLATE` meldet sofort einen Fehler, und danach bleibt nur der Standardstil – das Fenster führt Programme aus, erzeugt aber nicht die gewünschte Tabelle. Das offizielle `ODS PATH` hilft nur bei Templates, nicht bei Präferenzen; „pro Projekt eine Verknüpfung“ funktioniert zwar, wird aber bei vielen Projekten zu einem Haufen Symbole.

SAS Launcher gibt jedem Projekt eine eigene Konfiguration und bereitet für jeden Start ein eigenes, beschreibbares `SASUSER`-Verzeichnis vor, das beim Sitzungsende gelöscht wird. Mehrere Projekte können so gleichzeitig Berichte erzeugen, ohne sich zu stören.

![Projektseite: drei Projekte mit ihren Konfigurations-Chips](docs/images/overview-projects.png)

## Funktionen

**Drei Konfigurationsquellen**

- Der mitgelieferten Standardkonfiguration folgen; eine der ausgelieferten NLS-Konfigurationen wählen (`zh`, `u8`, `en` …, je eine pro `nls\\<code>\\sasv9.cfg`, meist über 20); oder daraus eine Benutzerkonfiguration ableiten.
- Eine Benutzerkonfiguration ist eine dünne `sasv9.cfg` mit drei Zeilen – `-CONFIG` zeigt auf die ausgelieferte Konfigurationsdatei, dazu Kodierung und Locale – und kann die vier gespeicherten SAS-Einstellungsdateien mitführen. Der Inhalt der ausgelieferten Datei wird nicht kopiert, ein SAS-Upgrade wird also automatisch übernommen statt einzeln nachgepflegt.
- Kodierung und Locale lassen sich im Formular ändern, oder man öffnet eine eigene SAS-Sitzung, passt die Oberflächenpräferenzen an und speichert den Katalog beim Beenden in die Konfiguration zurück.

![Konfigurationsseite: Zeile einer ausgelieferten Konfiguration und eine Benutzerkonfiguration mit Einstellungsdateien](docs/images/overview-configurations.png)

**Eine Umgebung pro Fenster**

- Jeder Start erhält ein eigenes, beschreibbares `-SASUSER`-Verzeichnis, das beim Sitzungsende gelöscht wird.
- Projekte nehmen standardmäßig Oberflächen- und Umgebungseinstellungen mit – Fensterlayout, Funktionstasten, Menüs, Explorer-Optionen und Bibliothekszuordnungen – und holen Änderungen beim Beenden zurück, unabhängig vom Exit-Code. ODS-Templates müssen weiterhin manuell ausgewählt werden.
- Häufig genutzte Projekte lassen sich gesammelt starten; bereits laufende Projekte werden übersprungen.

**Projektspezifischer Startcode**

- Ein Projekt kann seine eigene autoexec-Datei angeben. Bei jedem Start führt SAS sie zuerst aus und nimmt danach Eingaben an: globale Variablen, Rohdaten-/Analyse-/TFL-Verzeichnisse und Makrobibliotheken werden einmal im Projekt konfiguriert statt in jeder Sitzung erneut.
- Sie wird über `-APPEND AUTOEXEC` an die Site-autoexec-Kette angehängt, sodass das Installationsverzeichnis oder ein eigenes `autoexec.sas` weiterhin zuerst läuft. Fehlt die Datei, wird der Start abgelehnt, statt eine Sitzung mit unvollständigem Pfad zu öffnen.
- Mit „Der Sitzung den Pfad der Startdatei bereitstellen“ kann ein Skript `&SASLAUNCHER_STARTUP_FILE` lesen. Der Launcher führt vor den Projektdateien einen sehr kleinen Prelude aus, der sie definiert: SAS setzt `SAS_EXECFILEPATH` beim Start einer Fenstersitzung zurück, und Nicht-ASCII-Pfade passieren in einer UTF-8-Sitzung auch die Umgebungsvariablen-Ebene nicht, daher wird der Pfad als Makrovariable übergeben. Skripte, die nur `%sysget(SAS_EXECFILEPATH)` lesen, werden abgelehnt, statt mit leerem Pfad zu starten.
- Bereits nach `%sysget(SAS_EXECFILEPATH)` geschriebene Initialisierungsskripte lassen sich direkt aus dieser Ablehnung heraus reparieren: „Startdatei migrieren“ schreibt die bekannte Zuweisung in eine `%symexist`-Prüfung um, bewahrt die Originaldatei als `<Name>.bak` auf, aktiviert den Schalter und startet neu. Alles andere in der Datei bleibt unangetastet; Skripte mit Bezügen außerhalb des Umbaubereichs werden abgelehnt, statt halb umgeschrieben.
- Geltungsbereich: SAS-9.4-Foundation-Sitzungen unter Windows, die von diesem Launcher gestartet wurden. Das Skript hängt an der autoexec-Kette einer Fenstersitzung und wirkt daher nicht für Enterprise Guide, SAS Studio, SAS Grid oder SAS unter Linux/Unix. Auch eine Verzeichnishierarchie des Projekts wird nicht angenommen: Der Pfad kommt als absoluter Pfad vom Projekt, der Dateiname ist frei, und die Umwandlung dekodiert die Datei nicht als Text – ein GBK-Skript bleibt nach der Umwandlung GBK. Leitet ein Skript die Projektwurzel aus dem Pfad ab, ist die Anzahl der Ebenen seine eigene Vereinbarung; die Migration rührt diese Ableitung nicht an.
- Die Migration kennt genau eine Schreibweise: eine `%let`-Zuweisung, deren rechte Seite vollständig `%sysget(SAS_EXECFILEPATH)` ist. Andere Lokalisierungen ändert und meldet sie nicht, etwa `&_SASPROGRAMFILE`, `%sysfunc(getoption(sysin))` oder einen aus einer anderen Datei gelesenen Pfad.

Sucht man im Skript nach `SAS_EXECFILEPATH`, `_SASPROGRAMFILE` oder `getoption(sysin)`, lässt sich der Fall bestimmen:

| Schreibweise im Startskript | Vorgehen |
| --- | --- |
| `%let <Name> = %sysget(SAS_EXECFILEPATH);` | „Startdatei migrieren“ anklicken |
| `&_SASPROGRAMFILE`, `%sysfunc(getoption(sysin))`, Pfad aus einer anderen Datei | von Hand auf das Muster unten umstellen |
| fest eingetragener Pfad oder Skript lokalisiert sich nicht | keine Änderung nötig |

Alle Fälle enden im selben Code, und genau den schreibt die Schaltfläche. Der Wert von `&SASLAUNCHER_STARTUP_FILE` ist der Pfad der Startdatei selbst und entspricht dem Wert von `SAS_EXECFILEPATH`, wenn in der Sitzung ein Programm eingereicht wird:

```sas
%if %symexist(SASLAUNCHER_STARTUP_FILE) %then %do;
    %let <name> = &SASLAUNCHER_STARTUP_FILE;
%end;
%else %do;
    %let <name> = %sysget(SAS_EXECFILEPATH);  /* the original fallback */
%end;
```


**Diagnose, die zum Nachvollziehen taugt**

- Jeder Datensatz behält die Auflösungskette (dünne cfg → ausgelieferte cfg → tatsächlich wirksame Kodierung, Locale und `SASUSER`), die echten Startargumente, die fehlgeschlagene Phase und den Exit-Code.
- Das SAS-Log wird über `-ALTLOG` mitgeschrieben, und zwar als Kopie: Das Log-Fenster bleibt nutzbar. Aufbewahrt werden die letzten 20 Einträge.

![Sitzungsdiagnose: Auflösungskette und Logauszug](docs/images/overview-diagnostics.png)

## Voraussetzungen

> [!IMPORTANT]
> SAS Foundation 9.4 muss auf dem Rechner installiert und lizenziert sein. SAS Launcher startet es nur und bringt SAS nicht mit.

- Windows 10 oder 11, x64
- SAS Foundation 9.4 (geprüft mit TS1M7)
- WebView2-Runtime – in Windows 11 enthalten; unter Windows 10 kann die Evergreen-Runtime nötig sein. Native Overlay-Scrollbalken benötigen Version 125 oder neuer, ältere Versionen fallen auf klassische Scrollbalken zurück

## Installation

1. `SAS-Launcher-0.4.0-x64-Setup.exe` von den [Releases](https://github.com/wtwang1998/sas-launcher/releases) herunterladen.
2. Das Setup installiert nur für den aktuellen Benutzer nach `%LOCALAPPDATA%\\SAS Launcher`; Administratorrechte sind nicht nötig.
3. Upgrade und Deinstallation nutzen dieses Verzeichnis weiter.

> [!NOTE]
> Das Setup ist nicht codesigniert, deshalb meldet Windows SmartScreen „Windows hat den PC geschützt“. Das ist bei unsignierten Builds normal: **Weitere Informationen** → **Trotzdem ausführen**. Bitte nur von der Releases-Seite dieses Repositorys herunterladen.

### Daten und Deinstallation

Projekte, Konfigurationen und Diagnosedatensätze liegen unter `%LOCALAPPDATA%\\com.saslauncher.desktop`, unabhängig davon, wo das Programm installiert ist. Die Deinstallation entfernt nur die Programmdateien, die Daten bleiben. Wer sauber beginnen will, löscht dieses Verzeichnis von Hand.

## Einstellungen

**Einstellungen** bündelt die Schalter der Anwendung; alle wirken sofort, es gibt keinen Speichern-Knopf.

- **Design** – dem System folgen oder dunkel/hell erzwingen. Jeder Farbwert ist ein `light-dark()`-Paar, Titelleiste, native Dropdowns und Scrollbalken folgen mit; dafür ist eine neuere WebView2-Runtime nötig.
- **App-Sprache** – 简体中文, 繁體中文, English, 日本語, 한국어, Deutsch, Français, Español oder dem System folgen. Das Setup hat davon unabhängig 23 Oberflächensprachen und merkt sich die Auswahl.
- **Pfad zum SAS-Programm** – zeigt das gefundene `sas.exe`; Eingabe von Hand, Durchsuchen oder „Automatisch erkennen“, um die manuelle Angabe aufzugeben und neu zu suchen.
- **Beim Anmelden automatisch starten** – schreibt in `HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run` des aktuellen Benutzers; Pfade mit Leerzeichen starten ebenfalls korrekt.
- **Beim Start in den Infobereich minimieren** – nur der Autostart bei der Anmeldung startet still; die Zeile erscheint erst mit „Beim Anmelden automatisch starten“, ihr Wert bleibt beim Ausschalten des Autostarts erhalten und wirkt beim Wiedereinschalten wieder.
- **Beim Schließen in den Infobereich minimieren** – standardmäßig an: Schließen blendet nur aus, das Symbol im Infobereich bietet „Fenster anzeigen“ und „Beenden“. Ausgeschaltet beendet Schließen die Anwendung, laufen noch SAS-Sitzungen, wird vorher nachgefragt; ein erneuter Start holt das ausgeblendete Fenster zurück.

## Schnellstart


1. Die Anwendung starten; sie findet das installierte SAS 9.4 selbst. Wird es nicht gefunden, lässt sich `sas.exe` im Banner auswählen.
2. Unter **Konfigurationen** die ausgelieferten Konfigurationen durchsehen oder eine Benutzerkonfiguration ableiten und Kodierung sowie Locale anpassen.
3. Unter **Projekte** ein neues Projekt anlegen: Name, eine von drei Konfigurationsquellen, Quelle der Benutzerpräferenzen (Oberflächeneinstellungen aus `My SAS Files` übernehmen, Sitzungseinstellungen einer Konfiguration übernehmen oder saubere Umgebung), optionale Notiz und optionale Startcodedatei.
4. Sollen die SAS-Oberflächenpräferenzen einer Konfiguration geändert werden, „SAS öffnen und Einstellungen anpassen“ wählen, SAS beenden und dann speichern.
5. Läuft ein Start schief, zeigt die **Sitzungsdiagnose** Auflösungskette, Startargumente und Log.
6. Unter **Einstellungen** lassen sich Design, Sprache, Pfad zum SAS-Programm sowie das Verhalten beim Schließen und beim Anmelden ändern.

## Bekannte Einschränkungen

- Schrift, Farben und Tastenzuordnungen des SAS **Enhanced Editor** liegen in der Windows-Registrierung (`HKCU`). SAS bietet keine Option, das zu verlegen, deshalb lässt sich dieser Teil nicht pro Konfiguration trennen oder speichern. Das ist eine Einschränkung von SAS, kein fehlendes Feature.
- Daten aus Versionen vor 0.2 werden nicht migriert: Das alte Verzeichnis wird als `_old-<Datum>` im Datenverzeichnis der Anwendung archiviert, die Anwendung beginnt sauber.

## Aus dem Quellcode bauen

Nötig sind die Rust-MSVC-Toolchain (Edition 2024), Visual Studio C++ Build Tools, Node.js und die WebView2-Runtime. Das Repository legt Rust 1.99.1 über `rust-toolchain.toml` und Node 24.22.3 über `.nvmrc` fest; `rustup` und `nvm` wählen sie automatisch.

```powershell
winget install --id Rustlang.Rustup
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

git clone https://github.com/wtwang1998/sas-launcher.git
cd sas-launcher
npm ci
npm run tauri dev
```

Der erste Build der Rust-Abhängigkeiten dauert etwa 3–10 Minuten. Änderungen an `src/**` laufen per Hot Reload; Änderungen an `src-tauri/**` kompilieren neu und starten das Fenster erneut.

| Befehl | Zweck |
| --- | --- |
| `npm run check` | alle Prüfungen: Format, Lint, Clippy, Typen, beide Testsuiten (etwa 12 Sekunden) |
| `npm run test` / `npm run test:rust` | Frontend (vitest) / Backend (cargo test) |
| `npm run build` | Typprüfung, dann Frontend bauen |
| `npm run tauri build` | Anwendung kompilieren und NSIS-Setup nach `src-tauri/target/release/bundle/nsis/` schreiben |

Mit einem KI-Agenten entwickeln? Zuerst [AGENTS.md](AGENTS.md) lesen: Dort stehen die Invarianten, die Tests nicht abdecken.

## Kontinuierliche Integration

Jeder Pull Request und jeder Push auf `main` führt auf `windows-latest` dieselben Prüfungen wie lokal aus, in drei parallelen Jobs: `web` (Format, Lint, Typen, Frontend-Tests, Build), `rust` (cargo fmt, Clippy, Backend-Tests und ob `Cargo.lock` zum Manifest passt) und `commitlint`.

Ein Push eines `v<version>`-Tags startet die Veröffentlichung: Weicht der Tag von den drei Versionsangaben in `package.json`, `src-tauri/Cargo.toml` und `src-tauri/tauri.conf.json` ab, wird der Build abgelehnt; sonst entsteht ein unsigniertes Setup in einem Draft-Release, das auf menschliche Prüfung wartet.

Beiträge sind willkommen – siehe [CONTRIBUTING.md](CONTRIBUTING.md).

## Lizenz

Veröffentlicht unter [GPL-3.0-or-later](LICENSE).
