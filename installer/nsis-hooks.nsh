; PWDManager NSIS Installer Hooks
;
; These hooks run inside Section Install of the NSIS installer.
; nsDialogs pages (Disclaimer, Recovery Key) are defined in custom-installer.nsi
; as proper Page custom declarations, where nsDialogs works correctly.
;
; Supported macro names (defined by tauri-bundler):
;   NSIS_HOOK_PREINSTALL   — runs before file copy, registry, shortcuts
;   NSIS_HOOK_POSTINSTALL  — runs after file copy, registry, shortcuts

!macro NSIS_HOOK_PREINSTALL
  ; Disclaimer UI is handled by PageDisclaimer custom page in custom-installer.nsi
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Run --setup and store recovery key in global var for PageRecoveryKey page
  StrCpy $SetupFailed "0"
  nsExec::ExecToStack '"$INSTDIR\PWDManager.exe" --setup'
  Pop $R0
  Pop $R1

  ${If} $R0 != 0
    StrCpy $SetupFailed "1"
    Abort "Database setup failed"
  ${EndIf}

  StrCpy $RecoveryKey $R1
!macroend
