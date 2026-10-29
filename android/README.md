# Ultra STT Android 작은 음성 버튼

현재 소스·Fold7 설치 버전: **1.1.73-android.14** (`versionCode=14`), 패키지 `com.ultrastt.androidvoice`. 두 엔진 선택 카드, 버튼 자유 이동, API 키 저장 완료·수정 상태와 연결 테스트를 적용했다. 실제 Google 검색창의 아래·가로 이동, 위치 유지, 취소, 모드 메뉴 검사를 통과한 수정도 포함한다. 모바일 OpenAI 경로는 GPT Transcribe + GPT-6 Luna를 유지하며 Claude·Grok API 키를 추가하지 않았다. [최신 설정·설치·검증 범위](../docs/2026-10-29-model-selection-and-key-status.md) · [버튼 원인·실기 검증](../docs/2026-10-29-engine-cards-and-mobile-drag.md).
GitHub `v1.1.62-personal.2`의 다운로드는 아직 **.9**이며 .10 파일로 교체하지 않았다. 이전 릴리스 파일·태그는 그대로 보존한다. [버튼 수정·성능 검토·검증 범위](../docs/2026-10-04-bubble-usability-review.md).

[APK 다운로드: 개인 프리뷰 2](https://github.com/toughCSB/ultra-stt/releases/tag/v1.1.62-personal.2) · [왜 이런 구조인지 쉬운 설명](../README_ko.md#왜-이렇게-만들었나요-쉽게-이해하기)

첫 번째 키보드 형태는 폐기했다. 기존 키보드를 대체하거나 자동으로 바꾸지 않는다.
입력창이 활성화되면 보내기 버튼과 겹치지 않도록 입력칸보다 충분히 높은 오른편에 마이크 버튼을 표시한다.
버튼 전체 색상으로 현재 모드를 구분하며 검정 네모·한글 약자·작은 모드 배지는 없다.

## 조작

- **한 번 터치:** 녹음 시작.
- **녹음 시작:** 텍스트 키보드가 접히고 화면 중앙에 반투명 녹색 녹음창이 나타난다.
- **중앙창의 녹음 종료:** 선택한 엔진에서 전사·다듬기 → 원래 커서에 입력 요청 → `전송 완료`를 1.2초 표시한 뒤 창이 사라진다.
- **누른 뒤 바로 드래그:** 화면 안전 영역 안에서 버튼 위치를 자유롭게 이동한다. 입력칸 높이나 플로팅 키보드 창 크기로 이동을 막지 않는다. 위치는 저장되며 앱 설정의 `마이크 위치 초기화`로 복원한다.
- **가만히 길게 누른 뒤 드래그:** 진동 후 반원으로 다섯 아이콘이 펼쳐진다. 원하는 아이콘으로 이동해 손을 떼면 선택한다. 왼쪽에 둔 버튼의 메뉴는 오른쪽으로 열린다.
- 현재 선택에는 흰 테두리, 드래그 대상에는 더 굵은 테두리·확대·진동·모드 이름을 표시한다.
- 중앙에서 손을 떼거나 반원 밖으로 드래그하면 모드를 바꾸지 않는다. 길게 누른 동작이 녹음 시작으로 이어지지 않는다.
- 녹음 중에는 중앙창의 모드 이름을 눌러 다섯 모드 중 선택한다. 종료 시 선택을 요청에 넣고 처리 중에는 모드 변경을 잠근다.
- 중앙창에 `듣고 있어요`, 경과 시간, 현재 모드, 맥박처럼 움직이는 마이크를 표시한다.
- 종료 후에는 `변환 중…`과 회전 표시를 보여 준다. 종료 버튼은 비활성화되어 중복 요청하지 않는다. 오른쪽 위 `✕`는 녹음·처리를 취소한다.
- 녹음 중 커서·입력 내용·앱이 바뀌면 취소한다. 전송 중 대상이 바뀌면 자동 입력을 막고 도착한 문장을 복구 화면에 보관한다. 명시적으로 취소한 세션의 늦은 결과는 버린다.
- 입력하지 못한 마지막 문장은 녹음/앱 설정 화면에서 복사·지울 수 있다. 메모리에만 보관하며 프로세스 종료 시 사라진다.
- 대기 원형 버튼은 56dp, 터치 영역은 64dp, 불투명도는 74%다. 녹음창은 녹색으로 통일하고 모드 이름을 명시한다.

| 모드       | 버튼 전체 색        | 반원 메뉴 아이콘 |
| ---------- | ------------------- | ---------------- |
| 원문       | 녹색 `#20A044`      | 음성 파형        |
| 최소       | 청록 `#008C78`      | 지우개           |
| 깔끔하게   | 푸른 청록 `#007EA8` | 마법봉           |
| 구조화     | 파랑 `#2563EB`      | 목록             |
| 전문적으로 | 남보라 `#5046C8`    | 서류가방         |

색뿐 아니라 서로 다른 그림·전체 모드 이름·접근성 설명도 제공한다. [생성 이미지 자산](../docs/android-radial-icons.md).

## 처음 한 번 설정

1. `기존 엔진`을 사용할 때는 휴대폰과 개인 서버 PC에 같은 소유자 계정의 Tailscale을 연결한다.
   앱 전용 IP 연결이 설정된 기기는 **`Use Tailscale DNS`를 켤 필요가 없다**. 다른 앱의 인터넷 연결에 문제가 생겼던 기기는 꺼 둔다.
   `Excluded apps` 목록의 Ultra STT 체크는 **해제**한다. 제외 목록에 체크하면 VPN을 우회한다.
2. `기존 엔진`은 PC 연결 정보를 한 번 받는다. 승인된 갤럭시에는 이미 설정했으므로 **토큰을 직접 입력할 필요가 없다**. `GPT Transcribe`는 앱 설정에서 OpenAI API 키를 직접 입력해 저장한다.
3. Ultra STT 앱에서 마이크 권한을 허용한다.
4. 접근성 → 설치된 앱 → **Ultra STT 작은 음성 버튼**을 켠다.
5. 삼성 키보드·Gboard 등 원하는 기존 키보드로 입력창을 누르고 작은 마이크를 사용한다.

사이드로드 앱이라 접근성이 차단되면 Android 설정 → 애플리케이션 → Ultra STT →
우측 상단 메뉴 → 제한된 설정 허용 후 다시 접근성을 켠다. 앱이 이 보호 장치를 우회하거나 권한을 자동 허용하지 않는다.
앱의 `바로 테스트` 입력창에서도 같은 버튼을 확인할 수 있다. 비밀번호와 앱의 토큰 설정 칸에는 표시하지 않는다.

**기존 엔진을 사용할 때는 PC 앱과 원격 서버가 켜져 있어야 한다.** 이 APK에는 Whisper 모델·Codex 로그인·서버 토큰·OpenAI API 키를 넣지 않았다.
휴대폰 자체에서 실행하는 오프라인 AI, iOS 앱, 휴대폰 기록 관리, Play Store 배포·자동 업데이트는 이번 범위가 아니다.
PC 모델·추론·Fast 설정과 처리 속도를 변경하는 업데이트도 아니다.

### 2026-10-20 · 서버의 작업별 추론 정책 적용

모바일은 AI 모델을 직접 실행하지 않고, 선택한 모드와 다듬기 활성 여부를 기존 SB 서버에 보낸다. 업데이트된 SB v1.1.70에서는 원문·최소·깔끔하게를 **Low + Fast 요청**, 구조화·전문적으로를 **Medium + Fast 요청**으로 처리한다. 모바일의 AI 다듬기를 꺼 두면 이 단계는 건너뛴다. 이 변경에는 APK 재설치나 재로그인, 토큰·DNS·권한 변경이 필요 없다.

현재 소스의 `TranscriptionRequest`가 보내는 기존 모드 헤더와 서버의 동일한 `CodexProvider` 경로를 확인했다. Mac·Home에서 같은 서버 경로의 다섯 모드 처리를 검증했다. 이후 Home에 USB로 연결한 Fold7의 기존 `.10` 앱에서 SB HTTPS 연결·인증 성공 문구를 확인했다. 현재 최소 모드·AI 다듬기 활성화·접근성 활성 상태이며 설정 파일은 검사 전후 바뀌지 않았다. APK 교체나 토큰·권한 변경은 하지 않았다. 새 발화의 마이크→커서 입력과 모바일 처리 속도는 별도 검증이다. 속도 향상을 보장하지 않으며 [측정 결과와 한계](../docs/2026-10-20-polish-reasoning-effort.md)를 참고한다.

## 구조와 권한

- `VoiceAccessibilityService`: 사용 중인 편집 가능한 입력창 감지, `TYPE_ACCESSIBILITY_OVERLAY` 버튼/모드 메뉴.
- Android 13+는 접근성용 `InputMethod.AccessibilityInputConnection.commitText`를 우선 사용한다.
  이것은 기존 기본 키보드를 교체하는 IME 서비스가 아니다.
- 연결 API가 없으면 `ACTION_SET_TEXT`로 커서 앞·뒤의 기존 내용을 보존하고 선택 부분만 교체한다.
  커서 위치가 없거나 변경된 입력창에는 임의로 덮어쓰거나 클립보드를 사용하는 대신 입력을 거부한다.
  접근성 편집을 지원하지 않는 앱과 특수 편집기는 호환성 제한이 있다.
- `RecordingActivity`: 사용자 터치 때만 열리는 작고 투명한 비포커스 녹음 창.
  기존 키보드와 편집창의 포커스를 유지하면서 마이크를 사용할 수 있는 사용자 표시 Activity를 사용한다.
- `INTERNET`, `RECORD_AUDIO`를 요청한다. 서비스는 `BIND_ACCESSIBILITY_SERVICE`로 보호된다.
  별도의 ‘다른 앱 위에 표시’ 권한, 터치 탐색, 제스처 주입, 전체 키 입력 수집 권한은 요청하지 않는다.
- 입력창의 텍스트는 안전한 커서 입력/변경 감지를 위해 **기기 내부에서만 일시 비교**한다.
  화면 이력·기존 입력창 내용은 서버에 전송하지 않는다. 음성 녹음과 전사문을 휴대폰에 기록하지 않는다.
- 최대 120초의 mono PCM16 16 kHz WAV를 메모리에서 만든다. 화면 이동·잠금·취소 시 마이크를 중단한다.
- `기존 엔진`은 음성을 Tailnet HTTPS 개인 PC로 보낸다. AI 다듬기 사용 시 전사문은 기존 PC 경로로 OpenAI에 전송된다. `GPT Transcribe`는 음성과 다듬기용 전사문을 OpenAI에 직접 보낸다.

### Android 직접 OpenAI 엔진

설정의 `엔진 선택`에서 `기존 엔진`(기본값) 또는 `GPT Transcribe`를 고른다. 직접 엔진은 사용자가 입력한 **별도 OpenAI API 키 하나**를 AndroidKeyStore AES-256-GCM으로 암호화해 앱 전용 설정에 저장한다. 저장·삭제·엔진 변경은 API 호출을 하지 않는다. 기존 SB endpoint, 토큰, paired IP는 전환하거나 OpenAI 키를 삭제해도 유지한다.

녹음 종료 후 `gpt-transcribe`에 WAV 파일과 `languages[]=ko`를 보내고, 다듬기가 켜져 있으면 `/v1/responses`의 `gpt-6-luna`로 결과를 처리한다. 응답과 assistant 메시지가 모두 `completed`이고 `output_text`가 있어야 사용하며, 오류·미완료·거절·빈 결과는 전체 원문 전사문으로 되돌린다. 원문·최소·깔끔하게는 `reasoning.effort=low`, 구조화·전문적으로는 `medium`이며, 다듬기에 `service_tier=priority`, `store=false`, `max_output_tokens=8192`를 요청한다. `temperature`는 보내지 않는다. 엔진은 녹음 종료 후 파일 전체를 보내며 녹음 중 바로입력은 추가하지 않았다. 120초 녹음 제한도 유지한다.

다섯 다듬기 지시는 기존 `src-tauri/src/llm/prompt.rs`의 `ORIGINAL_BASE_PROMPT`, `BASE_PROMPT`, `THOUGHT_AWARE_RULES`, `append_polish_style_prompt`에서 충실도·출력·스타일 규칙을 Android에서 쓰는 범위로 옮겼다. Android에는 데스크톱의 앱 문맥·사용자 사전·선택 텍스트·장면 설정이 없으므로 해당 부분은 포함하지 않는다. 직접 엔진은 SB 서버의 실제 프롬프트와 완전 동일하다고 보장하지 않는다.

## 연결 설정과 인증

토큰을 없애거나 APK에 넣는 것이 아니라, **승인된 기기 한 대의 앱 내부에 한 번 저장**한다.
기존 서버의 Bearer 인증과 모드 헤더 계약은 변경하지 않았다.

```powershell
pwsh -NoProfile -File scripts/pair-android.ps1 -Serial '<승인된 갤럭시의 adb serial>'
```

이 개인용 debug 설정 도구는 갤럭시 SM-F966N인지 확인하고 기존 PC 서버 토큰을
ADB의 표준 입력으로 앱 전용 저장소에 전달한다. 토큰을 명령줄 인자·소스·APK·출력·PC 임시 파일에 넣지 않는다.
기기 내부의 10분 만료 연결 파일을 앱이 읽고 Android Keystore AES-256-GCM으로 암호화 저장한 뒤 삭제한다.
소비 여부와 암호화 저장 키 존재를 확인한다. 기존 사용자 설정과 서버 인증 파일은 삭제·변경하지 않는다.
다른 휴대폰 배포나 정식 배포용 페어링 방식이라고 주장하지 않는다.

연결 설정 도구는 PC의 현재 Tailscale hostname과 endpoint가 일치하는지 확인하고,
해당 PC의 Tailscale IPv4도 앱 전용 설정에 전달한다. 승인된 서버 hostname에 한해서
OkHttp의 `Dns`가 이 IP를 직접 반환한다. **HTTPS URL·SNI·기본 인증서/hostname 검증·Bearer 인증은 그대로 유지**한다.
시스템 DNS·VPN·다른 앱의 연결 설정은 변경하지 않는다. 서버 hostname을 수동으로 바꾸면 기존 IP 연결은 해제된다.
PC의 Tailscale IP가 바뀌면 연결 설정 도구를 다시 실행한다. IP·토큰을 APK에 넣지 않는다.

Wi-Fi는 앱의 사용 조건이 아니다. 모바일 데이터에서도 Tailscale과 서버 PC에 연결되면 같은 HTTPS 경로를 사용한다.
다만 개발자가 휴대폰을 조작하는 **Android 무선 디버깅**은 Wi-Fi 연결을 요구하며, Wi-Fi를 다시 켜면 포트가 바뀔 수 있다.
페어링은 PC를 신뢰한다는 등록이므로, 등록만으로 닫힌 디버깅 포트에 연결할 수는 없다.
이는 음성 전사 연결과 별개다. [Android 무선 디버깅 안내](https://developer.android.com/tools/adb#connect-to-a-device-over-wi-fi).

앱의 `PC 서버 연결 확인`은 기존 endpoint에 **오디오 없는 POST**를 보낸다.
인증 통과 후 서버의 정해진 HTTP 413/JSON 오류가 돌아오는지 확인하므로 실제 음성 전사·AI 호출·사용량이 발생하지 않는다.
HTTPS 인증서 검증을 유지하며 리디렉션을 따라 인증 토큰을 전달하지 않는다.
수동 토큰 변경이 필요한 경우에만 고급 연결 설정을 펼친다.

## 빌드

기존 `my-family-app/.sites-runtime/android-tools`의 JDK 21, SDK/build-tools 36, Gradle 8.14.3, AGP 8.13.0 캐시를 재사용한다.
사용자 승인으로 SDK 36과 호환되는 **OkHttp 5.4.0**과 그 전이 의존성을 사용한다.
5.5.0의 AAR은 compile SDK 37을 요구하므로 현재 도구를 바꾸지 않고 5.4.0을 선택했다.
앱 모듈의 AndroidX 지원만 켰으며 데스크톱·다른 프로젝트의 코드와 서명 키는 변경하지 않았다.
첫 빌드는 아래 명령에서 `-Offline`을 빼서 승인된 의존성을 내려받는다. 이후에는 캐시를 재사용한다.

```powershell
pwsh -NoProfile -File scripts/build-android.ps1 -Offline
```

다른 도구 위치는 `-ToolRoot 'D:\path\android-tools'`로 지정한다.
APK: `output/android/Ultra-STT-1.1.64-android.11-debug.apk`.
개인용 debug 서명 키 `android/.local/debug.keystore`를 유지해야 기존 설치 위에 업데이트할 수 있다.
이 빌드는 Play Store 릴리스가 아니다. min SDK 28/target SDK 36이며 현재 실기기 대상은 갤럭시 SM-F966N이다.
Android 9–12 fallback과 다른 제조사/특수 편집기는 이번 실기기 검증 대상이 아니다.

## 검증 상태

### Android .9 위젯 진입과 작은 녹음창

ChatGPT의 홈 화면 위젯으로 바로 열었을 때 입력창이 실제로 활성화돼 있어도,
포커스 조회 결과가 편집 불가능한 다른 요소를 가리켜 음성 버튼을 놓치는 경우를 재현했다.
기존 검사는 입력칸을 한 번 더 눌렀기 때문에 이 경로를 정확히 검사하지 못했다.
이제 빠른 포커스 조회가 실패하면 활성/포커스 앱 창 안에서 실제로 포커스된 편집 노드를 제한적으로 탐색한다.
비밀번호·비활성·보이지 않는 칸은 여전히 제외하며, 대상 확인 조건을 느슨하게 하지 않는다.
화면 전체를 저장하거나 전송하지 않고 최대 256개 노드·32단계까지만 확인한다.

녹음창 너비는 300dp에서 264dp로 줄이고 내부 여백·마이크·글자 크기도 줄였다.
배경 불투명도는 약 91–93%에서 80–82%로 낮췄다. 버튼과 글자는 읽기 쉽게 유지한다.
상단에는 일반 글꼴 대신 기존 `title-lockup.png`에서 추출한 흰색·민트색 **Ultra STT** 타이포그래피를 사용한다.
AI 배경 제거 후보는 사용하지 않았고, 사용자 승인 후 원본 글자 영역 추출·배경 투명화만 수행했다.
재생성: `pwsh -NoProfile -File scripts/extract-android-wordmark.ps1`.

- 최종 .9의 80개 Java 검사·제품/QA lint·빌드·서명 검사와 실기기 18개 시나리오가 통과했다.
- 실제 홈 화면 ChatGPT·Grok 위젯을 눌러 각각 5회 재진입하고, 입력칸을 다시 누르지 않아도 음성 버튼이 나타나는 것을 확인했다. 두 경로에서 녹음 시작·키보드 숨김·Home 취소도 통과했다.
- `findFocus`가 잘못된 호스트를 반환하는 가상 입력창 검사는 수정 전 실패, 수정 후 통과했다. 같은 구조의 비밀번호·포커스 없는 칸은 거부하는 검사도 통과했다.
- 실제 발화를 전송하지 않은 자동 검증이다. 모든 앱 호환성이나 서버 왕복·실제 문장 삽입 완료를 이 검사만으로 보장하지 않는다.

|                                                  길게 누른 뒤 드래그로 선택                                                   |                                             현재 모드·시간·종료                                             |                                                      녹음 중 모드 변경                                                       |
| :---------------------------------------------------------------------------------------------------------------------------: | :---------------------------------------------------------------------------------------------------------: | :--------------------------------------------------------------------------------------------------------------------------: |
| <img src="../docs/images/android-mode-drag.png" alt="반원 모드 메뉴에서 구조화가 흰 테두리로 강조된 실제 화면" width="240" /> | <img src="../docs/images/android-recording-chatgpt.png" alt="키보드를 숨긴 녹색 중앙 녹음창" width="240" /> | <img src="../docs/images/android-recording-mode-picker.png" alt="녹음 중 다섯 모드를 펼쳐 선택하는 실제 화면" width="240" /> |

2026-10-03 실제 Galaxy .9와 ChatGPT 새 입력창에서 촬영했습니다. 영어 예시 초안은 직접 입력한 미전송 문구로 촬영 뒤 삭제했고, 녹음은 취소했으며 음성을 업로드하지 않았습니다. [대기 버튼을 포함한 다섯 장과 조작 순서](../README_ko.md#모바일--익숙한-키보드는-그대로-녹음창은-작게). 당시 화면과 달리 현재 버전은 즉시 드래그로 위치 이동도 지원합니다. 길게 누른 뒤 드래그는 모드 선택입니다.

[위젯 문제 원인과 검증 상세](../docs/2026-10-03-android-widget-entry.md).

### Android .8 중앙 녹음창

작은 버튼 하나에 녹음·모드·진행 상태를 모두 넣으면 상태를 읽기 어렵고 다른 버튼을 가렸다.
이제 작은 버튼은 시작 용도로 유지하고, 녹음부터 완료까지는 PC 실행창처럼 별도 중앙창을 사용한다.
원래 입력창의 포커스를 빼앗지 않는 창이며, 녹음·처리·완료 표시 중 키보드를 접어 두고 닫힐 때 이전 표시 모드를 복원한다.
`전송 완료`는 원래 입력 연결로 텍스트 전달을 요청했다는 뜻이지 ChatGPT 등의 보내기 버튼을 눌렀다는 뜻이 아니다.
입력창 변경·홈 이동·오류는 완료 표시 대신 안전하게 취소한다.

- 80개 Java 검사, 제품/QA lint·APK 빌드·기존 키 서명 검사가 통과했다.
- 갤럭시에서 기존 입력창 회귀 13개와 새 중앙창 상태/취소 2개 시나리오가 통과했다.
- 실제 ChatGPT에서 버튼 표시 → 키보드 숨김 → 중앙 녹음창 → 녹음 유지 → Home 취소·키보드 정책 복원을 확인했다.
- 설치판의 녹음창 디자인은 실제 화면으로 확인했다. 상태·시간·모드 선택·변환 중 재클릭 차단·완료 표시는 실제 제품 View를 별도 QA 앱에서 검사했다.
- 자동 검사는 음성을 업로드하거나 메시지를 보내지 않았다. 실제 발화 전사→커서 입력→완료 표시→자동 닫힘의 전체 경로는 사용자 확인 항목이다.
- DNS/VPN·인증·기본 키보드는 변경하지 않았다. 공개 릴리스 파일도 바꾸지 않았다.

[구현 이유와 상세 검증](../docs/2026-10-03-android-recording-panel.md).

### Android .7 녹음 중 키보드 자동 숨김

마이크를 눌러 녹음을 시작하면 기본 텍스트 키보드를 접어 입력창이 가려지지 않게 한다.
녹음·전사 처리 동안 숨김을 유지하고 완료·취소·오류·서비스 종료 시 이전 키보드 표시 모드를 복원한다.
기본 키보드를 바꾸지 않고 Android의 `SoftKeyboardController`를 사용한다.
키보드를 접을 때 입력 연결이 끊기는 앱에서는 녹음 전에 같은 입력창의 내용·커서를 다시 확인한다.
다른 입력창으로 이동하거나 원래 대상을 확인하지 못하면 녹음하지 않는다.
키보드가 원래 숨김 모드였거나 다른 접근성 서비스가 표시 모드를 바꿨으면 그 설정을 덮어쓰지 않는다.

뒤늦게 보이는 입력창이 버튼 없이 남는 문제도 수정했다. 화면 내용 변경 이벤트를 감지하고 포커스 노드를 최신 상태로 확인한다.
수정 전 실패한 지연 표시 재현 검사가 수정 후 통과했다. 전체 화면 내용을 저장하거나 서버에 전송하지 않는다.

최종 APK에서 갤럭시 전용 테스트 앱의 13개 회귀 검사가 통과했다. 지연 표시, 숨김 시 포커스 종료/빈 내용의 위치 미상 응답,
준비 중 Home/텍스트 변경 취소, 불명확한 비어 있지 않은 커서 거부와 기존 7개 입력창 검사를 포함한다.
실제 ChatGPT의 **Chat·Work 두 화면**, Telegram, Grok에서 버튼 → 키보드 숨김 → 녹음 유지 → Home 취소와 표시 모드 복원을 확인했다.
Grok은 최초 진입 직후 검사가 실패했고 화면이 준비된 후 반복에서 통과했다. 모든 앱·화면에서의 호환성을 보장하는 검사는 아니다.
자동 검사는 메시지를 보내거나 음성을 업로드하지 않았다. 최종판의 실제 발화→결과 입력은 사용자 확인 항목이다.
[원인과 검증 기록](../docs/2026-10-03-android-keyboard-hide.md).

커서나 전체 내용을 신뢰할 수 없는 특수 편집기는 안전하게 거부한다. 특히 비어 있지 않은 원문에서 `offset=-1`을 임의로 0으로 보정하지 않는다.
표시 모드 복원은 기존 키보드의 일반 동작을 허용한다는 뜻이며, 앱과 키보드에 따라 키보드가 다시 보일 수 있다.
[Android 공식 키보드 표시 API](<https://developer.android.com/reference/android/accessibilityservice/AccessibilityService.SoftKeyboardController#setShowMode(int)>).

### Android .6 외부 입력창 수정

- 화면 갱신 이벤트가 연속으로 들어오면 버튼 표시가 계속 뒤로 밀리던 문제를 재현했다. 이제 처음 예약한 확인을 실행하고 뒤따르는 이벤트를 합친다.
- Telegram은 빈 입력창의 `메시지` 안내문을 실제 글자처럼 제공하면서 커서 위치를 알리지 않았다. Android 13+의 실제 입력 연결에서 내용과 커서를 확인한다. 확인할 수 없는 경우에는 여전히 덮어쓰지 않는다.
- 녹음창이 열리면서 Telegram 입력창의 포커스 표시가 사라지는 문제를 재현했다. 앱의 녹음창이 전면에 있고 원래 편집창의 입력 연결이 유지되는 경우에만 녹음을 유지한다. 실제 앱 전환·커서 이동·내용 변경은 취소한다.
- 갤럭시 SM-F966N / Android 17에서 연속 이벤트, 잘못된 힌트 표시, 기존 내용이 있는 커서, 녹음창 전환, 실제 커서 이동, 실제 내용 변경, 입력 연결 재시작의 7개 외부 입력창 검사가 통과했다. 최초 세 결함은 수정 전 실패를 확인했다.
- 같은 앱 안에서 입력 연결이 교체돼도 이전 결과를 보내지 않도록 시작할 때의 연결과 편집기 정보를 보관한다. 녹음 시작 검증이 실패하면 남아 있는 녹음 세션도 즉시 정리한다.
- 실제 Google 검색, ChatGPT, Grok, Telegram 저장한 메시지 입력창에서 버튼 → 녹음 시작·유지 → 홈 이동 취소를 확인했다. 이 자동 검사는 음성을 업로드하거나 메시지를 전송하지 않는다.
- Tailscale 설정에서 `Not using Tailscale DNS`를 확인한 상태로 일반 인터넷·서버 연결·인증 등 11개 통신 검사가 통과했다. Wi-Fi를 끄는 자동 검사는 디버깅 연결 단절로 완료하지 못했지만, 후속 실제 입력 요청에 사용자가 `입력 잘되는거 같아`라고 확인했다. 이는 사용자 실사용 확인이며 자동 모바일망 측정 결과는 아니다.

외부 입력창 검사는 제품과 별도인 `com.ultrastt.androidvoice.qa` 프로세스에서 실행한다.
제품 접근성 서비스를 중단하지 않도록 `FLAG_DONT_SUPPRESS_ACCESSIBILITY_SERVICES`를 사용한다.

```powershell
pwsh -NoProfile -File scripts/build-android.ps1 -Offline -IncludeDeviceTests
adb install -r android/qa/build/outputs/apk/debug/qa-debug.apk
adb shell am instrument -w -e scenario busy com.ultrastt.androidvoice.qa/com.ultrastt.androidvoice.EditorChecks
```

`scenario`에는 `busy`, `placeholder`, `placeholder-content`, `paused`, `cursor`, `text`, `restart`, `keyboard`를 사용할 수 있다.
`current`는 사용자가 이미 열어 둔 외부 입력창의 녹음 시작을 확인한 뒤 홈으로 이동해 취소한다.
기기 통신 검사는 별도의 `.test` APK를 사용하며, QA APK는 제품 APK와 배포 파일에 포함하지 않는다.

### 이전 버전에서 확인한 기본 동작

- APK 빌드·80개 Java 검사·Android Lint 오류 0·APK v2 서명 검증 통과.
- Java 검사는 WAV·HTTPS·토큰·모드·취소 세대와 커서 삽입/선택 교체/한국어·이모지 보존을 다룬다.
- 갤럭시 업데이트 설치·Activity 실행·기존 IME 제거·기존 기본 키보드 미변경 확인.
- 사용자 승인으로 기기 연결 설정을 저장했고 임시 연결 파일 삭제와 암호화 키 존재를 확인했다.
- 사용자가 접근성을 켰으며 시스템의 서비스 등록/활성화를 확인했다.
- 갤럭시의 실제 버튼 표시, 검정 사각형 없는 단색 원, 반원 메뉴의 다섯 그림·색상, 드래그 선택·저장을 확인했다.
- 다섯 모드를 각각 드래그해 저장됐는지 확인했고, 중앙에서 길게 누른 뒤 놓으면 선택이 유지되는 것도 확인했다.
- 녹음 창의 카메라 컷아웃·시스템 바 좌표 차이를 제거했다. 대기·녹음 버튼의 실제 프레임이 같고 녹음 중 반원 드래그로도 모드를 바꾸는 것을 확인했다.
- 빈 입력창의 힌트를 실제 내용으로 읽던 문제를 수정했다. 실기기의 `RECORD_AUDIO (running)`과 화면 이동 후 녹음 종료를 확인했다. 이 취소 검증에서는 음성을 서버에 보내지 않았다.
- 실제 휴대폰에서 앱 전용 IP 경로의 HTTPS·Bearer 연결 확인이 통과했다. 잘못된 Bearer 거부·다른 HTTPS hostname 거부·리디렉션 금지·취소·응답 크기 제한을 포함한 10개 기기 통신 검사가 통과했다. 이 검사는 오디오나 AI 요청을 보내지 않는다.
- 이전 버전에서 처리 중 재터치가 실제 취소로 이어지는 것을 재현했다. 재터치는 이제 취소하지 않는다. 오디오 캡처와 네트워크 worker를 분리해 중단된 통신 대기가 다음 녹음 실행을 막지 않도록 했다.
- 실기기에서 녹음 중 흰 종료 버튼·시간 표시와 마이크 실행을 확인했다. 사용자가 앱의 `바로 테스트` 입력창에서 실제 한국어 문장 → 녹음 종료 → 전사문 커서 입력 성공을 확인했다. 다른 앱·특수 편집기 전체의 호환성 검증을 의미하지 않는다.
- UIAutomator 실행 중 접근성 서비스가 일시 억제될 수 있으므로 버튼 실기기 검증에는 직접 터치 이벤트와 WindowManager·실제 화면을 함께 사용한다.
  앱의 인증 설정 화면은 `FLAG_SECURE`로 캡처에서 숨겨진다. 캡처의 검정 배경은 설정 화면 보호이며 버튼의 검정 네모가 아니다.

실기기 통신 검사 코드는 `app/src/androidTest/java/com/ultrastt/androidvoice/NetworkChecks.java`다.
기기 페어링 후 `:app:assembleDebugAndroidTest`로 테스트 APK를 빌드·설치하고,
`adb shell am instrument -w com.ultrastt.androidvoice.test/com.ultrastt.androidvoice.NetworkChecks`로 실행한다.
별도 테스트 앱의 `EditorActivity`는 실제 대화나 기존 내용을 건드리지 않는 외부 입력창 QA 화면이다.
Instrumentation 종료는 Android에서 대상 프로세스를 종료할 수 있으므로 테스트 후 앱 업데이트 설치로
이미 허용된 접근성 서비스를 재바인딩하고 상태를 확인한다. 테스트 앱은 제품 APK에 포함되지 않는다.

공식 근거:
[OkHttp HTTPS](https://lysine.dev/okhttp/features/https/),
[OkHttp 변경 이력](https://lysine.dev/okhttp/changelogs/changelog/),
[접근성 서비스](https://developer.android.com/reference/android/accessibilityservice/AccessibilityService),
[접근성용 입력 연결](https://developer.android.com/reference/android/accessibilityservice/InputMethod),
[AudioRecord](https://developer.android.com/reference/android/media/AudioRecord),
[Android Keystore](https://developer.android.com/privacy-and-security/keystore),
[입력 힌트 구분](<https://developer.android.com/reference/android/view/accessibility/AccessibilityNodeInfo#isShowingHintText()>),
[시스템 바 인셋과 창 좌표](<https://developer.android.com/reference/android/view/WindowManager.LayoutParams#setFitInsetsTypes(int)>),
[Tailscale Android 앱 제외 동작](https://tailscale.com/docs/features/client/android-app-split-tunneling).
