[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath (Join-Path $PSScriptRoot '..')

function Invoke-BuildCommand {
    param([string]$Executable, [string[]]$Arguments)
    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$Executable failed with exit code $LASTEXITCODE"
    }
}

$rustVersion = & rustc -vV
if ($LASTEXITCODE -ne 0 -or $rustVersion -notcontains 'host: x86_64-pc-windows-msvc') {
    throw 'Packaging requires the x64 Windows MSVC Rust host toolchain.'
}

$configuration = Get-Content 'src-tauri/tauri.conf.json' -Raw -Encoding UTF8 | ConvertFrom-Json
$metadata = & cargo metadata --no-deps --format-version 1 --manifest-path src-tauri/Cargo.toml | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw 'Could not locate the Cargo target directory.'
}

$tauri = (Resolve-Path -LiteralPath 'node_modules/.bin/tauri.cmd').Path
$bundles = Join-Path $metadata.target_directory 'release/bundle'
$staging = Join-Path $bundles 'windows-payloads'
$distribution = Join-Path $bundles 'windows'
New-Item -ItemType Directory -Force -Path $staging, $distribution | Out-Null

Invoke-BuildCommand $tauri @('build', '--no-bundle')

$makensis = Join-Path $env:LOCALAPPDATA 'tauri/NSIS/makensis.exe'
$generatedDir = Join-Path $metadata.target_directory 'release/nsis/x64'
$userPayload = Join-Path $staging 'current-user.exe'
$machinePayload = Join-Path $staging 'all-users.exe'

function New-ScopePayload {
    param([string]$PayloadPath)

    # Keep Tauri's in-use prompts. The chooser determines the directory,
    # and an /UPDATE always uses in-place maintenance.
    $template = [IO.File]::ReadAllText((Join-Path $generatedDir 'installer.nsi'))
    $template = $template.Replace([string][char]13 + [char]10, [string][char]10)
    $line = [string][char]10
    $maintenance = "Function PageReinstall$line"
    $directory = ('!define MUI_PAGE_CUSTOMFUNCTION_PRE SkipIfPassive' + $line + '!insertmacro MUI_PAGE_DIRECTORY')
    $outputLine = '!define OUTFILE "nsis-output.exe"'
    foreach ($anchor in @($maintenance, $directory, $outputLine)) {
        if (([regex]::Matches($template, [regex]::Escape($anchor))).Count -ne 1) {
            throw 'The Tauri NSIS layout changed. Review the upgrade page overrides.'
        }
    }

    $skipMaintenance = @('  ${If} $UpdateMode = 1', '    Abort', '  ${EndIf}') -join $line
    $template = $template.Replace($maintenance, $maintenance + $skipMaintenance + $line)
    $template = $template.Replace($directory, ('!define MUI_PAGE_CUSTOMFUNCTION_PRE Skip' + $line + '!insertmacro MUI_PAGE_DIRECTORY'))
    $template = $template.Replace($outputLine, ('!define OUTFILE "' + $PayloadPath + '"'))
    $path = Join-Path $generatedDir 'scope-payload.nsi'
    [IO.File]::WriteAllText($path, $template)
    Push-Location -LiteralPath $generatedDir
    try {
        Invoke-BuildCommand $makensis @('/INPUTCHARSET', 'UTF8', '/V2', $path)
    } finally {
        Pop-Location
    }
}

Invoke-BuildCommand $tauri @('bundle', '--bundles', 'nsis')
New-ScopePayload $userPayload

$machineConfig = Join-Path $staging 'all-users.json'
[IO.File]::WriteAllText($machineConfig, '{"bundle":{"windows":{"nsis":{"installMode":"perMachine"}}}}')
Invoke-BuildCommand $tauri @('bundle', '--bundles', 'nsis', '--config', $machineConfig)
New-ScopePayload $machinePayload

$output = Join-Path $distribution ('SAS Launcher {0} x64 Setup.exe' -f $configuration.version)
$source = (Resolve-Path -LiteralPath 'src-tauri/windows/scope-installer.nsi').Path
$icon = (Resolve-Path -LiteralPath 'src-tauri/icons/icon.ico').Path
Invoke-BuildCommand $makensis @(
    '/INPUTCHARSET', 'UTF8', '/V3',
    "/DPRODUCT=$($configuration.productName)",
    '/DPUBLISHER=saslauncher',
    "/DVERSION=$($configuration.version)",
    "/DICON=$icon",
    "/DUSER_PAYLOAD=$userPayload",
    "/DMACHINE_PAYLOAD=$machinePayload",
    "/DOUTPUT=$output",
    $source
)
Get-Item -LiteralPath $output | Select-Object -ExpandProperty FullName
