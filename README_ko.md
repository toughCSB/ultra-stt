<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" height="96" alt="Ultra STT 녹색 마이크 아이콘" />
</p>

<h1 align="center">Ultra STT</h1>
<p align="center"><strong>말한 그대로, 원하는 문장으로, 지금 커서에.</strong></p>
<p align="center">
  Windows · macOS 개인용 음성 키보드<br />
  로컬 Whisper 음성 인식 + 선택형 AI 다듬기 + 현재 입력창에 바로 입력
</p>
<p align="center"><strong>v1.1.62</strong> · Windows x64 · macOS Apple Silicon · MIT</p>
<p align="center">
  <a href="README.md">English</a> · 한국어 ·
  <a href="docs/FEATURES.md">상세 기능·기술 사양</a> ·
  <a href="docs/VALIDATION.md">검증·성능·한계</a>
</p>

<p align="center">
  <img src="docs/images/ultra-stt-dashboard.png" width="860" alt="Ultra STT 1.1.62 실제 홈 화면: 다섯 가지 다듬기 모드와 날짜별 기록 그래프" />
</p>
<p align="center"><em>2026년 10월 2일, 설치된 Windows 앱에서 촬영했습니다. 현재 선택은 최소 모드입니다.</em></p>

> **현재 상태:** [toughCSB/ultra-stt](https://github.com/toughCSB/ultra-stt)는 사용자 소유의 비공개 저장소입니다. Windows x64와 Apple Silicon Mac에 1.1.62를 빌드·설치했습니다. 이 시점에는 GitHub 다운로드 릴리스를 게시하지 않았습니다. Linux는 기반 코드가 있으나 이번 개인 버전의 검증 대상이 아닙니다. 모바일은 브라우저 녹음·복사·공유 단계이며 네이티브 키보드 앱은 아직 아닙니다.

## 무엇을 하는 앱인가요?

입력창에서 커서를 옮기지 않고 말하세요. 음성을 인식한 뒤 원하는 만큼만 다듬어, 사용 중인 앱에 입력합니다.

- **현재 커서에 입력:** 전역 단축키, 누른 채 말하기·토글 녹음, 키보드 입력·클립보드 출력.
- **내 말에 맞는 다듬기:** 원문 · 최소 · 깔끔하게 · 구조화 · 전문적으로.
- **어디서든 모드 전환:** 트레이 우클릭, 홈 이미지 버튼, 음성 실행창 선택 메뉴.
- **쓸모 있는 기록:** 날짜별 사용량 확인, 날짜 클릭 필터, 원시 전사·최종 문장 검색과 복사.
- **품질을 지키는 최적화:** GPU 모델 예열, 연결 재사용, Codex Fast, 작업별 Medium/High 자동 선택.
- **개발 환경과 분리:** 앱 전용 Codex 지침·로그인으로 개발용 설정을 끌어오지 않습니다.
- **Mac에서도 같은 받아쓰기 모드:** Windows 서버의 GPU·AI를 활용하고 Mac 커서에 입력합니다.

## 사용 순서

1. **설정**에서 음성 인식 제공자, 사용할 마이크와 출력 방식을 선택합니다.
2. AI 다듬기를 사용할지 정하고, **홈**이나 트레이에서 모드를 선택합니다.
3. 문장을 넣을 앱의 입력칸에 커서를 둡니다.
4. 받아쓰기 단축키로 녹음하고, 키를 놓거나 토글 단축키를 다시 눌러 완료합니다.
5. 실행창에서 녹음·전사·다듬기 상태를 확인합니다. 취소는 **Esc 키 또는 Esc 버튼**입니다.
6. 나중에 다시 사용할 문장은 **기록**에서 검색·복사합니다.

단축키는 변경할 수 있으며 기존 설치의 사용자 설정을 보존합니다. 소스의 초기 기본값은 다음과 같습니다.

| 작업 | Windows | macOS |
| --- | --- | --- |
| 받아쓰기 | `Ctrl+/` | `Fn` |
| Ask Anything | `Ctrl+.` | `Fn+Space` |
| 번역 | `Ctrl+Shift+/` | `Fn+LeftShift` |
| 진행 중 녹음·처리 취소 | `Esc` | `Esc` |

Windows 초기값은 누른 채 말하기, Mac은 토글입니다. 작업마다 최대 세 개의 단축키를 둘 수 있습니다. 실제 설치에서 바꾼 단축키는 기본값보다 우선하며, 스크린샷의 키 조합을 그대로 따라야 하는 것은 아닙니다.

## 다섯 가지 AI 다듬기

| 모드 | 어떻게 바꾸나요? | 어울리는 상황 |
| --- | --- | --- |
| **원문 · Original** | 명백한 오인식·오타·띄어쓰기와 무의미한 군더더기만 수정합니다. 단어·순서·말투·강조·기존 줄바꿈을 유지하고 자동 목록·문단 재구성을 하지 않습니다. | 말한 표현을 최대한 그대로 남기기 |
| **최소 · Minimal** | 표현과 순서는 최대한 유지하고 문장부호·자연스러운 문장 구분·명백한 군더더기를 정리합니다. | 가볍게 다듬는 일상 받아쓰기 |
| **깔끔하게 · Clean** | 사실·말투·정보량을 보존하면서 어순과 표현을 조금 정돈합니다. | 메시지·메모 |
| **구조화 · Structured** | 여러 항목을 목록으로 정리하고 필요할 때 짧은 주제 제목을 붙입니다. 새 사실은 추가하지 않습니다. | 할 일·회의 내용 |
| **전문적으로 · Professional** | 불필요한 인사말이나 과장 없이 간결한 업무 문장으로 정리합니다. | 이메일·보고 |

**원문도 AI 처리 모드입니다.** AI를 완전히 건너뛰려면 다듬기를 비활성화하세요. 선택 텍스트 편집·번역·씬·사용자 지침 등 명시적으로 다른 작업을 요청한 경우에는 해당 작업 규칙을 따릅니다.

### 모드 변경은 한 번의 선택으로

- **트레이:** 우클릭 → AI 다듬기 모드 → 원하는 모드.
- **홈:** 그림과 설명이 있는 다섯 버튼 중 하나를 클릭.
- **음성 실행창:** 모드 표시나 마이크 영역에서 선택 메뉴 열기.

초록색 강조와 체크 표시가 현재 선택입니다. 선택은 즉시 저장되고 다른 화면에도 동기화됩니다. 녹음 중 바꾸면 그 녹음이 끝날 때 적용되고, 이미 처리가 시작됐다면 다음 녹음에 적용됩니다.

## 실제 앱 화면

| AI 다듬기 설정 | 단축키·녹음·출력 설정 |
| --- | --- |
| <img src="docs/images/ultra-stt-ai-settings.png" width="440" alt="실제 AI 설정: ChatGPT 구독 Codex CLI, gpt-6-luna, 최소 모드" /> | <img src="docs/images/ultra-stt-shortcuts.png" width="440" alt="실제 일반 설정: 작업별 단축키, 토글 녹음, 키보드 출력" /> |

모두 Windows 설치 앱의 실제 캡처입니다. 앱 밖 영역과 불필요한 여백만 제외했으며, 생성형 이미지로 앱 화면을 꾸며낸 것이 아닙니다. API 키나 개인 전사 내용은 노출하지 않았습니다. 모드 버튼의 그림은 생성형 이미지이고 [제작 기록](docs/polish-mode-assets.md)을 별도로 남겼습니다.

## 기능 전체 보기

| 영역 | 구현된 기능 |
| --- | --- |
| 녹음 | 마이크 선택, 전역 단축키, 누른 채 말하기·토글, 상태별 플로팅 캡슐, 유휴 자동 숨김, 취소 |
| 출력 | 키보드 시뮬레이션, 자동 붙여넣기, 복사 전용, 대상 앱 변경 감지, 클립보드 대체와 경고 |
| 다듬기 | 다섯 스타일, 사용자 지침, 현재 앱에 맞춘 글쓰기, 앱별 매핑 |
| 선택 텍스트 | 선택한 문장에 음성으로 수정·요약·길이 조절·번역 요청 |
| Ask Anything | 한 번의 음성 질문, 플로팅 답변 메모, 복사; 대화 이력을 이어가는 채팅은 아님 |
| 번역 | 전용 단축키와 대상 언어 설정 |
| 사전 | 사용자 용어, 가져오기·내보내기, 활성 교정 규칙, 기록에서 교정 규칙 만들기 |
| 씬 | 기본·사용자 작성 지침과 가져오기·내보내기 |
| 기록 | 로컬 SQLite 저장, 원시·최종 텍스트 검색, 날짜 필터, 복사, 확인 후 전체 비우기, 입력 실패 진단 |
| 홈 | 오늘 건수, 클릭 가능한 최근 14일 그래프, 현재 설정, 마지막 처리 시간 |
| 데스크톱 | 다크·라이트·시스템 테마, 자동 시작, 트레이 클릭으로 창 열기·숨기기, 녹색 마이크 아이콘 |
| 원격 | 비공개 Tailnet 브라우저 녹음 화면과 Mac `ultra-remote` 프리셋 |
| UI 언어 | 10개 언어; 일부 고급 설명은 영어 fallback |

고급 작업은 제공자 구성에 따라 사용할 수 있는 범위가 달라집니다. 현재 Mac 개인 원격 경로는 **다듬기 모드와 AI 활성 여부**를 서버에 전달합니다. Mac의 씬·사전·선택 텍스트·번역 설정까지 모두 전달하는 범용 원격 API는 아닙니다. [상세 범위](docs/FEATURES.md)를 참고하세요.

### 홈의 막대 그래프는 무엇인가요?

**날짜별로 불러온 전사 기록 건수**입니다. 음량 표시나 장식이 아닙니다.

- 최근 14일의 날짜·건수를 보여 주며, 기록이 없는 날도 표시합니다.
- 막대를 클릭하면 해당 날짜의 기록 화면으로 이동합니다.
- 현재 불러온 **최대 200건**이 기준이며 전체 DB 통계인 것처럼 표시하지 않습니다.
- 마지막 처리 시간은 현재 실행 세션의 마지막 작업 기준입니다. 말한 시간은 포함하지 않습니다.
- Mac 원격 경로는 서버 STT와 AI를 합친 원격 처리 시간으로 표시합니다.

## 기술 사양

| 항목 | 구현·구성 |
| --- | --- |
| 앱 쉘 | Tauri 2.11.2, Rust 2021 edition, Rust 최소 1.87 |
| 프런트엔드 | React 19, TypeScript 5.8, Vite 8, Tailwind CSS 4, Zustand 5, i18next |
| 음성 캡처·출력 | `cpal`, Rust 출력 어댑터, Windows SendInput·Mac 네이티브 붙여넣기 |
| 데이터 | SQLite 기록·사전, 로컬 설정, OS credential vault 연동 |
| 개인 STT | `faster-whisper 1.2.1` + `CTranslate2 4.8.1`, Whisper large-v3, CUDA float16, beam size 5, VAD |
| 개인 AI | `codex-cli`, ChatGPT 구독 로그인, `gpt-6-luna` |
| Fast | 이 Codex 어댑터의 모든 요청에서 `service_tier="priority"` 지정 |
| 추론 강도 | 단순·짧은 받아쓰기 Medium, 복잡한 작업·용어 보존이 중요한 작업 High |
| 원격 연결 | Tailscale Serve HTTPS, 동일 소유자 확인, 기존 Bearer 인증 |
| 설치·검증 대상 | Windows x64, macOS arm64 |
| 모바일 | 브라우저 마이크 → 결과 → 복사·공유; 네이티브 키보드는 미구현 |

GPU 경로에는 CUDA를 사용할 수 있는 NVIDIA 환경과 별도 모델 다운로드가 필요합니다. 검증되지 않은 최소 VRAM·RAM 수치를 제시하지 않습니다. Apple Speech, Whisper 호환 서버, Deepgram, AssemblyAI, Aliyun Qwen3, Volcengine 등의 통합 코드도 있지만, 제공자별 인증·플랫폼 지원이 필요하며 모두 실사용 검증한 것은 아닙니다.

## 속도 최적화 원칙

품질을 낮추기 위해 작은 STT 모델로 바꾸거나 AI를 몰래 생략하지 않습니다.

1. **GPU 모델 예열·재사용:** 시작 때 준비하고 성공한 전사 뒤 모델을 유지합니다.
2. **불필요한 호출 제거:** HTTP 연결을 재사용하고, 서버가 다듬은 Mac 결과를 다시 AI에 보내지 않습니다.
3. **Codex 완전 분리:** 앱 전용 `CODEX_HOME`과 짧은 앱 전용 지침을 사용합니다. 개발용 설정·인증 파일을 복사하지 않습니다.
4. **Fast + 조건부 추론:** Original/Minimal/Clean의 1,024자 이하 일반 받아쓰기는 추가 컨텍스트가 없을 때 Medium입니다. 선택 텍스트·번역·씬·사용자 지침·사전·활성 교정 규칙이 있거나 복잡한 스타일이면 High를 유지합니다.
5. **취소 전파:** 실행창에서 취소하면 소유한 처리 작업을 종료하고, 원격 클라이언트 연결이 끊기면 서버 작업도 중단합니다.

**현재도 클라우드 AI 처리에는 수 초가 걸립니다.** Fast 지정이 일정한 지연을 보장하지 않으며 구독 사용량 제한도 그대로 적용됩니다. 입력 토큰 절약률과 구독 사용량 절약률은 같지 않습니다. Typeless 동급 속도는 입증하지 않았습니다. [실측·검증 범위](docs/VALIDATION.md)에 측정값과 한계를 함께 정리했습니다.

## 소스에서 실행·빌드

비공개 저장소에 접근할 수 있는 계정으로 복제합니다.

```sh
git clone https://github.com/toughCSB/ultra-stt.git
cd ultra-stt
npm ci
npm run tauri dev
```

### 개발 환경

- Node: 20.x 사용 시 **20.19 이상**, 또는 **22.12 이상**.
- Rust: stable **1.87 이상**.
- Windows: Visual Studio C++ Build Tools, WebView2.
- macOS: Xcode Command Line Tools.
- 개인 CUDA 서버: Python **3.11**, `local_stt/pyproject.toml`의 의존성, NVIDIA CUDA 환경과 Whisper 모델.

일반 앱 번들:

```sh
npm run tauri build
```

산출물은 `src-tauri/target/release/bundle/`에 생성됩니다. 저장소에는 모델·Python 가상환경·인증·개인 설정을 넣지 않습니다.

### 준비된 개인 환경에 설치

Windows helper는 부모 폴더의 기존 런타임·Cargo cache를 사용하는 도구입니다. 새 PC에서 모든 환경을 자동 준비하는 설치 명령은 아닙니다.

```powershell
powershell -NoProfile -File scripts/build-windows.ps1 -Install
```

Mac에서는 다음과 같이 빌드·설치합니다.

```sh
sh scripts/build-macos.sh --install
```

기존 앱은 교체 전 백업합니다. Mac은 `com.ultrastt.personal` 식별자의 안정적인 로컬 ad-hoc 서명을 사용하며 **Apple 공증을 받은 공개 배포판은 아닙니다**. 음성 캡처에는 마이크 권한, 커서 입력에는 손쉬운 사용 권한이 필요합니다.

### 앱 전용 Codex 로그인

Windows의 기존 공식 native CLI를 공유하되 로그인과 지침 폴더를 분리합니다.

```powershell
pwsh -NoProfile -File scripts/configure-codex-profile.ps1 -Login
pwsh -NoProfile -File scripts/configure-codex-profile.ps1 -Profile isolated
```

개발용 인증 파일을 복사하지 않고 앱 프로필에서 직접 로그인합니다. 현재 Mac 원격 구성은 Windows 서버의 AI를 사용하므로 Mac에서 두 번째 Codex 로그인을 할 필요가 없습니다.

세부 안내: [Windows 설치](docs/windows-personal.md) · [Mac 설치·권한](docs/macos-personal.md) · [Codex 분리](docs/codex-isolation-personal.md) · [원격 구성](remote/README.md).

## 개인정보·운영 경계

- 로컬 STT는 선택한 로컬 호스트에서 처리합니다. Mac 원격 모드의 음성은 비공개 Tailnet을 통해 Windows 서버로 전송됩니다.
- Codex 다듬기를 켜면 인식한 텍스트와 작성 지침은 설정한 ChatGPT 서비스로 전송됩니다. 완전 오프라인 AI인 것처럼 설명하지 않습니다.
- 선택 텍스트는 이를 사용하는 작업에서만 컨텍스트로 포함합니다. 앱 분류는 로컬에서 하며 원본 창 제목을 앱 컨텍스트로 보내지 않습니다.
- 사용 가능한 OS 보안 저장소를 이용합니다. 보안 저장소 실패 시의 기존 fallback은 플랫폼 안내에 설명되어 있으므로, 모든 설정 파일이 비밀정보 없는 파일이라고 가정하지 마세요.
- `.env`, 모델, 인증 파일, 개인 DB·런타임·로그는 Git에 넣지 않습니다.
- Ultra STT 자체의 관리형 구독·포함 API 할당량은 없습니다. 기반 프로젝트의 cloud 연동은 이 개인 배포 구성과 별개입니다.
- 원격 연결은 **Serve**이며 인터넷 공개용 **Funnel**을 사용하지 않습니다. 서버가 켜져 있고 Tailnet에 연결돼 있어야 합니다.

## 검증 상태와 남은 개발

1.1.62 기준 프런트엔드 **414개**, Windows native **629개**, Mac native **635개** 테스트가 통과했습니다. 기본 native 실행에서는 외부 인증·제공자에 의존하는 live 테스트 세 개씩을 제외합니다. 두 데스크톱 대상의 실제 빌드·설치도 완료했습니다.

Mac → Windows 원격 요청에서 다섯 모드, AI 비활성화, 기존 헤더 없는 요청, 잘못된 헤더, 인증 없는 요청과 클라이언트 취소를 확인했습니다. 최신 Mac의 실제 마이크 → 커서 입력 → Esc 사용 확인은 별도 수동 확인 항목입니다. 테스트 통과를 물리적 키 입력 검증으로 대신하지 않습니다.

**모바일은 빌드만 남은 상태가 아닙니다.** 현재는 브라우저에서 녹음하고 결과를 복사·공유합니다. Android/iOS 입력 방식, 권한과 실기기 검증은 후속 구현 대상입니다. [검증 상세](docs/VALIDATION.md).

## 문서·기여·라이선스

- [상세 기능·기술 사양](docs/FEATURES.md)
- [검증 결과·성능 측정·알려진 한계](docs/VALIDATION.md)
- [성능 조사 기록](docs/performance-personal.md)
- [브랜드·아이콘](docs/brand-personal.md) · [모드 이미지 제작 기록](docs/polish-mode-assets.md)
- [기여 안내](CONTRIBUTING.md) · [보안 정책](SECURITY.md) · [이슈](https://github.com/toughCSB/ultra-stt/issues)

[OpenTypeless](https://github.com/tover0314-w/opentypeless)를 바탕으로 개발했습니다. 원 프로젝트의 기여자 표기와 [MIT 라이선스](LICENSE)를 유지합니다. 다른 언어의 기존 README와 오래된 설계 문서는 기반 프로젝트의 기능을 설명할 수 있으므로, 개인 버전 1.1.62의 현황은 이 한국어 문서와 영문 README를 기준으로 보세요.
