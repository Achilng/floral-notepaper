!macro NSIS_HOOK_PREINSTALL
  ; Append product name subdirectory if user selected a generic folder
  StrLen $R0 "\${PRODUCTNAME}"
  IntOp $R0 0 - $R0
  StrCpy $R1 "$INSTDIR" "" $R0
  StrCmp $R1 "\${PRODUCTNAME}" +2
    StrCpy $INSTDIR "$INSTDIR\${PRODUCTNAME}"
  ; Refresh output path so files are extracted to the (possibly updated) $INSTDIR
  SetOutPath $INSTDIR
!macroend
