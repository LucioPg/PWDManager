; PWDManager Custom NSIS Template
; Based on dioxus native bundler NSIS_TEMPLATE (main branch)
; with nsDialogs pages for Disclaimer and Recovery Key
;
; Handlebars variables are captured via !define and referenced
; with NSIS syntax to avoid backslash-escape issues in paths.

!include "MUI2.nsh"
!include "nsDialogs.nsh"
!include "FileFunc.nsh"
!include "x64.nsh"

; Capture Handlebars variables into NSIS defines and reference with NSIS syntax
!define PRODUCT_NAME "{{product_name}}"
!define VERSION "{{version}}"
!define BUNDLE_ID "{{bundle_id}}"
!define MAIN_BINARY_NAME "{{main_binary_name}}"
!define START_MENU_FOLDER "{{start_menu_folder}}"
{{#if publisher}}
!define PUBLISHER "{{publisher}}"
{{/if}}
{{#if copyright}}
!define COPYRIGHT "{{copyright}}"
{{/if}}

; Basic installer attributes
Name "${PRODUCT_NAME}"
OutFile "{{output_path}}"
Unicode true
{{#if install_mode_per_machine}}
InstallDir "$PROGRAMFILES\${PRODUCT_NAME}"
{{else}}
InstallDir "$LOCALAPPDATA\${PRODUCT_NAME}"
{{/if}}

; Request appropriate privileges
{{#if install_mode_per_machine}}
RequestExecutionLevel admin
{{else if install_mode_both}}
RequestExecutionLevel admin
{{else}}
RequestExecutionLevel user
{{/if}}

; Version information
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${PRODUCT_NAME}"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "FileDescription" "{{short_description}}"
{{#if publisher}}
VIAddVersionKey "CompanyName" "${PUBLISHER}"
{{/if}}
{{#if copyright}}
VIAddVersionKey "LegalCopyright" "${COPYRIGHT}"
{{/if}}

; MUI settings
!define MUI_ABORTWARNING
{{#if installer_icon}}
!define MUI_ICON "{{installer_icon}}"
{{/if}}
{{#if header_image}}
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "{{header_image}}"
{{/if}}
{{#if sidebar_image}}
!define MUI_WELCOMEFINISHPAGE_BITMAP "{{sidebar_image}}"
{{/if}}

; PWDManager custom page variables
Var DisclaimerAccepted
Var RecoveryKey
Var SetupFailed

; Pages
{{#if license}}
!insertmacro MUI_PAGE_LICENSE "{{license}}"
{{/if}}
!insertmacro MUI_PAGE_DIRECTORY

; PWDManager - Disclaimer page (nsDialogs)
Page custom PageDisclaimer PageLeaveDisclaimer

!insertmacro MUI_PAGE_INSTFILES

; PWDManager - Recovery key page (nsDialogs)
Page custom PageRecoveryKey PageLeaveRecoveryKey

!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; Language
!insertmacro MUI_LANGUAGE "English"
{{#each additional_languages}}
!insertmacro MUI_LANGUAGE "{{this}}"
{{/each}}

; Include installer hooks BEFORE the section so macros are defined
{{#if installer_hooks}}
!include "{{installer_hooks}}"
{{/if}}

; Installer section
Section "Install"
    SetOutPath $INSTDIR

    ; Install main binary
    File "{{main_binary_path}}"

    ; Install resources
    ; New bundler stages under resources/ but Dioxus expects assets/ at install time
    CreateDirectory "$INSTDIR\assets"
    {{#each staged_files}}
    SetOutPath "$INSTDIR\assets"
    File "{{this.source}}"
    {{/each}}

    SetOutPath $INSTDIR

    ; Pre-install hook (disclaimer handled by custom page)
    !ifmacrodef NSIS_HOOK_PREINSTALL
        !insertmacro NSIS_HOOK_PREINSTALL
    !endif

    ; Create uninstaller
    WriteUninstaller "$INSTDIR\uninstall.exe"

    ; Create Start Menu shortcuts
    CreateDirectory "$SMPROGRAMS\${START_MENU_FOLDER}"
    CreateShortcut "$SMPROGRAMS\${START_MENU_FOLDER}\${PRODUCT_NAME}.lnk" "$INSTDIR\${MAIN_BINARY_NAME}" "" "$INSTDIR\${MAIN_BINARY_NAME}" 0
    CreateShortcut "$SMPROGRAMS\${START_MENU_FOLDER}\Uninstall ${PRODUCT_NAME}.lnk" "$INSTDIR\uninstall.exe"

    ; Create Desktop shortcut
    CreateShortcut "$DESKTOP\${PRODUCT_NAME}.lnk" "$INSTDIR\${MAIN_BINARY_NAME}" "" "$INSTDIR\${MAIN_BINARY_NAME}" 0

    ; Write registry keys for Add/Remove Programs
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\${BUNDLE_ID}" \
        "DisplayName" "${PRODUCT_NAME}"
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\${BUNDLE_ID}" \
        "UninstallString" '"$INSTDIR\uninstall.exe"'
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\${BUNDLE_ID}" \
        "DisplayVersion" "${VERSION}"
    {{#if publisher}}
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\${BUNDLE_ID}" \
        "Publisher" "${PUBLISHER}"
    {{/if}}
    WriteRegStr SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\${BUNDLE_ID}" \
        "InstallLocation" "$INSTDIR"

    ; Get installed size
    ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
    IntFmt $0 "0x%08X" $0
    WriteRegDWORD SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\${BUNDLE_ID}" \
        "EstimatedSize" "$0"

    ; Post-install hook (runs --setup, stores recovery key)
    !ifmacrodef NSIS_HOOK_POSTINSTALL
        !insertmacro NSIS_HOOK_POSTINSTALL
    !endif

    {{#if install_webview}}
    ; WebView2 installation - skip if already installed
    !define WEBVIEW2APPGUID "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
    ${If} ${RunningX64}
        ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
    ${Else}
        ReadRegStr $0 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
    ${EndIf}
    ${If} $0 == ""
        ReadRegStr $0 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\${WEBVIEW2APPGUID}" "pv"
    ${EndIf}
    ${If} $0 == ""
        {{webview_install_code}}
    ${EndIf}
    !undef WEBVIEW2APPGUID
    {{/if}}

SectionEnd

; Uninstaller section
Section "Uninstall"
    ; Remove files
    RMDir /r "$INSTDIR"

    ; Remove Start Menu items
    RMDir /r "$SMPROGRAMS\${START_MENU_FOLDER}"

    ; Remove Desktop shortcut
    Delete "$DESKTOP\${PRODUCT_NAME}.lnk"

    ; Remove registry keys
    DeleteRegKey SHCTX "Software\Microsoft\Windows\CurrentVersion\Uninstall\${BUNDLE_ID}"
SectionEnd

; ============================================================================
; PWDManager - Disclaimer page (nsDialogs)
; ============================================================================
Function PageDisclaimer
  !insertmacro MUI_HEADER_TEXT "Disclaimer" "Read and accept before installing"
  nsDialogs::Create 1018
  Pop $0
  ${IfThen} $(^RTL) = 1 ${|} nsDialogs::SetRTL $(^RTL) ${|}
  StrCpy $DisclaimerAccepted "0"

  ${NSD_CreateLabel} 0 0 100% 14u "PWDManager - Disclaimer"
  Pop $1

  ${NSD_CreateLabel} 0 18u 100% 24u "This application is provided 'as is' without warranty. The developer assumes no liability for damages or data loss."
  Pop $1

  ${NSD_CreateLabel} 0 46u 100% 24u "Passwords are stored locally with AES-256 encryption. No data is transmitted to any external server."
  Pop $1

  ${NSD_CreateLabel} 0 74u 100% 24u "A recovery key will be generated during setup. You are solely responsible for storing it safely. Without it, your data cannot be recovered."
  Pop $1

  ${NSD_CreateButton} 10u 110u 45% 14u "Accept and Continue"
  Pop $2
  ${NSD_OnClick} $2 DisclaimerAccept

  ${NSD_CreateButton} -200u 110u 45% 14u "Decline"
  Pop $2
  ${NSD_OnClick} $2 DisclaimerDecline

  nsDialogs::Show
FunctionEnd

Function DisclaimerAccept
  StrCpy $DisclaimerAccepted "1"
  SendMessage $HWNDPARENT ${WM_COMMAND} 1 0
FunctionEnd

Function DisclaimerDecline
  StrCpy $DisclaimerAccepted "0"
  SendMessage $HWNDPARENT ${WM_COMMAND} 2 0
FunctionEnd

Function PageLeaveDisclaimer
  ${If} $DisclaimerAccepted == "0"
    Quit
  ${EndIf}
FunctionEnd

; ============================================================================
; PWDManager - Recovery key page (nsDialogs)
; ============================================================================
Function PageRecoveryKey
  ; Skip if setup failed or recovery key is empty
  ${If} $SetupFailed = 1
  ${OrIf} $RecoveryKey == ""
    Abort
  ${EndIf}

  !insertmacro MUI_HEADER_TEXT "Recovery Key" "Save your recovery key"
  nsDialogs::Create 1018
  Pop $0
  ${IfThen} $(^RTL) = 1 ${|} nsDialogs::SetRTL $(^RTL) ${|}

  ${NSD_CreateLabel} 0 0 100% 14u "PWDManager - Save Your Recovery Key"
  Pop $1

  ${NSD_CreateLabel} 0 18u 100% 20u "Copy your recovery key and save it in a safe place:"
  Pop $1

  ${NSD_CreateText} 0 40u 100% 40u ""
  Pop $1
  ${NSD_SetText} $1 $RecoveryKey

  ${NSD_CreateButton} 0 95u 100% 14u "OK - I have saved my recovery key"
  Pop $2
  ${NSD_OnClick} $2 RecoveryKeyOk

  nsDialogs::Show
FunctionEnd

Function RecoveryKeyOk
  SendMessage $HWNDPARENT ${WM_COMMAND} 1 0
FunctionEnd

Function PageLeaveRecoveryKey
  ; Save recovery key to file as backup
  FileOpen $0 "$INSTDIR\recovery_key.txt" w
  FileWrite $0 $RecoveryKey
  FileClose $0
FunctionEnd
