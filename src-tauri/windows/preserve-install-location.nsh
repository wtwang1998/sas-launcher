; Keep the first installation where it is.
;
; Installed-elsewhere is not only an in-place upgrade: pointing the page at a folder that
; holds no copy makes NSIS treat the run as a fresh install, which would leave the old
; files behind and move the uninstall entry to the new folder. Ask the registry instead of
; trusting the update flag.
;
; The copy is chosen at run time from $LANGUAGE instead of LangString: tauri's template
; includes this file before the language table is loaded, and LangString is not valid
; inside a Section at all. English is the fallback, so a new installer language reads a
; sentence rather than an empty string.
!macro NSIS_HOOK_PREINSTALL
  ReadRegStr $0 SHCTX "${MANUPRODUCTKEY}" ""
  ${If} $0 != ""
  ${AndIf} $INSTDIR != $0
    ${If} $LANGUAGE == ${LANG_SIMPCHINESE}
      StrCpy $1 "请使用原来的安装目录："
    ${ElseIf} $LANGUAGE == ${LANG_TRADCHINESE}
      StrCpy $1 "請使用原本的安裝資料夾："
    ${Else}
      StrCpy $1 "Use the original installation folder: "
    ${EndIf}
    ; NSIS has already made the folder the page pointed at, and refusing leaves it empty.
    ; RMDir without /r only removes an empty folder, so an ordinary existing one survives.
    ; The folder is the current output directory, and windows refuses to remove the process
    ; working directory, so step out of it first.
    SetOutPath "$TEMP"
    RMDir "$INSTDIR"
    Abort "$1$0"
  ${EndIf}
!macroend
