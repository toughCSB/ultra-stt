# Windows 입력 대상 확인 멈춤 방지

## 무엇을 고쳤나요?

음성 결과를 엉뚱한 입력창에 넣지 않으려면, 녹음을 시작한 입력창과 현재 입력창이 같은지 확인해야 합니다. Windows에서는 HWND와 UI Automation(UIA)의 runtime ID를 함께 사용합니다.

기존 구현은 확인 작업을 별도 스레드에 맡기고 150ms만 기다렸습니다. 하지만 **기다리기를 끝내는 것과 실제 작업을 끝내는 것은 다릅니다.** 특정 앱의 UIA 응답이 멈추면 그 스레드가 계속 붙잡히고, 다음 정상 입력창을 확인하는 요청도 처리하지 못할 수 있었습니다.

이 위험은 로컬 `9f480dd`의 입력 대상 보호 코드에서 발견했습니다. 이전 Home 녹음창이 다른 창 뒤에 가려졌던 문제와는 별개이며, 이번 재현은 실제 타 앱을 고장 내지 않는 인위적 지연/멈춤 테스트입니다.

## 바뀐 동작

- UIA 호출을 앱 본체가 아닌 **동일 실행 파일의 숨겨진 보조 프로세스 하나**로 분리했습니다. 새 프로그램·서비스·의존성 설치는 없습니다.
- 보조 모드는 앱 화면, 단축키, 개인 프로필, 음성 서버를 초기화하기 **전에** 분기합니다. 앱을 두 개 실행하거나 로그인을 중복 수행하지 않습니다.
- 정상 상태에서는 프로세스를 재사용합니다. 매번 실행 파일을 띄우는 비용을 내지 않도록 앱 시작 시 미리 준비합니다.
- 호출자의 대기 예산은 여전히 150ms입니다. 실패/시간 초과/프로토콜 오류 시 보조 프로세스의 Job Object를 닫아 종료하고, 250ms의 재시도 간격 뒤 새 보조 프로세스를 준비합니다.
- 시작 준비에는 별도 최대 2초가 허용되지만 앱의 요청자가 2초를 기다리지는 않습니다. 준비 중이거나 입력 대상을 확인하지 못하면 자동 입력을 허용하지 않습니다. 시작 실패가 계속될 때는 다음 요청으로 재시도를 유도하며, 백그라운드 무한 재실행 루프는 없습니다.
- 요청 대기열은 최대 1개입니다. 밀려서 기한이 지난 요청은 UIA에 전달하지 않고 버립니다. 늦은 응답도 입력 허가에 쓰지 않습니다.
- 전달하는 데이터는 프로세스 ID와 최대 64개의 runtime ID 정수뿐입니다. 입력창 본문·비밀번호·음성·인증 정보는 보조 통신에 포함하지 않습니다.
- 기존 HWND/프로세스/포커스 재확인, 제한된 native Edit fallback, 입력 복구 화면, 취소 결과 폐기는 유지합니다. 안전 확인을 끄거나 예전 대상을 재사용하지 않습니다.
- 비정상 SAFEARRAY 범위 계산에 checked arithmetic을 사용해 정수 overflow도 차단했습니다.

## 검증

### 고치기 전

실제 기존 요청 처리 루프에 첫 요청의 600ms 지연을 넣었습니다. 첫 150ms 시간 초과 뒤 두 번째 정상 요청 역시 실패했습니다(`None`, 기대값 `Some([2])`). 기다리기만 중단해서는 다음 요청이 복구되지 않는다는 것을 확인했습니다.

### 고친 뒤

- Windows Rust: **646 passed / 기존 3 ignored**.
- 신규 격리 테스트 8개: 무한 멈춤 후 복구, 충돌/잘못된 길이/부분 응답 후 복구, 정상 프로세스 재사용, 기한 지난 요청 폐기, 동시 16개 요청의 대기 제한, 소유자 해제 시 종료, 실행 파일 부재, 프로토콜 상한.
- 무한 멈춤→복구를 5회 연속 반복하고, 단순 PID 변경뿐 아니라 이전 프로세스의 Windows handle이 종료 상태가 되는 것도 확인했습니다.
- 기존 `LocalProcess` 테스트 **5개 통과**: 자식/후손 종료, 취소, 시간 제한, 출력 상한 등.
- 실제 개발용 앱 실행 파일 통합 검사 **2개 통과**: UIA 전용 실행/EOF 종료/반복 실제 UIA 질의, 잘못된 추가 인수로 본 앱을 열지 않음.
- 해당 개발용 실행 파일에서 시작 준비 666ms, 일치하지 않는 대상 확인 20회 p50 1.97ms / p95 4.33ms / 최대 16.52ms를 관측했습니다. **다른 앱의 실제 편집창이나 음성 전사 속도 벤치마크는 아닙니다.**
- `npx tauri build --no-bundle` 배포용 Windows 빌드 통과. 완성된 `src-tauri/target/release/Ultra_STT.exe`에 같은 통합 검사 2개를 다시 실행해 통과했습니다. GUI subsystem의 실제 실행 파일에서도 파이프 통신이 정상입니다. 시작 준비 365ms, 일치하지 않는 대상 확인 20회 p50 1.43ms / p95 3.54ms / 최대 16.18ms를 관측했습니다.
- 배포용 실행 파일 SHA256: `BEAE9EE161E5251DEDC21A55DF4DAD8B25D1554F747D810DCFC59176568FC97E`. 이 명령은 설치 프로그램을 만들거나 기존 앱을 교체하지 않습니다.
- Frontend **45개 파일 / 423 tests**, production build, lint 통과. 기존 Rust 경고와 Vite chunk/dynamic-import 경고는 남아 있으며 이번 변경의 실패는 아닙니다.

재현 명령(기존 Rust/Visual Studio CMake 환경 사용):

```powershell
cargo test --locked --offline --manifest-path src-tauri/Cargo.toml --lib -- --quiet
cargo build --locked --offline --manifest-path src-tauri/Cargo.toml --example process_fixture
cargo test --locked --offline --manifest-path src-tauri/Cargo.toml --test local_process --test uia_helper -- --nocapture
npm test -- --run
npm run build
npm run lint
```

새 UIA 단위 테스트는 표준 라이브러리만 쓰는 fixture를 현재 Rust compiler로 자동 빌드하므로 별도 example 사전 빌드가 필요하지 않습니다. 기존 `local_process` 통합 테스트의 `process_fixture` 사전 빌드는 그대로 필요합니다.

## 적용 범위와 남은 확인

이 변경은 **Windows UIA 장애 격리**입니다. STT·LLM 모델/프롬프트·음질·모바일·Mac 동작을 바꾸지 않습니다. 모든 앱의 UIA 응답이나 전체 운영체제 장애를 해결하는 기능은 아닙니다. 확인이 계속 불가능한 입력창에서는 자동 입력 대신 기존 복구 경로를 사용해야 합니다.

소스/테스트/배포용 Windows 빌드/실행 파일의 보조 모드까지 확인했습니다. SB/Home 앱 교체·재시작 승인은 질문해 둔 상태이며, 아직 설치하지 않았습니다. SB 실행 중인 기존 앱의 SHA256은 `8A3288848F4765BA44BD1920EF4EAC353BEED9DA5E2E09B55F9FA8634BFC9E7B` 그대로입니다. 새 버전의 실제 음성→커서 입력, 새 보호 기능을 포함한 Home 실사용은 별도 확인 사항입니다. GitHub commit/push/release와 Mac/Android 재설치는 이번 작업에서 수행하지 않았습니다.

## 설계 근거

- Microsoft는 UIA 호출을 창을 소유하지 않는 별도 MTA 스레드에서 수행하도록 안내합니다. 보조 프로세스는 이 조건으로 COM을 초기화하며 UI를 만들지 않습니다: [Understanding Threading Issues](https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-threading).
- 기존 `LocalProcess`의 Windows Job Object와 kill-on-close 소유권을 재사용합니다. 마지막 Job handle 종료로 해당 작업의 프로세스를 종료하는 Windows 동작에 근거합니다: [Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects).

원본 작업: [입력 복구와 대상 보호](2026-10-03-input-recovery-target-protection.md). 보조 프로세스에는 권한 상승, 전역 설정 변경, 외부 네트워크 요청을 추가하지 않았습니다.
