Unicode true
RequestExecutionLevel user
ManifestDPIAware true
SetCompressor /SOLID lzma
Name "${PRODUCT}"
OutFile "${OUTPUT}"
Icon "${ICON}"
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${PRODUCT}"
VIAddVersionKey "FileDescription" "SAS Launcher Setup"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" ""

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "nsDialogs.nsh"
!include "WordFunc.nsh"

!define INSTALL_KEY "Software\${PUBLISHER}\${PRODUCT}"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT}"

Var Dialog
Var UserOption
Var MachineOption
Var Scope
Var InstalledPath
Var InstalledVersion
Var TargetPath
Var Payload
Var PayloadArgs
Var Result
Var LaunchError

Page custom ShowScope InstallSelected
!insertmacro MUI_PAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimpChinese"

LangString scopeTitle ${LANG_ENGLISH} "Choose the installation location"
LangString scopeTitle ${LANG_SIMPCHINESE} "选择安装位置"
LangString scopeHelp ${LANG_ENGLISH} "Choose who can use SAS Launcher on this computer."
LangString scopeHelp ${LANG_SIMPCHINESE} "选择这台电脑上哪些用户可以使用 SAS Launcher。"
LangString userOption ${LANG_ENGLISH} "Install for me only"
LangString userOption ${LANG_SIMPCHINESE} "仅为我安装"
LangString machineOption ${LANG_ENGLISH} "Install for all users on this computer"
LangString machineOption ${LANG_SIMPCHINESE} "为此电脑的所有用户安装"
LangString userHelp ${LANG_ENGLISH} "Administrator permission is not required."
LangString userHelp ${LANG_SIMPCHINESE} "无需管理员权限。"
LangString machineHelp ${LANG_ENGLISH} "Windows will ask for administrator permission after you continue."
LangString machineHelp ${LANG_SIMPCHINESE} "继续后，Windows 会请求管理员权限。"
LangString upgradeTitle ${LANG_ENGLISH} "Use the existing installation"
LangString upgradeTitle ${LANG_SIMPCHINESE} "沿用现有安装"
LangString upgradeHelp ${LANG_ENGLISH} "Setup will use the existing installation scope and folder."
LangString upgradeHelp ${LANG_SIMPCHINESE} "安装程序将沿用原来的安装范围和目录。"
LangString elevationCancelled ${LANG_ENGLISH} "The administrator request was cancelled."
LangString elevationCancelled ${LANG_SIMPCHINESE} "已取消管理员权限请求。"
LangString failedToStart ${LANG_ENGLISH} "Could not start setup (Windows error $LaunchError)."
LangString failedToStart ${LANG_SIMPCHINESE} "无法启动安装程序（Windows 错误 $LaunchError）。"
LangString failedToInstall ${LANG_ENGLISH} "Setup did not complete (exit code $Result)."
LangString failedToInstall ${LANG_SIMPCHINESE} "安装未完成（退出代码 $Result）。"
LangString newerInstalled ${LANG_ENGLISH} "A newer version is already installed."
LangString newerInstalled ${LANG_SIMPCHINESE} "电脑上已安装更新的版本。"

Function .onInit
  SetErrorLevel 1
  SetRegView 64
  SetShellVarContext current

  ; Prefer an existing installation for this Windows user.
  ReadRegStr $InstalledVersion HKCU "${UNINSTALL_KEY}" "DisplayVersion"
  ReadRegStr $InstalledPath HKCU "${INSTALL_KEY}" ""
  StrCpy $Scope "user"
  ${If} $InstalledVersion == ""
  ${OrIf} $InstalledPath == ""
    ReadRegStr $InstalledVersion HKLM "${UNINSTALL_KEY}" "DisplayVersion"
    ReadRegStr $InstalledPath HKLM "${INSTALL_KEY}" ""
    ${If} $InstalledVersion != ""
    ${AndIf} $InstalledPath != ""
      StrCpy $Scope "machine"
    ${Else}
      StrCpy $InstalledVersion ""
      StrCpy $InstalledPath ""
    ${EndIf}
  ${EndIf}

  IfSilent 0 init_done
  Call InstallSelected
  Quit
  init_done:
FunctionEnd

Function ShowScope
  nsDialogs::Create 1018
  Pop $Dialog
  ${If} $Dialog == error
    Abort
  ${EndIf}

  ${If} $InstalledPath == ""
    !insertmacro MUI_HEADER_TEXT "$(scopeTitle)" "$(scopeHelp)"
    ${NSD_CreateRadioButton} 0 7u 100% 13u "$(userOption)"
    Pop $UserOption
    ${NSD_CreateLabel} 14u 23u -14u 22u "$LOCALAPPDATA\${PRODUCT}"
    Pop $0
    ${NSD_CreateLabel} 14u 47u -14u 16u "$(userHelp)"
    Pop $0

    ${NSD_CreateRadioButton} 0 71u 100% 13u "$(machineOption)"
    Pop $MachineOption
    ${NSD_CreateLabel} 14u 87u -14u 22u "$PROGRAMFILES64\${PRODUCT}"
    Pop $0
    ${NSD_CreateLabel} 14u 111u -14u 22u "$(machineHelp)"
    Pop $0
    ${NSD_Check} $UserOption
  ${Else}
    !insertmacro MUI_HEADER_TEXT "$(upgradeTitle)" "$(upgradeHelp)"
    ${If} $Scope == "machine"
      ${NSD_CreateLabel} 0 14u 100% 13u "$(machineOption)"
    ${Else}
      ${NSD_CreateLabel} 0 14u 100% 13u "$(userOption)"
    ${EndIf}
    Pop $0
    ${NSD_CreateLabel} 0 34u 100% 30u "$InstalledPath"
    Pop $0
  ${EndIf}

  GetDlgItem $0 $HWNDPARENT 1
  SendMessage $0 ${WM_SETTEXT} 0 "STR:$(^InstallBtn)"
  nsDialogs::Show
FunctionEnd

Function InstallSelected
  ${If} $InstalledPath != ""
    ${VersionCompare} "$InstalledVersion" "${VERSION}" $0
    ${If} $0 == 1
      MessageBox MB_ICONSTOP "$(newerInstalled)" /SD IDOK
      Abort
    ${EndIf}
  ${Else}
    IfSilent choose_done
    ${NSD_GetState} $MachineOption $0
    ${If} $0 == ${BST_CHECKED}
      StrCpy $Scope "machine"
    ${Else}
      StrCpy $Scope "user"
    ${EndIf}
    choose_done:
  ${EndIf}

  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  ClearErrors
  ${If} $Scope == "machine"
    File /oname=machine-setup.exe "${MACHINE_PAYLOAD}"
    StrCpy $Payload "$PLUGINSDIR\machine-setup.exe"
  ${Else}
    File /oname=user-setup.exe "${USER_PAYLOAD}"
    StrCpy $Payload "$PLUGINSDIR\user-setup.exe"
  ${EndIf}
  ${If} ${Errors}
    StrCpy $LaunchError 0
    MessageBox MB_ICONSTOP "$(failedToStart)" /SD IDOK
    Abort
  ${EndIf}

  StrCpy $PayloadArgs ""
  IfSilent 0 +2
    StrCpy $PayloadArgs "/S"
  ${If} $InstalledPath != ""
    StrCpy $PayloadArgs "$PayloadArgs /UPDATE"
  ${EndIf}

  ; Pass the confirmed location explicitly. The stock payload can otherwise
  ; restore an old directory from a registry key left by an uninstall.
  StrCpy $TargetPath $InstalledPath
  ${If} $TargetPath == ""
    ${If} $Scope == "machine"
      StrCpy $TargetPath "$PROGRAMFILES64\${PRODUCT}"
    ${Else}
      StrCpy $TargetPath "$LOCALAPPDATA\${PRODUCT}"
    ${EndIf}
  ${EndIf}
  ; NSIS requires /D to be the last argument, with its path unquoted.
  StrCpy $PayloadArgs "$PayloadArgs /D=$TargetPath"

  HideWindow
  StrCpy $Result -1
  StrCpy $LaunchError 0
  ${If} $Scope == "machine"
    Call RunElevated
  ${Else}
    ClearErrors
    ExecWait '"$Payload" $PayloadArgs' $Result
    ${If} ${Errors}
      StrCpy $Result -1
    ${EndIf}
  ${EndIf}

  SetOutPath "$TEMP"
  Delete "$Payload"
  ${If} $Result == 0
    SetErrorLevel 0
    Quit
  ${EndIf}

  BringToFront
  ${If} $LaunchError == 1223
    MessageBox MB_ICONINFORMATION "$(elevationCancelled)" /SD IDOK
  ${ElseIf} $Result == -1
    MessageBox MB_ICONSTOP "$(failedToStart)" /SD IDOK
  ${ElseIf} $Result != 1
    ; NSIS uses exit code 1 when the user cancels its wizard.
    MessageBox MB_ICONSTOP "$(failedToInstall)" /SD IDOK
  ${EndIf}
  Abort
FunctionEnd

Function RunElevated
  ; The NSIS process is 32-bit, so SHELLEXECUTEINFOW is 60 bytes.
  ; SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC returns a process handle.
  System::Call '*(&w6 "runas")p.r0'
  System::Call '*(&w${NSIS_MAX_STRLEN} "$Payload")p.r1'
  System::Call '*(&w${NSIS_MAX_STRLEN} "$PayloadArgs")p.r2'
  System::Call '*(i 60,i 0x140,p $HWNDPARENT,p r0,p r1,p r2,p 0,i 1,p 0,p 0,p 0,p 0,i 0,p 0,p 0)p.r3'
  System::Call 'shell32::ShellExecuteExW(p r3)i.r4 ?e'
  Pop $LaunchError
  ${If} $4 <> 0
    StrCpy $LaunchError 0
    System::Call '*$3(i,i,p,p,p,p,p,i,p,p,p,p,i,p,p .r5)'
    ${If} $5 P<> 0
      System::Call 'kernel32::WaitForSingleObject(p r5,i -1)i.r6'
      ${If} $6 == 0
        System::Call 'kernel32::GetExitCodeProcess(p r5,*i .r7)i.r6'
        ${If} $6 <> 0
          StrCpy $Result $7
        ${EndIf}
      ${EndIf}
      System::Call 'kernel32::CloseHandle(p r5)'
    ${EndIf}
  ${EndIf}
  System::Free $3
  System::Free $2
  System::Free $1
  System::Free $0
FunctionEnd

Section
SectionEnd
