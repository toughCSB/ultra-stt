# 두 가지 엔진 선택 — 구현 및 검증 범위

후속 상태: [엔진 카드·모바일 이동 검증](2026-10-29-engine-cards-and-mobile-drag.md)에서 Fold7 Android .12 설치와 실제 Google 검색창 조작까지 완료했습니다. 아래 검증 목록은 최초 구현 시점의 결과이며 데스크톱 설치·실제 유료 API 비교는 여전히 미완료입니다.

## 사용자에게 보이는 선택

설정의 **엔진 선택**에서 두 가지만 고릅니다.

| 선택 | 전사 | AI 다듬기 | SB 필요 여부 |
| --- | --- | --- | --- |
| 현재 설정 사용 | 저장된 Whisper / SB 연결 그대로 | 저장된 Codex CLI 경로 그대로 | 기존 기기 설정에 따름 |
| GPT Transcribe + GPT-6 Luna | 기기 → OpenAI `gpt-transcribe` | 기기 → OpenAI Responses `gpt-6-luna` | 필요 없음 |

새 경로는 하나의 OpenAI API 키로 두 단계를 호출합니다. ChatGPT/Codex 구독 로그인과 별도이며 API 사용량이 청구됩니다. 두 모델에 접근 가능한 API 프로젝트가 필요합니다. 키 저장 표시는 인증·잔액·모델 접근 확인이나 속도 측정 성공을 뜻하지 않습니다.

Mac은 원래 자체 Codex CLI를 실행할 수 있습니다. 다만 이번 변경은 기존 Mac 연결을 임의로 바꾸지 않습니다. 새 API 경로를 선택하면 Mac과 Fold7 모두 전사뿐 아니라 다듬기도 SB를 거치지 않습니다. 인터넷 연결은 계속 필요하며 OpenAI 지연·한도·장애까지 없어지는 것은 아닙니다.

## 보존 및 안전 경계

- 저장된 기존 provider·서버 주소·토큰·Codex 모델을 덮어쓰지 않고 실행 시 설정 복사본에 새 경로를 적용합니다. 기존 선택으로 돌아가면 원래 연결을 사용합니다.
- 설정 UI에서는 다른 엔진 선택을 제거했습니다. 이전 설정 호환성을 위해 내부 provider 구현은 남겨 두었습니다.
- 5개 다듬기 스타일, 단축키, 입력 대상 보호, 취소, 기존 기록 동작을 유지합니다. 모바일은 녹음 시작 시 선택한 엔진을 해당 녹음에 고정합니다.
- 데스크톱 API 키는 전용 OS 자격 증명 항목, Android는 별도 AndroidKeyStore AES-GCM 항목에 저장합니다. 기존 서버 토큰을 OpenAI 키로 재사용하지 않습니다.
- 공식 HTTPS 주소만 사용하고 리디렉션·자동 유료 재시도를 하지 않습니다. OpenAI에 SB 전용 헤더를 보내지 않습니다.
- 다듬기 요청은 `store:false`, `service_tier:priority`를 사용합니다. Priority는 추가 비용이 있을 수 있고 실제 적용 등급·응답 시간은 계정과 서비스 상태에 따릅니다. `store:false`는 모든 제공자 보관 정책을 없애는 보장이 아닙니다.
- 원문·최소·깔끔하게는 Low, 구조화·전문적은 Medium, 데스크톱의 복잡한 선택 편집/질문은 기존 effort 분기를 적용합니다.
- 다듬기 응답이 불완전하거나 잘렸으면 완성 결과로 입력하지 않습니다. 전사 원문을 유지하고 모바일 원문 대체 시 안내합니다.
- 보관된 녹음을 OpenAI로 재전사할 때는 전송 대상·사용량 비용을 표시하고 두 번째 확인을 받습니다. 복구 음성은 자동으로 입력하거나 삭제하지 않습니다.

## 바로입력과 길이 제한

데스크톱의 새 바로입력은 말을 잠깐 멈춘 구간을 `gpt-transcribe`에 보내 확정 구간을 입력합니다. 자라는 녹음 전체를 계속 유료 재전사하지 않습니다. HTTP 처리 중 새로 들어온 음성도 다음 구간으로 보존합니다. 마지막 구간 실패를 앞부분만 성공한 것으로 처리하지 않도록 worker 오류를 최종 종료까지 전달합니다.

이는 `gpt-live-transcribe` 또는 Realtime WebSocket 구현이 아닙니다. 연속 발화는 최대 구간 경계까지 입력이 늦을 수 있습니다. 바로입력은 기존처럼 확정 전사 구간을 입력하며 전체 문서 AI 다듬기를 매 구간 실행하지 않습니다.

일반 데스크톱 녹음은 새 엔진에서 기본 권장 600초, 클라이언트 버퍼 한도 기반 상한 720초입니다. 이는 제품 제한이지 실제 10분 한국어 품질 검증 결과가 아닙니다. 모바일은 기존 120초 녹음 후 전사 방식 그대로이며 실시간 입력이나 녹음 시간 연장을 추가하지 않았습니다.

## 검증과 적용

테스트 로그는 저장소 상위 `qa/engine-profiles-20261029/`에 둡니다. 네트워크 회귀 검사는 로컬 mock을 사용합니다. Android instrumentation 테스트 APK 빌드는 기기에서 테스트를 실행했다는 뜻이 아닙니다.

2026-10-29 최종 확인:
- Frontend 52 files / 434 tests 통과, TypeScript·ESLint·Prettier·production frontend build 통과.
- Windows Rust 직렬 실행: 771 unit tests + 9 integration tests 통과, 기존 ignored 3개 유지. `cargo fmt` 및 `git diff --check` 통과.
- 최초 병렬 Rust 검사에서 기존 UIA helper 시작 제한시간 관련 6개가 실패했습니다. 제한시간을 늘리거나 검사를 끄지 않았고 최종 전체 직렬 검사가 통과했습니다. 병렬 부하 민감성은 남아 있습니다.
- Android 117 local checks, 앱·instrumentation APK build, lint, APK signature 통과. APK SHA256: `F08D35F4DAC7672C2345819EC4BABFF98906A3D97F501EF0C8CA663DDA83362E`.
- 연결된 브라우저가 없어 실제 렌더링 시각 검증은 진행하지 못했습니다. DOM 기반 컴포넌트 검사를 실기 화면 검증으로 간주하지 않습니다.
- macOS native build/GUI, Android instrumentation 실행/실기, 실제 API 전사·다듬기·속도 비교는 미검증입니다.

실제 OpenAI 키 입력·유료 요청·한국어 긴 발화 비교·각 기기 실제 커서 입력은 후속 확인 대상입니다. 신규 키를 코드에 넣거나 다른 프로젝트 키를 가져오지 않았습니다. 설치된 SB/Home/Mac/Fold7 앱, FP16 엔진, 드라이버, 기존 인증·설정은 이번 구현에서 교체하지 않았습니다. commit/push/release도 하지 않았습니다.

공식 API 근거:
- [Audio transcription](https://developers.openai.com/api/reference/resources/audio/subresources/transcriptions/methods/create)
- [GPT-6 Luna](https://developers.openai.com/api/docs/models/gpt-6-luna)
- [Responses](https://developers.openai.com/api/reference/typescript/resources/responses/methods/create)
- [Fast mode](https://developers.openai.com/api/docs/guides/fast-mode)

기존 모델 품질·외부 벤치마크 검토는 `2026-10-29-stt-external-evidence.md`와 `2026-10-29-openai-stt-engine-redesign.md`를 참고합니다. 구조상 SB 병목을 피하지만 실측 없이 속도 개선 수치를 보장하지 않습니다.
