# Ultra STT Android 작은 음성 버튼

현재 개인용 APK: **1.1.62-android.5**, 패키지 `com.ultrastt.androidvoice`.

[APK 다운로드: 개인 프리뷰 릴리스](https://github.com/toughCSB/ultra-stt/releases/tag/v1.1.62-personal.1) · [왜 이런 구조인지 쉬운 설명](../README_ko.md#왜-이렇게-만들었나요-쉽게-이해하기)

첫 번째 키보드 형태는 폐기했다. 기존 키보드를 대체하거나 자동으로 바꾸지 않는다.
입력창이 활성화되면 키보드 위쪽 오른편에 작은 마이크 버튼만 표시한다.
버튼 전체 색상으로 현재 모드를 구분하며 검정 네모·한글 약자·작은 모드 배지는 없다.

## 조작

- **한 번 터치:** 녹음 시작.
- **다시 터치:** 녹음 종료 → 기존 개인 PC 서버에서 전사·다듬기 → 원래 커서에 입력.
- **길게 누른 채 드래그:** 왼쪽 반원으로 다섯 아이콘이 펼쳐진다. 원하는 아이콘으로 이동해 손을 떼면 선택한다.
- 현재 선택에는 흰 테두리, 드래그 대상에는 더 굵은 테두리·확대·진동·모드 이름을 표시한다.
- 중앙에서 손을 떼거나 반원 밖으로 드래그하면 모드를 바꾸지 않는다. 길게 누른 동작이 녹음 시작으로 이어지지 않는다.
- 녹음 중에도 모드를 바꿀 수 있다. 종료 시 모드를 요청에 넣는다. 처리 중 모드 변경은 다음 요청부터 반영된다.
- 녹음 중에는 흰 종료 사각형과 `00:03` 같은 경과 시간, 맥박처럼 움직이는 원을 표시한다.
- 종료 후에는 흰 회전 표시로 처리를 구분한다. 처리 중 재터치는 안내만 표시하며 작업을 취소하지 않는다.
- 커서·입력 내용·앱이 바뀌거나 기기를 잠그면 취소하고 늦게 도착한 결과도 버린다.
- 원형 버튼은 48dp, 터치 영역은 56dp, 대기 불투명도는 90%다. 녹음·처리 중에도 모드 색상을 유지한다.

| 모드 | 버튼 전체 색 | 반원 메뉴 아이콘 |
| --- | --- | --- |
| 원문 | 녹색 `#20A044` | 음성 파형 |
| 최소 | 청록 `#008C78` | 지우개 |
| 깔끔하게 | 푸른 청록 `#007EA8` | 마법봉 |
| 구조화 | 파랑 `#2563EB` | 목록 |
| 전문적으로 | 남보라 `#5046C8` | 서류가방 |

색뿐 아니라 서로 다른 그림·전체 모드 이름·접근성 설명도 제공한다. [생성 이미지 자산](../docs/android-radial-icons.md).

## 처음 한 번 설정

1. 휴대폰과 개인 서버 PC에 같은 소유자 계정의 Tailscale을 연결한다.
   `Use Tailscale DNS`를 켜고, `Excluded apps` 목록의 Ultra STT 체크는 **해제**한다. 제외 목록에 체크하면 VPN을 우회한다.
2. PC 연결 정보를 한 번 받는다. 승인된 갤럭시에는 이미 설정했으므로 **토큰을 직접 입력할 필요가 없다**.
3. Ultra STT 앱에서 마이크 권한을 허용한다.
4. 접근성 → 설치된 앱 → **Ultra STT 작은 음성 버튼**을 켠다.
5. 삼성 키보드·Gboard 등 원하는 기존 키보드로 입력창을 누르고 작은 마이크를 사용한다.

사이드로드 앱이라 접근성이 차단되면 Android 설정 → 애플리케이션 → Ultra STT →
우측 상단 메뉴 → 제한된 설정 허용 후 다시 접근성을 켠다. 앱이 이 보호 장치를 우회하거나 권한을 자동 허용하지 않는다.
앱의 `바로 테스트` 입력창에서도 같은 버튼을 확인할 수 있다. 비밀번호와 앱의 토큰 설정 칸에는 표시하지 않는다.

**PC 앱과 원격 서버가 켜져 있어야 한다.** 이 APK에는 Whisper 모델·Codex 로그인·서버 토큰을 넣지 않았다.
휴대폰 자체에서 실행하는 오프라인 AI, iOS 앱, 휴대폰 기록 관리, Play Store 배포·자동 업데이트는 이번 범위가 아니다.
PC 모델·추론·Fast 설정과 처리 속도를 변경하는 업데이트도 아니다.

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
- 음성은 Tailnet HTTPS 개인 PC로 보낸다. AI 다듬기 사용 시 전사문은 기존 PC 경로로 OpenAI에 전송된다.

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
APK: `output/android/Ultra-STT-1.1.62-android.5-debug.apk`.
개인용 debug 서명 키 `android/.local/debug.keystore`를 유지해야 기존 설치 위에 업데이트할 수 있다.
이 빌드는 Play Store 릴리스가 아니다. min SDK 28/target SDK 36이며 현재 실기기 대상은 갤럭시 SM-F966N이다.
Android 9–12 fallback과 다른 제조사/특수 편집기는 이번 실기기 검증 대상이 아니다.

## 검증 상태

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
[입력 힌트 구분](https://developer.android.com/reference/android/view/accessibility/AccessibilityNodeInfo#isShowingHintText()),
[시스템 바 인셋과 창 좌표](https://developer.android.com/reference/android/view/WindowManager.LayoutParams#setFitInsetsTypes(int)),
[Tailscale Android 앱 제외 동작](https://tailscale.com/docs/features/client/android-app-split-tunneling).
