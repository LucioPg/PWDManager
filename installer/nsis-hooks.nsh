; PWDManager NSIS Installer Hooks
;
; These macros are called inside Section Install of the NSIS installer.
; nsDialogs pages (Disclaimer, Recovery Key) are defined in custom-installer.nsi
; as proper Page custom declarations, where nsDialogs works correctly.
;
; Supported macro names:
;   NSIS_HOOK_PREINSTALL   — runs before file copy, registry, shortcuts
;   NSIS_HOOK_POSTINSTALL  — runs after file copy, registry, shortcuts

!macro NSIS_HOOK_PREINSTALL
  ; Disclaimer UI is handled by PageDisclaimer custom page in custom-installer.nsi
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Ensure working directory is the install folder
  ; (get_db_path() in Rust uses std::env::current_dir())
  SetOutPath $INSTDIR

  ; $IsUpdate is set in .onInit (before any page is shown)
  ${If} $IsUpdate == "1"
    ; Update mode — skip setup, launch the updated app
    Exec '"$INSTDIR\PWDManager.exe"'
  ${Else}
    ; Fresh install — warn user before creating new keyring key
    ${If} ${FileExists} "$INSTDIR\database.db"
      MessageBox MB_YESNO|MB_ICONEXCLAMATION \
        "An existing PWDManager installation was found.$\n$\n\
        Reinstalling will create a NEW encryption key.$\n\
        Your existing passwords will NO LONGER be accessible.$\n$\n\
        Do you want to continue?" \
        IDYES DoSetup
      StrCpy $SetupFailed "1"
      Abort "Installation cancelled by user"
      DoSetup:
      ; Remove old database files so --setup starts fresh
      Delete "$INSTDIR\database.db"
      Delete "$INSTDIR\database.db-shm"
      Delete "$INSTDIR\database.db-wal"
      Delete "$INSTDIR\database.db.salt"
      Delete "$INSTDIR\recovery_key.txt"
    ${EndIf}

    nsExec::ExecToStack '"$INSTDIR\PWDManager.exe" --setup'
    Pop $R0
    Pop $R1

    ${If} $R0 != 0
      StrCpy $SetupFailed "1"
      MessageBox MB_OK "Database setup failed. Exit: $R0 Output: $R1"
      Abort "Database setup failed"
    ${EndIf}

    StrCpy $RecoveryKey $R1
  ${EndIf}
!macroend
