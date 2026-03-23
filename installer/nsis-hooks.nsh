; PWDManager NSIS Installer Hooks
;
; Supported macro names (defined by tauri-bundler):
;   NSIS_HOOK_PREINSTALL   — runs before file copy, registry, shortcuts
;   NSIS_HOOK_POSTINSTALL  — runs after file copy, registry, shortcuts
;   NSIS_HOOK_PREUNINSTALL — runs before file removal
;   NSIS_HOOK_POSTUNINSTALL — runs after file removal
;
; Note: nsDialogs::Create 1018 does NOT work inside Section context because
; the dialog template resource is only available in page function context.
; We use MessageBox for dialogs and System::Call for clipboard operations.

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

    ; Save recovery key to file as backup
    FileOpen $0 "$INSTDIR\recovery_key.txt" w
    FileWrite $0 $R1
    FileClose $0

    ; Copy recovery key to clipboard via Win32 API (CF_UNICODETEXT = 13)
    ; TODO: System::Call clipboard copy does not work in practice — the key
    ; is NOT copied to the clipboard. Only the file save (recovery_key.txt) is
    ; reliable. The clipboard approach should be replaced with a proper solution.
    System::Call 'user32::OpenClipboard(p 0)i.r0'
    ${If} $0 <> 0
        System::Call 'user32::EmptyClipboard()'
        StrLen $0 $R1
        IntOp $0 $0 + 1
        IntOp $0 $0 * 2
        System::Call 'kernel32::GlobalAlloc(i 0x0042, i $0)p.r2'
        System::Call 'kernel32::GlobalLock(p $r2)p.r3'
        System::Call 'user32::lstrcpyW(p $r3, t $R1)'
        System::Call 'kernel32::GlobalUnlock(p $r2)'
        System::Call 'user32::SetClipboardData(i 13, p $r2)'
        System::Call 'user32::CloseClipboard()'
    ${EndIf}

    MessageBox MB_OK|MB_ICONINFORMATION "Your recovery key has been copied to the clipboard. Paste it (Ctrl+V) to save it securely. A backup was also saved to $INSTDIR\recovery_key.txt. Without this key, your data cannot be recovered."
!macroend
