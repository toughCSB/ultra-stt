# 단독키 단축키와 Windows 자동 실행 설정

## 무엇이 달라졌나

- 설정 → 일반 → 단축키에서 `A`, `7`, `/` 같은 키를 Ctrl 없이 지정할 수 있다. 기존 `Ctrl+/` 등의 조합은 그대로 사용할 수 있다.
- F키뿐 아니라 문자·숫자·기호의 단독 입력을 막던 설정 화면의 제한을 제거했다. 한글 입력 상태의 문자키도 물리 키 코드로 인식한다.
- 설정 → 일반에 **자동 실행** 영역을 항상 표시한다. ‘시작 시 자동 실행’을 켜거나 끈 뒤 **저장**해야 적용된다. 고급 설정을 펼칠 필요가 없다.
- Windows 자동 실행이 켜져 있으면 앱 시작 시 현재 실행 파일 경로로 등록을 갱신한다. 공백과 한글이 있는 경로도 따옴표로 감싸 등록한다.

## 원인과 수정 범위

단독키는 Rust 등록기에서 이미 지원했지만 화면에서 문자·숫자·기호를 무시했다. 또한 일부 탐색키와 기호는 프런트엔드 설정 정규화 목록에서 누락되어 있었다. 화면과 저장 변환을 기존 Rust 지원 범위에 맞췄다. 알 수 없는 키는 무시하지 않고 오류를 표시한다.

자동 실행도 기존 Tauri 플러그인과 저장 명령이 있었지만 고급 설정에 숨겨져 있었다. 조사 시 SB의 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\Ultra_STT`는 설치 폴더가 아닌 프로젝트의 `src-tauri\target\release\Ultra_STT.exe`를 가리켰으며 따옴표도 없었다. 이것이 모든 과거 자동 실행 실패를 설명한다고 단정할 수는 없지만, 잘못된 경로가 유지될 수 있는 실제 결함이다.

현재 잠긴 버전의 `auto-launch 0.5.0`은 등록 항목 존재 여부만 검사하고 경로는 비교하지 않는다. 따라서 Windows에서 자동 실행이 켜져 있으면 등록을 갱신하고, 기존 플러그인이 작성한 실행 경로를 인용된 현재 경로로 보정한다. 자동 실행을 끄면 Ultra STT의 등록 항목만 제거한다. 새 패키지를 추가하지 않고 기존 `windows-sys`의 Registry 기능만 사용했다.

자동 실행 변경이나 설정 저장이 실패하면 기존 활성 상태로 되돌리기를 시도하고, 되돌리기까지 실패하면 그 오류도 보고한다. `ULTRA_STT_DATA_DIR`로 분리한 QA 프로필에서는 시스템 자동 실행 설정을 변경하지 않는다. 녹음·STT·AI 모델·선택 조회·UIA 격리 코드는 이번 기능을 위해 변경하지 않았다.

## 단독키의 한계

- ‘비어 있는 키’는 모든 앱의 내부 키 기능을 조사한다는 뜻이 아니다. 앱 내부 기능 간 중복을 거부하고, 저장 시 운영체제 등록에 실패하면 이전 단축키를 복구한다.
- 문자·Space 등을 전역 단축키로 지정하면 평소 타이핑에 영향을 줄 수 있다. 사용하지 않는 F키(예: F8)를 권장한다.
- 운영체제 예약 키와 다른 앱이 이미 전역 등록한 키는 사용할 수 없을 수 있다. F12는 Windows 디버거용 예약 키이다.
- Ctrl·Shift 등의 임의 보조키 단독 지원을 새로 추가한 것은 아니다. 기존 네이티브 Fn/RightAlt 처리도 변경하지 않았다.

## 검증과 적용 상태

- 프런트엔드 전체 436개 테스트 통과. 단독 문자·한글 입력 상태·숫자·기호·탐색키, 중복 거부, 네이티브 등록 오류 표시/기존 설정 복원, 고급 설정 없이 자동 실행 저장, 다국어 키 일치 포함.
- Rust 전체 657개 통과, 기존 3개 ignored. 경로 인용, 활성 등록 갱신, 비활성 설정 유지, 실패 처리, 단독키 저장→등록 변환 포함.
- 프런트엔드 production build 및 lint 통과. Windows release build도 20:13에 통과했다. 실행 파일은 `src-tauri/target/release/Ultra_STT.exe`, SHA256은 `D115B85118D4E9D1168671C4AEC65D4D954750ADED399D561409484B0940F5A2`이다. 기존 원격 서버의 unused 경고와 프런트엔드 chunk 경고는 남아 있으며 이번 작업 범위가 아니다.
- 실제 브라우저의 가상 IPC 설정 화면에서 `R` 단독키 지정과 자동 실행 영역의 상시 표시를 확인했다. 이 화면은 실제 Windows 단축키/자동 실행 등록을 변경하지 않는다.
- 설치된 SB 앱은 아직 교체하지 않았다. SB 설치·짧은 재시작·자동 실행 경로 적용 여부를 사용자에게 질문했으며 답변 전이다. Home/Mac/Android 설치, commit/push/release는 하지 않았다.
- Windows 로그아웃/재로그인에 의한 자동 실행 검사는 수행하지 않았다. 동작 중인 사용자 세션을 끊지 않는다.

## 참고한 공식 문서

- [Tauri Autostart](https://v2.tauri.app/plugin/autostart/)
- [Windows RegisterHotKey](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey)
- [Windows RegSetKeyValueW](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regsetkeyvaluew)
- [Windows RegDeleteKeyValueW](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regdeletekeyvaluew)

## 추가 수정 및 실제 설치 — 20:30~20:32

사용자가 SB·Home 설치와 녹음창 위치 수정을 승인했다. 이전 절의 미설치 상태와 빌드 해시는 아래 결과로 대체된다.

- 녹음/Ask 시작 시 Windows의 **주 디스플레이 작업 영역 하단 중앙**으로 초기화한다. 기존 80 logical px 하단 여백을 유지하며 작업 표시줄을 피한다. 모니터별 배율과 음수 좌표를 반영한다.
- 기존 드래그 동작은 유지한다. 해당 녹음의 전사·다듬기 과정에서는 옮긴 위치를 유지하고 다음 녹음은 기본 위치로 돌아간다. 비동기 준비 단계가 녹음 단계로 빠르게 바뀌어도 초기화 요청이 유실되지 않게 했다. 드래그 해제를 녹음 종료 클릭으로 처리하지 않는 테스트도 추가했다.
- 설치 전 SB 개인용 `server-root.txt` 프로필이 내부에서 `ULTRA_STT_DATA_DIR`를 설정한다는 추가 원인을 확인했다. 기존 환경변수 존재 검사로는 정상 설치까지 시험용으로 오인했다. 프로필 설정 전 외부 override 유무를 `StartupScope`로 보존하여 정상 설치의 자동 실행은 허용하고 실제 QA override만 차단한다.
- 최종 native 658개 통과/기존 3개 ignored. frontend 전체 438개 통과 후 드래그 테스트 1개 추가, 해당 CapsuleFlow 19개 통과. lint와 Windows release build 통과.
- SB와 Home 설치 해시: `29B09EA1B755A0C5287E05CC8F26CF5FEFB2207C01EC411A7FE10280E770F3E1`. 버전 숫자는 1.1.62 유지, 이번 작업은 새 공개 릴리스가 아니다.
- SB PID46904/session1, Home PID17320/session1. SB 원격 서버 17846 재개 및 로컬 STT 준비 완료, Home HTTP pre-warm 완료. 두 설치 모두 실행 지속 확인.
- 설정·프로필 표식 및 DB 이외 프로필 파일을 해시 비교하여 보존 확인. 기존 history 전체 행 비교 결과 SB22건/Home33건 동일. 인증·단축키 `Ctrl+/` 유지.
- 백업: SB `%LOCALAPPDATA%\Ultra_STT-backups\settings-position-20261006-203031`, Home 동일 상위 폴더의 `settings-position-20261006-203123`. 원본 실행 파일, 프로필 파일, DB/WAL 스냅샷과 행 해시 보관.
- SB 자동 실행은 기존 true를 유지하며 인용된 실제 설치 경로로 보정됐다. Home은 기존 false를 유지했다. 켜려면 일반 설정의 ‘시작 시 자동 실행’을 켜고 저장한다. 임의로 사용자 설정을 켜지 않았다.
- Home 실행을 위한 일회성 interactive task는 실행 완료 후 삭제했다. 추가 서비스나 상시 예약 작업은 남기지 않았다. Mac·Android 설치 및 commit/push/release 없음.
- 한계: SB의 설치 앱 홈 화면은 실제 screenshot/AX에서 확인했지만 자동 클릭으로 설정 화면 전환이 되지 않았다. 설치 후 실제 녹음창 드래그·새 녹음 복귀/음성 입력, Windows 재로그인은 사용자 확인이 남아 있다. 유닛/가상 IPC 검증을 실사용 완료로 간주하지 않는다.
- 증거: 부모 프로젝트 `qa/windows-settings-20261006/install-result.json`, `home-install-result.json`, 각 PC 백업 폴더의 `history-proof.json`과 시작 로그.

## 최종 패키지 빌드 재검증 — 21:25

- 프런트엔드 전체 **439개**, Rust 라이브러리 **658개/기존 ignored 3개**, 새 release 실행 파일 대상 UIA helper 통합 **3개** 통과. frontend lint와 production build도 통과했다.
- `npx tauri build --bundles nsis`로 Windows 설치 파일을 생성했다. `src-tauri/target/release/bundle/nsis/Ultra_STT_1.1.62_x64-setup.exe`, SHA256 `4EC5EB5BB0E5EBD0E4078EEF3CDDA8CF915BD5B3A70527394F6F6458FCB44F85`.
- 패키징된 `src-tauri/target/release/Ultra_STT.exe`의 SHA256은 `447AB434A5E2CD9AD30FAEE1C10AA31178D5B813DFE3F2993F3C1F10865B2764`이다. NSIS 패키징은 실행 파일의 bundle type 정보를 수정하므로 이전 `--no-bundle` 설치본과 해시가 다르다. 이 파일로 SB/Home을 재설치하거나 재시작하지 않았다.
- Android 기존 도구로 offline 빌드·앱/QA lint·v2 서명 확인 및 Java **114개** 검사를 통과했다. APK는 `output/android/Ultra-STT-1.1.62-android.10-debug.apk`, SHA256 `C3FAE5107378B2E1EED69B5AE63E73C9D5668B3A3ED3BC480935A5AB31A31EAE`. 기기 검사용 APK 생성과 실제 기기 테스트 실행은 다르며 이번에는 기기 조작을 하지 않았다.
- 기존 compiler/chunk/Android SDK XML·Gradle deprecation 경고는 남아 있다. 빌드 실패는 없었다. Windows 재로그인·사람 음성 입력·Mac 실기 검증을 추가로 수행했다는 뜻은 아니며 새 릴리스도 만들지 않았다.

## v1.1.64 버전 정리 — 2026-10-07

사용자가 커밋·푸시, README 이력과 버전 상승을 요청했다. 별도 Mac 1.1.63 설치와 번호를 구분하기 위해 공통 소스를 1.1.64로 올렸다. npm/Tauri/Cargo와 두 lockfile의 앱 버전을 맞췄으며 의존성 버전은 바꾸지 않았다. Android는 같은 기능의 `1.1.64-android.11`, `versionCode=11`이다.

- 다시 실행한 frontend 439개·ESLint, Rust 658개/기존 ignored 3개, Android Java 114개·앱/QA 빌드·lint·APK v2 서명 검사를 통과했다. 새 Windows release EXE의 UIA helper 통합 3개도 최종 통과했다.
- 통합 검사의 `window=0`은 현재 전경 창이 없는 환경에서 다른 경로로 진행할 수 있어 기대값이 달라졌다. UIA3의 실제 규약은 `0=조회 불가`, `u32::MAX=확인된 빈 선택/조회 금지`이다. 항상 존재하지 않는 창 핸들로 시험 대상을 바꾸고 조회 불가를 검사한다. 중간에 두 응답의 의미를 반대로 가정한 재검사도 실패했으며, 실제 송수신 코드를 확인한 뒤 수정했다. 제품의 선택 보호·프로토콜은 변경하지 않았다.
- Windows NSIS: `src-tauri/target/release/bundle/nsis/Ultra_STT_1.1.64_x64-setup.exe`, SHA256 `26B9BA55E9435B89F1977600E2E2F700A2DC07C4C272D466979A93AFDCBA57F1`.
- Windows 실행 파일: `src-tauri/target/release/Ultra_STT.exe`, 파일/제품 버전 1.1.64, SHA256 `0A738BEC05A32F6B7F343EF582B2430C8A2B91F881B4757937A469A5195077BB`.
- Android APK: `output/android/Ultra-STT-1.1.64-android.11-debug.apk`, SHA256 `B7199F0981C3B2AF1A6813B6AB1C34E75AAD8FC387486AE5361CCACAEF212989`.
- 전체 서식 검사는 통과하지 않았다. Prettier가 33개 파일, rustfmt가 25개 파일의 차이를 보고했다. 상당수는 이번 버전 정리 전부터 있던 코드이며, 무관한 전체 서식 변경이나 CI 설정 완화를 하지 않았다. 따라서 기능 테스트·빌드 성공을 전체 CI 성공으로 표현하지 않는다.
- README의 한국어/영어 변경 이력은 실제 설치본과 새 소스·패키지를 구분한다. 기존 사진·릴리스 다운로드를 유지하고, 새 패키지 재설치·새 태그·GitHub Release 생성은 하지 않는다. Mac 별도 후보의 추가 소스 통합과 잠금 해제 후 실기 검증도 여전히 별도 작업이다.
