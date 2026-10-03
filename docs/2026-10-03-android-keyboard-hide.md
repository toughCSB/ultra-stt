# Android 녹음 중 키보드 자동 숨김

## 최종 설치 상태 — 후속 수정

아래 조사 중 미완료 기록 이후, 사용자가 ChatGPT 버튼 누락과 미적용을 다시 보고해 추가 재현·검증을 진행했다.

- 설치판: `1.1.62-android.7`, SHA256 `9108D86CDD408B82959C8B5C32C86DF602C9D9E3213E362E7FEFDD0171F7FF36`. 기존 키로 업데이트했으며 앱 데이터·기본 키보드·DNS/VPN·인증은 변경하지 않았다.
- 새로 확인한 원인: 서비스가 `TYPE_WINDOW_CONTENT_CHANGED`를 구독하지 않아, 포커스 이후 늦게 보이는 입력창의 가시성 변화를 놓쳤다. `late-visible` fixture는 .6에서 버튼 누락으로 실패하고 최종 .7에서 통과했다. 내용 변경 이벤트 수신과 포커스 노드 `refresh()`를 추가했다. 기존 100ms 이벤트 병합은 유지한다.
- 최종 .7의 80개 Java 검사, 제품/QA lint, APK 빌드·v2 서명 검증이 통과했다.
- 최종 .7의 실기기 13개 시나리오 통과: `late-visible`, `keyboard`, `keyboard-blur`, `keyboard-cancel`, `keyboard-blur-content`, `keyboard-edit`, `busy`, `placeholder`, `placeholder-content`, `paused`, `cursor`, `text`, `restart`.
- 최종 .7의 실제 ChatGPT Chat·Work, Telegram, Grok에서 버튼 표시·숨김·녹음 유지·Home 취소·모드 복원이 통과했다. Grok은 최초 화면 진입에서 버튼 검사에 실패했지만 화면 준비 후 반복에서 통과했다. 임의의 편집기나 모든 레이아웃에 대한 보장은 아니다.
- 자동 검사는 음성을 업로드하거나 메시지를 전송하지 않았다. 최종 실제 발화→커서 삽입은 사용자 확인으로 남긴다. GitHub 릴리스는 여전히 .5이고 새 commit/push/release는 하지 않았다.
- 아래 SHA256과 미완료 항목은 **이전 후보 조사 당시 기록**이며, 현재 설치판과 구분한다.

## 목표와 보존 조건

마이크를 누르면 기존 텍스트 키보드를 접어 입력창을 가리지 않게 한다. 기본 키보드, 서버 인증, DNS/VPN 설정은 바꾸지 않는다. 원래 입력창·텍스트·커서가 바뀌면 취소하며, 다른 입력창에 결과를 넣지 않는다.

## 호환성 조사

- `SoftKeyboardController.setShowMode(SHOW_MODE_HIDDEN)`으로 키보드를 숨기고, 성공·오류·취소·서비스 종료 시 이전 모드를 복원한다. 다른 접근성 서비스가 모드를 변경한 경우 덮어쓰지 않는다.
- 녹음 시작 후 숨기는 첫 구현은 Telegram·Grok·Google에서 통과했지만 ChatGPT에서는 입력 연결 종료로 녹음이 취소됐다.
- 녹음 중 원래 입력창에 `ACTION_FOCUS`를 보내는 복구는 ChatGPT를 앞으로 가져와 녹음 Activity가 중단됐다. 이 방식은 폐기하고 Activity의 `onPause` 취소 보호를 유지한다.
- 숨김·원래 커서 검증을 녹음창 실행 **이전**으로 옮겼다. 준비 중 다른 입력창 클릭·앱 전환·내용/커서 변경·잠금·시간 초과는 취소한다.
- ChatGPT는 재포커스 후 실제 내용과 커서가 같아도 `SurroundingText.offset=-1`을 반환했다. Android에서 -1은 실패가 아니라 절대 오프셋을 모른다는 의미다. 절대 위치가 필요한 전체 내용 교체에 그대로 사용해서는 안 된다.
- 반복 실패 후 기기를 사용자 확인 완료판 Android .6으로 일시 복구하고 독립 검토를 진행했다. 진단은 내용·토큰이 아닌 상태/일치 여부만 기록했다.

## 검증 상태

- 후보 APK `.7`의 80개 Java 검사·제품/QA lint·빌드·기존 키의 v2 서명 검사를 통과했다. 제품 임시 진단 로그는 제거했다.
- 후보 SHA256: `DD41F2A1323DB8B2C12459D16F27A95C33FFA07DEAFAC83F9EA9878F3B01A09D`.
- 빈 원문·빈 실제 응답·선택 0,0·힌트가 아닌 빈 노드가 모두 일치하는 경우에만 `offset=-1`을 허용한다. 비어 있지 않은 원문으로 확대하지 않았다. 준비 시와 입력 직전 동일 검사를 사용한다.
- 직전 진단 빌드의 `keyboard`, `keyboard-blur`, `keyboard-cancel` 실기기 검사가 통과했다. blur fixture는 실제 숨김에 따른 포커스 손실과 `offset=-1` 응답 발생까지 확인한다. 가시성 대기·진단 제거를 반영한 최종 후보 APK의 실기기 반복은 아직 하지 못했다.
- 마지막 실제 ChatGPT 검사에서는 입력창이 보이지 않는다는 판정으로 안전 취소됐다. 사용자가 조작 중인 정황 및 Fold 펼침 상태 변화가 있었다. `refresh()` 실패와 레이아웃 이동 중 일시적인 비가시성을 그 로그만으로 구분할 수 없다. 단정하지 않고 손을 뗀 상태로 재검증해야 한다.
- 마지막 후보는 동일 노드 refresh·같은 활성 창·잠금 해제·다른 대상 없음 등의 조건이 모두 통과한 경우에만 가시성 회복을 기존 1.5초 기한 안에서 기다린다. 보이지 않는 노드에는 포커스 명령이나 녹음 시작을 하지 않는다.
- `keyboard-blur-content`와 `keyboard-edit` 부정 경로도 QA에 추가하고 QA 빌드·lint를 통과했지만 실기기 실행은 아직이다. 준비 시 내용 변경·비어 있지 않은 위치 미상 원문은 취소해야 한다.
- 사용자에게 잠금 해제와 1분 조작 중지를 요청했다. 미검증 후보를 사용판으로 남기지 않도록 **마이크가 동작하지 않는 것을 확인한 뒤 .6을 다시 설치**했다. 앱 데이터는 지우지 않았으며 Samsung HoneyBoard와 키보드 표시 모드 0을 확인했다.
- 최종 설치·실제 음성 입력 확인은 미완료다. 이전 테스트 통과를 최종 APK 검증으로 간주하지 않는다. commit/push/release는 하지 않았다.

## 공식 API 근거

- [SoftKeyboardController](https://developer.android.com/reference/android/accessibilityservice/AccessibilityService.SoftKeyboardController)
- [SurroundingText.getOffset](https://developer.android.com/reference/android/view/inputmethod/SurroundingText#getOffset())
- [AccessibilityInputConnection](https://developer.android.com/reference/android/accessibilityservice/InputMethod.AccessibilityInputConnection)
