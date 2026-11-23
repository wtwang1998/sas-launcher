; Keep the original directory during in-place upgrades.
!macro NSIS_HOOK_PREINSTALL
  ${If} $UpdateMode = 1
    ReadRegStr $0 SHCTX "${MANUPRODUCTKEY}" ""
    ${If} $0 != ""
    ${AndIf} $INSTDIR != $0
      ${If} $LANGUAGE = ${LANG_SIMPCHINESE}
        Abort "请使用原来的安装目录：$0"
      ${Else}
        Abort "Use the original installation folder: $0"
      ${EndIf}
    ${EndIf}
  ${EndIf}
!macroend
