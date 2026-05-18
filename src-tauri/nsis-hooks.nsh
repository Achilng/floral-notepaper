!macro NSIS_HOOK_PREINSTALL
  StrLen $R0 "\${PRODUCTNAME}"
  IntOp $R0 0 - $R0
  StrCpy $R1 "$INSTDIR" "" $R0
  StrCmp $R1 "\${PRODUCTNAME}" +2
    StrCpy $INSTDIR "$INSTDIR\${PRODUCTNAME}"
!macroend
