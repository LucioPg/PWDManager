; PWDManager NSIS Installer Hooks
;
; Supported macro names (defined by tauri-bundler):
;   NSIS_HOOK_PREINSTALL   — runs before file copy, registry, shortcuts
;   NSIS_HOOK_POSTINSTALL  — runs after file copy, registry, shortcuts
;   NSIS_HOOK_PREUNINSTALL — runs before file removal
;   NSIS_HOOK_POSTUNINSTALL — runs after file removal

!macro NSIS_HOOK_PREINSTALL
    MessageBox MB_YESNO|MB_ICONQUESTION "This application is provided 'as is' without warranty. The developer assumes no liability for damages or data loss. Passwords are stored locally with AES-256 encryption. No data is transmitted to any external server. A recovery key will be generated during setup. You are solely responsible for storing it safely. Without it, your data cannot be recovered. Do you accept these terms?" IDYES +2
    Quit
!macroend

!macro NSIS_HOOK_POSTINSTALL
    nsExec::ExecToStack '"$INSTDIR\PWDManager.exe" --setup'
    Pop $R0
    Pop $R1

    ${If} $R0 != 0
        MessageBox MB_ICONSTOP "Database setup failed. Exit code: $R0"
        Quit
    ${EndIf}

    ; Save recovery key to file for copy/paste
    FileOpen $0 "$INSTDIR\recovery_key.txt" w
    FileWrite $0 $R1
    FileClose $0

    MessageBox MB_OK|MB_ICONINFORMATION "Your recovery key has been saved to $INSTDIR\recovery_key.txt - Save it in a safe place. Without it, your data cannot be recovered."
!macroend
