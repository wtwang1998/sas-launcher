; Keep the original directory during in-place upgrades.
;
; The messages are LangString entries rather than a two-way `$LANGUAGE` branch, so every
; installer language reads its own copy. They must stay inside the macro body: the Tauri
; template includes this file before the language table is loaded, so a top-level
; LangString (or ${MANUPRODUCTKEY}) does not compile there.
!macro NSIS_HOOK_PREINSTALL
  LangString LauncherFolderChanged ${LANG_ENGLISH} "Use the original installation folder: $0"
  LangString LauncherFolderChanged ${LANG_SIMPCHINESE} "请使用原来的安装目录：$0"
  LangString LauncherFolderChanged ${LANG_TRADCHINESE} "請使用原本的安裝資料夾：$0"

  ${If} $UpdateMode = 1
    ReadRegStr $0 SHCTX "${MANUPRODUCTKEY}" ""
    ${If} $0 != ""
    ${AndIf} $INSTDIR != $0
      Abort "$(LauncherFolderChanged)"
    ${EndIf}
  ${EndIf}
!macroend
