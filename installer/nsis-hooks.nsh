; PWDManager NSIS Installer Hooks
; Two-phase approach: privacy notice before install, DB setup after install.

!macro NSIS_HOOK_PRE_INST
    ; TODO: Show privacy notice / acceptance page
    ; If user declines: Abort
    !insertmacro MUI_HEADER_TEXT "Privacy Notice" "Please read and accept"
!macroend

!macro NSIS_HOOK_POST_INST
    ; Run --setup and capture stdout (passphrase)
    nsExec::ExecToStack '"$INSTDIR\pwdmanager.exe" --setup'
    Pop $0  ; exit code
    Pop $1  ; stdout (recovery passphrase)

    ${If} $0 != 0
        MessageBox MB_ICONSTOP "Database setup failed. Installation cannot continue.$\n$\nExit code: $0" /SD IDOK
        Abort "Database setup failed"
    ${EndIf}

    ; TODO: Show passphrase display page with $1
    ; User must click "I have saved the recovery key" to continue
!macroend
