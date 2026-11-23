; NSIS hooks (ADR-0039). Tauri includes this file in its installer script and inserts the
; PREUNINSTALL macro at the start of the uninstall section; MUI2 (included by the same script)
; provides LogicLib, so ${If} ${FileExists} is available.

; "Register webcam for meets" writes per-user COM keys and the MF virtual camera, so the
; uninstaller removes them for the account that runs it (Store policy 10.2.7). The app's own
; command-line mode does the work without opening a window. Its exit code is ignored on
; purpose: a failed cleanup must never block the uninstall.
;
; During an upgrade Tauri runs the previous uninstaller with /UPDATE. Keep the registrations
; then, exactly like the MSI action is limited to a real uninstall: the new version uses them.
!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    DetailPrint "Removing the MeetBurner camera registrations..."
    ${If} ${FileExists} "$INSTDIR\meetburner.exe"
      ExecWait '"$INSTDIR\meetburner.exe" --unregister-cameras' $0
    ${EndIf}
  ${EndIf}
!macroend
