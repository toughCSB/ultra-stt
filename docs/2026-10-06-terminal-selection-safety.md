# Windows 선택 텍스트 조회와 터미널 중단 방지

## 적용 상태

2026-10-06 SB의 실제 설치 앱에 수정판을 적용하고 기존 사용자 프로필로 다시 실행했다. 설치 위치는 C:\Users\Holyglass.Choi\AppData\Local\Ultra_STT\Ultra_STT.exe, SHA256은 8FE33313DC7EF082849D2A0FCC8E0E4E91CC08308304018D99386A1EA2934FC9 이다. 최종 정상 실행 PID는 36148, UIA 보조 프로세스 PID는 48608이다. 이 값은 검증 시점의 상태다.

Ctrl+/ 호출 단축키, selected_text_enabled=true, output_mode=keyboard, insertion_strategy=auto를 유지했다. settings.json과 server-root.txt의 SHA256이 교체 전과 동일하다. 교체 전 기록 19건의 id/created_at/raw_text/polished_text가 모두 동일하게 남아 있다. 현재 기록은 21건으로, 일반 프로필에서 직접 수행한 마이크 시험 2건만 추가됐다. 별도의 QA 프로필에서 한 입력 시험은 사용자 기록과 분리했다. 인증 파일은 수정하거나 복사하지 않았다.

원본 실행 파일, 설정, marker, SQLite의 일관된 백업은 C:\Users\Holyglass.Choi\AppData\Local\Ultra_STT-backups\ctrl-c-20261006-141432 에 보존했다. 백업의 원본 실행 파일 SHA256은 8A3288848F4765BA44BD1920EF4EAC353BEED9DA5E2E09B55F9FA8634BFC9E7B 이다. rollback은 Ultra STT만 종료한 뒤 이 실행 파일을 설치 위치에 복원하고 다시 실행하는 방법이며, 사용자 데이터 복원은 필요하지 않다.

## 원인과 수정

기존 선택 원문 조회는 설정이 켜져 있으면 녹음 종료 시 전경 창에 Ctrl+C를 보냈다. 선택 영역이 없는 터미널에는 이 입력이 중단 입력으로 전달될 수 있었다. 단축키 변경이나 선택 기능 끄기로 해결하지 않고 해당 자동 입력 경로를 수정했다.

Windows는 격리된 UIA helper의 TextPattern.GetSelection()으로 원문을 우선 읽는다. Win32 Edit/RichEdit와 이를 감싼 WinForms 입력란은 EM_GETSEL/WM_GETTEXT 읽기 메시지를 사용한다. 직접 조회에 성공하면 키 입력과 클립보드 쓰기가 없다. 선택이 없거나 비밀번호 등으로 읽기가 금지됐다고 확인된 경우에도 복사 키를 보내지 않는다.

직접 조회를 지원하지 않거나 조회에 실패한 앱에는 기존 sentinel 기반 클립보드 복사/복원 경로를 유지하되 Ctrl+Insert를 사용한다. Windows 선택 조회에는 Ctrl+C fallback이 없다. 전경 창 변경, 다른 PID, 비밀번호 입력란을 점검하며, 오래된 클립보드 값을 선택 원문으로 받아들이지 않는다. macOS/Linux 단축키는 변경하지 않았다.

UIA3 내부 프로토콜은 '선택 없음 확인'과 'provider 미지원/실패'를 구분한다. 선택 문자열은 최대 65536 UTF-8 바이트, 직접 조회는 최대 16384 UTF-16 단위로 제한하며, 초과 원문을 잘라 사용하지 않는다. 기존 150ms caller deadline, 단일 대기열, helper timeout/crash 시 Job Object 종료와 재시작을 유지했다.

음성 녹음, 전사, 자동 입력, 선택 원문을 이용한 질문/번역/수정 기능은 유지한다. audio/STT/pipeline/output/hotkey 코드는 이번 수정에서 바꾸지 않았다.

## 검증 결과

- 최종 Rust 라이브러리 테스트: 651 통과, 0 실패, 기존 ignored 3건. 선택 조회, 빈 선택, Unicode, 비밀번호/다른 PID, helper 중단·재시작·초과 길이·잘못된 UTF-8, 출력과 voice-intent 회귀가 포함된다.
- 최종 Windows production build 통과. 이 빌드가 실제 설치 파일과 같은 SHA256임을 확인했다.
- 실제 설치 exe 대상 helper 통합 테스트 3개 통과. UIA3 시작, 대상이 없는 선택 조회, 잘못된 helper 인자가 main 앱을 시작하지 않는 동작을 확인했다. helper startup 190ms, unavailable runtime-ID query p95 1.93ms이며 음성 처리 지연 수치가 아니다.
- 실제 selection.rs를 포함한 모의 부작용 probe 통과: 직접 조회/확인된 빈 선택에는 키와 클립보드 쓰기 없음, 미지원 provider에는 Ctrl+Insert/Unicode 원문/클립보드 복원, stale clipboard 거부, 전경 창 변경 시 키 전송 취소. 최초 Ctrl+C 경로와 UIA-only 호환성 손실은 각각 실패로 재현했다.
- 실제 설치 앱과 task-owned WinForms 입력창에서 Ctrl+/로 마이크 녹음 시작/종료, 실제 Whisper large-v3 전사, Codex 정리, 자동 입력을 실행했다. 제어된 시험 음성 '내일 오후 3시에 회의가 있습니다.'가 입력창에 자동으로 들어갔고 status=Inserted, strategy=Keyboard, chars_inserted=19를 확인했다.
- 같은 설치판에서 해당 문장 19글자를 선택하고 기존 번역 단축키 Ctrl+Shift+/로 처리했다. 'There is a meeting tomorrow at 3 p.m.'로 선택 영역이 자동 교체됐고 status=Inserted, chars_inserted=37를 확인했다.
- 최종 두 시험 전후 시험 입력창의 Ctrl+C 수신 횟수는 0으로 유지됐고 Ctrl+Insert 횟수도 3으로 유지됐다(증가 0). 앞선 후보의 fallback 시험에서 생긴 3회이며 최종 직접 조회 시험은 어떤 복사 키도 보내지 않았다.
- 앞선 설치 후보에서도 영어 선택 문장과 한국어 번역 요청을 결합해 번역 답변 창을 만드는 기존 기능을 확인했다. 한국어 dictation 명령은 기존 router의 UnsupportedLocale 경로로 AskSelection을 선택하므로 자동 교체가 아닌 팝업 답변이 정상인 경우가 있다. 전용 번역 단축키의 자동 교체는 최종 설치판에서 별도로 확인했다.
- QA 프로필을 종료하고 임시 override가 없는 새 프로세스로 원래 marker와 프로필을 읽어 정상 실행했다. 원래 설정에서도 Ctrl+/로 실제 마이크 48000Hz/2ch 캡처, 선택 없음 조회, 녹음 종료, 실제 local STT와 자동 입력을 통과했다. 이 무음/잡음 시험은 '시청해주셔서 감사합니다.'를 출력했으므로 사람 음성의 정확도 검증으로 세지 않는다.
- 시험용 proxy/QA 앱/helper/입력창을 종료하고 빈 메모장 시험 탭만 닫았다. 최종 앱과 필요한 local STT 서비스만 유지했다. 임시 17847 서버는 종료됐으며 정상 17846은 PID36148, local STT17845는 PID34268이 소유한다.

## 검증 범위와 남은 확인

자동 Ctrl+C 중단 경로는 제거했고 녹음/전사/자동 입력과 지원되는 입력창의 선택 원문 기능은 설치판에서 검증했다. 제어된 음성 시험은 QA 프로필의 임시 local proxy가 녹음 요청의 오디오를 알려진 WAV로 대체하고, 실제 설치 앱의 인증된 remote endpoint와 실제 managed Whisper로 전사하는 방식이다. 마이크 캡처 자체는 실제였지만 WAV 시험은 사람의 실제 발화를 끝까지 인식한 시험과 다르다.

UIA/native 조회와 Ctrl+Insert를 모두 지원하지 않는 앱의 선택 원문 호환성까지 보장하지 않는다. Herdr 0.9.3의 공식 client/shell/input.rs는 자체 선택 복사에 Ctrl+C/Super+C만 인식한다. PaneSelectionRead API에는 pane ID와 anchor/cursor 좌표가 필요하며 현재 client selection 좌표를 얻는 API는 확인되지 않았다. Windows Terminal의 자체 선택과 Herdr 내부 선택은 다르다. 이번 시험은 사용자 터미널에 키를 자동 입력하거나 세션 종료를 유도하지 않았으므로 해당 Herdr 내부 선택의 실제 호환성은 아직 확인되지 않았다. '모든 앱에서 완벽한 선택 호환성'이나 사람 목소리 인식 정확도까지 완료했다고 보고하지 않는다.

후속 확인 대상은 사용자 지정 Ultra STT 세션 01a0f0d2-986d-7f31-84df-206b63cf2c0a 이다. 사용자가 이후 팔로업을 맡는다. Home/Mac/Android는 이번 작업에서 변경하지 않았다. 기존 Android/UIA 미커밋 작업을 보존했으며 commit/push/release는 하지 않았다.

## 증거 위치

프로젝트 qa/ctrl-c-selection-20261006 아래에 native-tests-final.log, release-build-final.log, release-helper-tests-final.log, selection-probe.log, qa-app-final.log, dictation-result-final.txt, translation-result-final.txt, normal-app-final2.log, preservation-result.json이 있다. QA 파일은 작업 전용이며 공개하지 않았다.

정상 프로필 복원 중 첫 시도는 .NET 환경 변수 삭제 호출이 빈 값을 남겨 'ULTRA_STT_DATA_DIR must be absolute'로 실패했다. 임시 환경 변수가 없는 새 PowerShell 프로세스에서 재실행해 해결했고 정상 시작·실제 입력·프로필 보존을 다시 확인했다. 실패 로그 normal-app-final.err도 보존했으며 앱 소스에 해당 임시 launch 환경 변경을 추가하지 않았다.

참고: [UIA TextPattern](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationclient/nn-uiautomationclient-iuiautomationtextpattern), [Windows Ctrl+C 처리](https://learn.microsoft.com/en-us/windows/console/ctrl-c-and-ctrl-break-signals), [Windows 복사 단축키](https://support.microsoft.com/en-us/accessibility/windows/keyboard-shortcuts-in-windows), [Windows Terminal 복사](https://learn.microsoft.com/en-us/windows/terminal/customize-settings/actions#copy), [Herdr 0.9.3 선택 복사 소스](https://github.com/herdrdev/herdr/blob/v0.9.3/src/client/shell/input.rs).
