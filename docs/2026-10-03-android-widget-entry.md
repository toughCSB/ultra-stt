# Android .9 — 위젯 진입 버튼 누락과 녹음창 다듬기

## 재현과 원인

사용자는 ChatGPT·Grok의 홈 화면 위젯으로 바로 연 입력창에서 음성 버튼이 사라진다고 보고했다.
기존 `current` 검사는 입력창이 포커스되지 않았다고 판단하면 다시 탭했으므로 정확한 위젯 진입 검사가 아니었다.
추가 `current-observe`는 입력창을 건드리지 않고 버튼만 확인한다.

수정 전 ChatGPT 위젯에서 `Voice button missing without extra editor tap` 실패를 확인했다.
별도 QA의 노드 순회에는 보이고 편집 가능하며 포커스된 입력칸이 있었다. 반면 실제 접근성 서비스의 포커스 조회는
편집 불가능하고 포커스되지 않은 요소를 반환했다. 내용 변경 이벤트는 서비스에 도착했으므로 단순 이벤트 미구독 문제는 아니었다.
단일 포커스 조회 결과만 신뢰하는 경로가 실제 입력칸을 놓쳤다. 프레임워크 캐시와 앱의 가상 노드 제공 중 어느 쪽이 최초 원인인지는 단정하지 않는다.

`stale-focus` fixture는 실제 `AccessibilityNodeProvider`가 포커스 조회에 편집 불가능한 호스트를 반환하지만,
하위에는 실제 포커스된 가상 편집 노드가 있는 경우다. 수정 전 버튼 누락으로 실패했고 수정 후 녹음·취소까지 통과했다.

## 수정 범위

- 기존 빠른 `findFocus`·`refresh()` 경로를 유지한다.
- 실패하면 활성 또는 포커스된 앱 창 안에서 최대 256개 노드, 깊이 32까지 살핀다.
- 실제 `isFocused`, 편집 가능, 활성화, 가시성, 비밀번호 제외를 모두 만족하는 노드만 채택하고 다시 refresh한다.
- 커서·내용·앱 변경 시 취소하는 기존 보호를 유지한다. 포커스되지 않은 입력칸을 추측해 선택하지 않는다.
- 전체 화면 저장/전송, 서비스 권한 추가, 전역 접근성 캐시 비활성화는 하지 않았다. 임시 진단 로그도 제거했다.

공식 API: [AccessibilityNodeProvider.findFocus](https://developer.android.com/reference/android/view/accessibility/AccessibilityNodeProvider#findFocus(int)),
[AccessibilityNodeInfo](https://developer.android.com/reference/android/view/accessibility/AccessibilityNodeInfo).

## 디자인

- 패널 너비 300 → 264dp, 여백 20/16 → 16/12dp, 마이크 60 → 48dp, 타이머 26 → 22sp.
- 배경 alpha 232–238 → 204–210/255. 텍스트와 조작 버튼 전체에 alpha를 적용하지 않아 읽기 쉬움을 유지한다.
- 원본 `src/assets/brand/title-lockup.png`의 글자만 `scripts/extract-android-wordmark.ps1`로 추출해 검정 배경을 투명화했다.
- 사용자가 코드 기반 원본 추출을 명시 승인했다. AI 배경 제거 후보 두 개는 글자 손상 때문에 사용하지 않았다. 원본 파일을 덮어쓰거나 글꼴로 재작성하지 않았다.
- `brand_wordmark.png`: 1346×229 RGBA, 모서리 alpha 0. 최종 설치 화면에서 글자·투명 배경·크기·취소 조작을 확인했다.

## 최종 검증

- 버전 `1.1.62-android.9` / code 9, SM-F966N Android 17.
- APK `output/android/Ultra-STT-1.1.62-android.9-debug.apk`.
- SHA256 `5E95F1F948C60A28301AD2B8DFE7F270E3D18544EFD5468FE0E1C2002F3E5CC4`, 설치된 base.apk와 일치.
- `scripts/build-android.ps1 -Offline -IncludeDeviceTests`: 80개 Java 검사, 제품·QA lint 오류 0, APK·계측 앱 빌드, 기존 서명 키 v2 검증 통과.
- 실기기 18개: `stale-focus`, `stale-focus-password`, `stale-focus-unfocused`, `panel-states`, `panel-cancel`, `keyboard`, `keyboard-blur`, `keyboard-cancel`, `keyboard-blur-content`, `keyboard-edit`, `busy`, `placeholder`, `placeholder-content`, `paused`, `cursor`, `text`, `restart`, `late-visible`.
- 최종판에서 실제 ChatGPT·Grok 홈 화면 위젯을 각각 5회 눌러 진입했다. 매회 실제 앱 Activity를 확인하고 입력칸 재터치 없이 음성 버튼 Window가 나타나는 것을 검사했다. 바로가기 Activity 직접 실행으로 대체하지 않았다.
- 두 위젯 경로 모두 녹음 시작·키보드 숨김·중앙 패널·녹음 유지·Home 취소·키보드 정책 복원 통과.
- 최종 녹음창 실제 스크린샷 `docs/images/android-recording-panel-9.png`. 녹음 중 캡처 후 취소했으며 전사 서버에 음성을 보내지 않았다.
- 검증 후 마이크 비활성, 표시 정책 0, Samsung HoneyBoard 유지. 임시 QA 앱 제거; 본앱 데이터·인증·DNS/VPN은 변경하지 않았다.

## 검증 한계

실제 발화→서버 전사→문장 삽입→완료 표시 전체 경로를 이번 자동 검사로 확인한 것은 아니다.
임의의 앱·다른 기기·모든 위젯 레이아웃에 대한 보장도 아니다. 구현 검증 당시에는 기존 .8 APK를 보존하고 commit/push/release를 하지 않았다. 이후 사용자 요청에 따른 배포 내용은 [개인 프리뷰 2 릴리스 기록](releases/v1.1.62-personal.2.md)에 별도로 정리했다.
