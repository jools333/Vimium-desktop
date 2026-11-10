; ==============================================================================
; Vimium Desktop NSIS Installer Script
; Produces a modern 64-bit Windows installer with bilingual support (RU/EN)
; ==============================================================================

!ifndef VERSION
  !define VERSION "0.4.0"
!endif

!ifndef SOURCE_EXE
  !define SOURCE_EXE "..\target\release\vimium-desktop.exe"
!endif

!ifndef OUT_FILE
  !define OUT_FILE "..\dist\vimium-desktop-windows-x86_64-setup.exe"
!endif

!define PRODUCT_NAME "Vimium Desktop"
!define PRODUCT_PUBLISHER "jools333"
!define PRODUCT_WEB_SITE "https://github.com/jools333/Vimium-desktop"
!define PRODUCT_DIR_REGKEY "Software\Microsoft\Windows\CurrentVersion\App Paths\vimium-desktop.exe"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\VimiumDesktop"
!define PRODUCT_UNINST_ROOT_KEY "HKCU"
!define PRODUCT_EXE "vimium-desktop.exe"

; Modern UI 2
!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

; Modern Unicode installer
Unicode true

; General Configuration
Name "${PRODUCT_NAME}"
OutFile "${OUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\Vimium-desktop"
InstallDirRegKey ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_DIR_REGKEY}" ""
ShowInstDetails show
ShowUnInstDetails show
RequestExecutionLevel user

; Icons & Visuals
!define MUI_ICON "..\assets\icon.ico"
!define MUI_UNICON "..\assets\icon.ico"
!define MUI_ABORTWARNING

; ------------------------------------------------------------------------------
; Interface Settings & Pages
; ------------------------------------------------------------------------------

; Welcome page
!insertmacro MUI_PAGE_WELCOME

; Directory selection page
!insertmacro MUI_PAGE_DIRECTORY

; Components / Tasks (Shortcuts & Autostart)
!define MUI_PAGE_CUSTOMFUNCTION_SHOW PreComponents
!insertmacro MUI_PAGE_COMPONENTS

; Instfiles page
!insertmacro MUI_PAGE_INSTFILES

; Finish page with option to run the app
!define MUI_FINISHPAGE_RUN "$INSTDIR\${PRODUCT_EXE}"
!define MUI_FINISHPAGE_RUN_NOTCHECKED
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; ------------------------------------------------------------------------------
; Languages
; ------------------------------------------------------------------------------
!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "English"

; Localized Strings
LangString MSG_CLOSE_RUNNING ${LANG_RUSSIAN} "${PRODUCT_NAME} в данный момент запущен.$\nЗакрыть его для продолжения установки?"
LangString MSG_CLOSE_RUNNING ${LANG_ENGLISH} "${PRODUCT_NAME} is currently running.$\nClose it to continue installation?"

LangString SEC_MAIN_TITLE ${LANG_RUSSIAN} "Основные файлы программы"
LangString SEC_MAIN_TITLE ${LANG_ENGLISH} "Core Application Files"

LangString SEC_DESKTOP_TITLE ${LANG_RUSSIAN} "Ярлык на Рабочем столе"
LangString SEC_DESKTOP_TITLE ${LANG_ENGLISH} "Desktop Shortcut"

LangString SEC_AUTOSTART_TITLE ${LANG_RUSSIAN} "Запуск вместе с Windows"
LangString SEC_AUTOSTART_TITLE ${LANG_ENGLISH} "Start with Windows"

LangString UNINST_SUCCESS ${LANG_RUSSIAN} "${PRODUCT_NAME} был успешно удален с вашего компьютера."
LangString UNINST_SUCCESS ${LANG_ENGLISH} "${PRODUCT_NAME} was successfully uninstalled from your computer."

; ------------------------------------------------------------------------------
; Process check & termination helper
; ------------------------------------------------------------------------------
Function CheckAndCloseApp
  CheckLoop:
    nsExec::Exec 'cmd.exe /c "tasklist /FI \"IMAGENAME eq ${PRODUCT_EXE}\" | findstr /I \"${PRODUCT_EXE}\""'
    Pop $0
    ${If} $0 == 0
      MessageBox MB_YESNO|MB_ICONQUESTION $(MSG_CLOSE_RUNNING) IDYES DoKill IDNO DoAbort
      DoKill:
        nsExec::Exec 'taskkill /F /IM ${PRODUCT_EXE}'
        Sleep 800
        Goto CheckLoop
      DoAbort:
        Abort
    ${EndIf}
FunctionEnd

Function .onInit
  ; Detect language
  !insertmacro MUI_LANGDLL_DISPLAY
  Call CheckAndCloseApp
FunctionEnd

Function PreComponents
FunctionEnd

; ------------------------------------------------------------------------------
; Installer Sections
; ------------------------------------------------------------------------------

Section $(SEC_MAIN_TITLE) SecCore
  SectionIn RO
  SetOutPath "$INSTDIR"
  SetOverwrite on

  ; Install main executable
  File /oname=${PRODUCT_EXE} "${SOURCE_EXE}"

  ; Write uninstaller
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  ; Start Menu Shortcuts
  CreateDirectory "$SMPROGRAMS\${PRODUCT_NAME}"
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk" "$INSTDIR\${PRODUCT_EXE}" "" "$INSTDIR\${PRODUCT_EXE}" 0
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\Uninstall ${PRODUCT_NAME}.lnk" "$INSTDIR\Uninstall.exe"

  ; Register in Windows Registry (App Paths)
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_DIR_REGKEY}" "" "$INSTDIR\${PRODUCT_EXE}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_DIR_REGKEY}" "Path" "$INSTDIR"

  ; Register in Windows Add/Remove Programs (Programs & Features)
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayIcon" "$INSTDIR\${PRODUCT_EXE},0"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegDWORD ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "NoModify" 1
  WriteRegDWORD ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "NoRepair" 1

  ; Compute installed size in KB
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "EstimatedSize" "$0"
SectionEnd

Section $(SEC_DESKTOP_TITLE) SecDesktop
  CreateShortCut "$DESKTOP\${PRODUCT_NAME}.lnk" "$INSTDIR\${PRODUCT_EXE}" "" "$INSTDIR\${PRODUCT_EXE}" 0
SectionEnd

Section /o $(SEC_AUTOSTART_TITLE) SecAutostart
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "VimiumDesktop" '"$INSTDIR\${PRODUCT_EXE}"'
SectionEnd

; Component Descriptions
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecCore} $(SEC_MAIN_TITLE)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} $(SEC_DESKTOP_TITLE)
  !insertmacro MUI_DESCRIPTION_TEXT ${SecAutostart} $(SEC_AUTOSTART_TITLE)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; ------------------------------------------------------------------------------
; Uninstaller Section
; ------------------------------------------------------------------------------

Function un.onInit
  ; Close app if running before uninstalling
  UnCheckLoop:
    nsExec::Exec 'cmd.exe /c "tasklist /FI \"IMAGENAME eq ${PRODUCT_EXE}\" | findstr /I \"${PRODUCT_EXE}\""'
    Pop $0
    ${If} $0 == 0
      nsExec::Exec 'taskkill /F /IM ${PRODUCT_EXE}'
      Sleep 800
      Goto UnCheckLoop
    ${EndIf}
FunctionEnd

Section "Uninstall"
  ; Remove autostart entry if set
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "VimiumDesktop"

  ; Remove shortcuts
  Delete "$DESKTOP\${PRODUCT_NAME}.lnk"
  Delete "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk"
  Delete "$SMPROGRAMS\${PRODUCT_NAME}\Uninstall ${PRODUCT_NAME}.lnk"
  RMDir "$SMPROGRAMS\${PRODUCT_NAME}"

  ; Remove registry keys
  DeleteRegKey ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}"
  DeleteRegKey ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_DIR_REGKEY}"

  ; Remove installed files
  Delete "$INSTDIR\${PRODUCT_EXE}"
  Delete "$INSTDIR\Uninstall.exe"
  Delete "$INSTDIR\vimium_config.txt"
  Delete "$INSTDIR\vimium-desktop-update.tmp"
  Delete "$INSTDIR\vimium-desktop.exe.old"

  ; Remove install directory if empty
  RMDir "$INSTDIR"

  SetAutoClose true
SectionEnd
