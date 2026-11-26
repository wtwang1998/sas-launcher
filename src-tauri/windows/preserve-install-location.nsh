; Keep the original directory during in-place upgrades.
;
; The copy is chosen at run time from $LANGUAGE instead of LangString: tauri's template
; includes this file before the language table is loaded, and LangString is not valid
; inside a Section at all. English is the fallback, so a new installer language reads a
; sentence rather than an empty string.
!macro NSIS_HOOK_PREINSTALL
  ${If} $UpdateMode = 1
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
      Abort "$1$0"
    ${EndIf}
  ${EndIf}
!macroend
