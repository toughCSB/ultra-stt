# Gemini 3.5 Transcribe — Ultra STT 적용 검토

- 조사 기준: 2026-10-02. Google 공식 문서·공개 구현, 벤치마크 운영자의 원본 결과, 현재 Ultra STT 소스·기존 QA 로그를 확인했다. 가격·벤치마크는 열람 시점의 스냅샷이며 변경될 수 있다.
- 범위: 전용 STT 모델의 문서상 계약. API 호출, 계정·인증 상태 조회, 음성 업로드, 앱 코드 변경, 실측 벤치마크는 수행하지 않았다.
- 결론: **적용 후보로는 유망하지만, 최고 성능이라는 주장이나 기본 모델 교체는 근거가 부족하다.** 마이크 입력에는 `gemini-3.5-transcribe-live`, 파일 처리에는 `gemini-3.5-transcribe`를 검토한다. 일반 Gemini Flash의 오디오 이해나 음성 대화 모델과 별도 제품이다. 한국어 지원은 확인했으나 우리 환경에서의 품질·속도 우위는 미확인이다. [모델 문서](https://ai.google.dev/gemini-api/docs/models/gemini-3.5-transcribe), [Live 가이드](https://ai.google.dev/gemini-api/docs/live-api/live-transcribe)

## 의사결정 요약

| 질문 | 판단 | 근거 수준 |
| --- | --- | --- |
| 실제 전용 STT 모델인가? | 그렇다. 파일형과 실시간형이 각각 존재한다. | 공식 API 문서 확인 |
| 모든 STT 중 가장 정확하고 빠른가? | 아니다. 공개 비교에서 더 낮은 오류율·지연을 가진 모델이 있다. | 아래 독립 벤치마크 스냅샷 |
| 한국어에서 우리 Whisper보다 좋은가? | 아직 모른다. 한국어 지원과 한국어 품질 우위는 다른 주장이다. | 동일 음성 A/B 미실시 |
| STT만 교체하면 현재 느림이 해결되는가? | 개선 폭이 작을 가능성이 높다. 기존 짧은 문장 QA에서는 AI 다듬기가 주 병목이었다. | 현재 소스·기존 로그 기반 추론 |
| 큰 속도 개선 가능성은? | Smart 전사로 별도 AI 호출을 생략하는 경로. 다만 기존 모드와 출력 정책이 달라 별도 동의·검증이 필요하다. | 공식 기능 + 설계 제안, 실측 아님 |
| 지금 권고하는 조치 | 기본값 보존 → 선택형 시험 provider → 한국어 A/B → 통과한 경로만 적용. | 제안 |

## 확인 사항

### 모델, 한국어, 상태

| 용도 | 정확한 모델 ID | 공식 호출 경로 | 핵심 한도 |
| --- | --- | --- | --- |
| 파일/비실시간 | `gemini-3.5-transcribe` | Interactions API `POST https://generativelanguage.googleapis.com/v1beta/interactions`; 공식 가이드의 기본 예시는 Files API 업로드 후 URI·MIME type 전달 | 요청당 오디오 최대 1시간, 화자 분리 또는 단어 타임스탬프 사용 시 최대 30분 |
| 마이크/실시간 | `gemini-3.5-transcribe-live` | Live API 양방향 WebSocket `wss://generativelanguage.googleapis.com/ws/google.ai.generativelanguage.v1beta.GenerativeService.BidiGenerateContent` 또는 Google Gen AI SDK `live.connect` | 세션당 연속 스트리밍 최대 10분 |

출처: [모델 카드](https://ai.google.dev/gemini-api/docs/models/gemini-3.5-transcribe), [파일 전사 가이드](https://ai.google.dev/gemini-api/docs/transcribe), [Live 전사 가이드](https://ai.google.dev/gemini-api/docs/live-api/live-transcribe).

- 두 모델의 지원 언어 목록에 한국어 `ko-KR`이 명시되어 있다. 자동 언어 판별 및 언어 전환도 지원한다. 한국어 힌트는 `ko-KR` 사용 가능하나 한국어 품질 우위는 문서만으로 입증되지 않는다. [모델 카드](https://ai.google.dev/gemini-api/docs/models/gemini-3.5-transcribe), [Live 전사 가이드](https://ai.google.dev/gemini-api/docs/live-api/live-transcribe)
- Google 릴리스 노트는 **2026-08-26** 두 Transcribe 모델을 GA로 발표했다. 모델 ID에 `-preview`가 없다. 다만 Live 예시의 WebSocket 경로와 파일 전사 예시는 `v1beta` 프로토콜을 사용한다. **모델의 GA와 API 버전·보조 기능의 Preview 상태를 구분해야 한다.** [릴리스 노트](https://ai.google.dev/gemini-api/docs/changelog), [파일 전사 가이드](https://ai.google.dev/gemini-api/docs/transcribe), [Live 전사 가이드](https://ai.google.dev/gemini-api/docs/live-api/live-transcribe)
- Live는 `TEXT` 응답 모드, `inputAudioTranscription` 설정 후 16-bit PCM, 16 kHz, mono, little-endian 오디오를 실시간 전송한다. `interim_input_transcription`은 수정될 수 있는 중간 결과, `input_transcription`은 발화가 확정된 결과다. 화자 분리·단어별 타임스탬프는 Live에서 미지원이다. 파일 모델은 두 기능을 지원하지만 타임스탬프는 정확도를 떨어뜨릴 수 있고, custom vocabulary와 양립하지 않는다. [Live 전사 가이드](https://ai.google.dev/gemini-api/docs/live-api/live-transcribe), [모델 카드](https://ai.google.dev/gemini-api/docs/models/gemini-3.5-transcribe)

### 가격과 과금 단위

2026-10-02 열람한 [Google 가격표](https://ai.google.dev/gemini-api/docs/pricing)의 **Standard / Paid Tier** 기준. 두 모델 모두 표에 Free Tier 입력·출력 무료가 표시되지만, 이는 무제한이나 해당 프로젝트의 접근 가능성을 보증하지 않는다.

| 모델 | 오디오 입력 | 텍스트 출력 | 문서의 예시 합계 |
| --- | --- | --- | --- |
| `gemini-3.5-transcribe` | **$2.00 / 100만 입력 토큰**; 약 **$0.003 / 오디오 분** | **$12.00 / 100만 출력 토큰**; 약 **$0.002 / 전사 분** | 약 **$0.005 / 오디오 분** |
| `gemini-3.5-transcribe-live` | **$3.50 / 100만 입력 토큰**; 약 **$0.005 / 오디오 분** | **$21.00 / 100만 출력 토큰**; 약 **$0.004 / 전사 분** | 약 **$0.009 / 오디오 분** |

분당 값은 고정 정액이 아니라 **입력 오디오 25토큰/초(1,500토큰/분), 출력 텍스트 175토큰/분** 가정으로 환산한 추정치다. 실제 청구는 입력·출력 토큰 소비량에 따른다. 60분 예시 합계는 파일 약 **$0.30**, Live 약 **$0.54**이지만 Live에는 10분 세션 제한이 있어 한 세션 60분 처리는 불가능하다. 위 금액은 기준 토큰 속도를 60배 한 단순 추정이며 실제 한국어 텍스트량·세션 운영 비용을 반영한 견적은 아니다. [가격표](https://ai.google.dev/gemini-api/docs/pricing)

### 인증·구독·데이터 처리

- Gemini API 호출에는 프로젝트에 속한 API key가 필요하다. 파일 REST 예시는 `x-goog-api-key`, Live WebSocket 예시는 key를 URL 파라미터로 전달한다. key에는 독립 과금 설정이 없고 프로젝트의 tier·결제 상태를 따른다. **개인용 Gemini 앱 구독만으로 API 유료 tier가 부여된다는 공식 근거는 확인하지 못했다.** API 유료 tier는 Cloud Billing을 프로젝트에 연결하는 별도 절차다. 계정별 결제 방식은 Prepay 또는 Postpay이며 Prepay 적용 시 최소 선결제 금액은 문서상 **$5 USD 상당**이다. [API key 가이드](https://ai.google.dev/gemini-api/docs/api-key), [Billing 가이드](https://ai.google.dev/gemini-api/docs/billing)
- 클라이언트가 Live에 직접 연결한다면 영구 API key 노출을 피하도록 Google은 백엔드에서 발급한 단기 **ephemeral token** 사용을 권한다. 이 토큰 기능 자체는 **Preview**이며 Live API 전용이다. 이것은 전사 모델의 GA 상태와 다른 구분이다. [Ephemeral tokens 가이드](https://ai.google.dev/gemini-api/docs/live-api/ephemeral-tokens)
- 일반적인 Unpaid Services에서는 제출한 콘텐츠와 응답을 Google 제품·서비스 개선에 사용할 수 있고 사람이 입력·출력을 검토할 수 있다. 민감·기밀·개인 정보를 보내지 말라는 문구가 있다. Paid Services에서는 프롬프트·파일·응답을 제품 개선에 사용하지 않지만, 남용 방지와 법적 요구 등을 위한 제한 기간의 로깅은 가능하다. 따라서 **유료 = 저장·검토가 전혀 없음**이라고 해석하면 안 된다. EEA·스위스·영국의 무료 이용에는 약관상 Paid Services 데이터 조건이 적용되는 지역 예외가 있다. 계정·프로젝트의 실제 약관 적용은 별도 확인이 필요하다. [Gemini API 추가 약관](https://ai.google.dev/gemini-api/terms)
- 파일 모델이 사용하는 Interactions API는 기본값 `store=true`로 Interaction 객체를 저장한다. 문서상 기본 보존 기간은 **Free 1일 / Paid 55일**이며, 요청에 `store=false`를 명시하면 이 상태 저장을 끌 수 있다. 단 `store=false`는 background 실행 및 이후 `previous_interaction_id` 연결과 양립하지 않는다. 이는 **제품 개선 사용 여부와 별도** 설정이다. [Interactions API 개요](https://ai.google.dev/gemini-api/docs/interactions-overview), [Logs and datasets](https://ai.google.dev/gemini-api/docs/logs-datasets)
- `store=false`만으로 파일 모델의 데이터가 모두 즉시 없어지지는 않는다. Files API에 올린 원본은 별도로 보관되므로 삭제하거나 만료될 때까지 남는다. Live API에서 session resumption을 설정하여 handle을 만들면 텍스트·오디오를 포함한 세션 상태가 최대 **24시간** 보존될 수 있다. Google의 ZDR 문서도 유료 서비스의 남용 감시 로깅은 별개라고 명시한다. [Zero data retention 가이드](https://ai.google.dev/gemini-api/docs/zdr)

### 쿼터·Preview 조건

- 일반 rate limit 차원은 RPM(분당 요청), 입력 TPM(분당 토큰), RPD(일당 요청)이며 **API key별이 아니라 프로젝트별**로 적용된다. RPD는 태평양 시간 자정에 초기화된다. 모델·tier별 활성 수치는 AI Studio의 **Rate limits**에서 확인하도록 공식 문서가 안내한다. 공개 문서에서 두 Transcribe 모델의 확정 RPM·TPM·RPD 숫자를 찾지 못했으므로 이 문서에 임의 수치를 쓰지 않는다. 계정 조회를 하지 않았으므로 무료 quota와 동시 Live 세션의 실제 허용량도 미확인이다. [Rate limits 가이드](https://ai.google.dev/gemini-api/docs/rate-limits)
- 유료 tier의 지출 기반 한도는 rolling **10분**당 Tier 1 **$10**, Tier 2 **$50**, Tier 3 **$200**이며 초과 시 `429 RESOURCE_EXHAUSTED`를 반환한다. 이는 두 모델의 RPM·TPM 수치를 뜻하지 않는다. Preview/experimental 모델은 일반적으로 더 제한적인 rate limit을 갖지만, 두 Transcribe 모델 자체는 GA로 발표되었다. [Rate limits 가이드](https://ai.google.dev/gemini-api/docs/rate-limits), [릴리스 노트](https://ai.google.dev/gemini-api/docs/changelog)

## 주의 사항

- 한국어 구어체·고유명사·잡음·혼합 언어에서의 정확도, 실시간 지연, 10분 세션 경계의 연속성, 실제 계정의 모델 접근·rate limit은 공식 문서만으로 판정할 수 없다. 사용자 음성의 외부 전송과 과금 설정을 허가받은 뒤에만 별도 실측이 가능하다.
- `smart` 전사는 군더더기 제거·문법 정리뿐 아니라 발화를 목록·숫자·날짜 형식으로 바꿀 수 있다. Ultra STT의 **Original/Minimal** 출력 정책에는 원문 변경이 될 수 있으므로 기본값인 `verbatim`을 우선 검토해야 한다. `smart`는 파일 모델에서 화자 분리·단어 타임스탬프와도 양립하지 않는다. [파일 전사 가이드](https://ai.google.dev/gemini-api/docs/transcribe), [Live 전사 가이드](https://ai.google.dev/gemini-api/docs/live-api/live-transcribe)
- 모델 GA 여부는 API 릴리스 노트를 기준으로 판단했다. 일부 [제품 소개 페이지](https://deepmind.google/models/gemini-audio/ai-transcription/)는 Preview로 표기하여 공식 페이지 사이에 상태 표기가 일치하지 않는다. 사용 프로젝트의 접근 가능성과 API 동작은 계약 테스트로 다시 확인해야 한다.

## 공개 성능 비교: 최고라는 주장을 그대로 받아들이면 안 되는 이유

### Artificial Analysis: 비실시간 STT

WER는 단어 오류율이며 낮을수록 좋다. 다음은 전체 순위표를 재현한 것이 아니라 비교를 위한 일부 모델이다.

| 모델 | WER | 중앙값 처리 배속 |
| --- | --- | --- |
| Gemini 3.5 Transcribe | **2.6%** | **88.3×** |
| MAI-Transcribe-2 | 2.0% | 373.9× |
| ElevenLabs Scribe v2 | 2.2% | 88.1× |
| Grok Voice Transcribe 2.0 | 2.3% | 175.5× |

Gemini가 정확도·속도 모두의 단독 1위는 아니다. 이 비교는 다른 모델의 한국어 지원·한국어 우위까지 뜻하지 않는다. [원본 결과](https://artificialanalysis.ai/speech-to-text/non-streaming)

평가는 영어 중심 데이터와 텍스트 정규화를 사용한다. 속도는 **10분 오디오**의 처리 시간을 기반으로 하므로 `3초 ÷ 88.3`을 짧은 받아쓰기 지연으로 해석하면 안 된다. 네트워크·요청 고정 비용과 AI 다듬기·커서 입력도 별도다. [평가 방법](https://artificialanalysis.ai/methodology/speech-to-text)

### Pipecat: 실시간 STT

| 모델 | Pooled semantic WER | TTFS P50 | TTFS P95 |
| --- | --- | --- | --- |
| Gemini 3.5 Transcribe Live | **2.24%** | **458 ms** | **532 ms** |
| Deepgram nova-3-general | 1.62% | 247 ms | 298 ms |
| Soniox stt-rt-v5 | 1.27% | 260 ms | 305 ms |
| AssemblyAI universal-3-6-pro | 0.96% | 307 ms | 401 ms |

각 모델 1,000개 샘플을 동일 파이프라인으로 비교한 결과다. TTFS는 발화 종료 기준의 확정 전사 지연이며 우리 앱의 **녹음 종료 → 실제 커서 입력** 시간과 동일하지 않다. 표의 공개 순서는 순위가 아니다. 의미 기반 WER는 문장부호·대소문자·군더더기·숫자 표기 차이를 무시하므로 Original의 형식 보존 평가는 별도로 필요하다. 참조 전사를 Gemini로 생성하고 사람이 검토하는 방식도 해석에 반영해야 한다. [원본 결과·방법](https://www.pipecat.ai/benchmarks/speech-to-text), [벤치마크 소스](https://github.com/pipecat-ai/stt-benchmark)

### 앱형 평가의 다른 결과

Superwhisper의 Gemini 평가 페이지는 8개 데이터셋 평균 WER **8.2%**, 텍스트 대기 **2.34초**를 보고한다. 한국어 단문 결과가 아니며 해당 앱에 실제 제공되는 모델이라는 뜻도 아니다. 평가 데이터·측정 구간에 따라 숫자가 크게 달라지므로 위 결과와 합쳐 순위를 만들거나 Ultra STT 실측으로 대체할 수 없다. [해당 평가](https://superwhisper.com/benchmarks/gemini-3-5-transcribe)

**확인되지 않은 것:** 한국어 일상 받아쓰기·영어 혼용·고유명사·부정 표현·숫자에서 현재 local Whisper large-v3 대비 품질 우위를 입증하는 동일 조건 결과. Google의 다국어 종합 수치는 한국어 단독 수치로 취급하지 않는다.

## 우리 앱의 실제 병목

검토한 기존 기록은 `../runtime/windows-updated.log`와 `../qa/macos-remote-mode-smoke.json`이다. **이번 조사에서 새로 실행한 Gemini 성능 테스트가 아니다.** 동일한 약 2.83초 한국어 QA 음성에 대한 기존 서버 측 기록 중 아래 구간을 사용했다.

| 모드 | STT | AI 단계, 인증 포함 | 서버 전체 |
| --- | --- | --- | --- |
| Original | 608 ms | 4,348 ms | 4,957 ms |
| Minimal | 569 ms | 3,903 ms | 4,472 ms |
| Clean | 630 ms | 4,133 ms | 4,763 ms |
| Structured | 607 ms | 5,586 ms | 6,194 ms |
| Professional | 560 ms | 8,808 ms | 9,368 ms |
| AI 비활성, 별도 요청 | 596 ms | 0 ms | 596 ms |

- **사실:** 해당 시료의 AI 단계가 STT보다 훨씬 길다. Medium/High와 `service_tier="priority"`, 전용 Codex 프로필을 사용한 기록에서도 이 차이가 남아 있다.
- **추론:** 이 요청들에서 STT 시간을 이론상 0으로 만들어도 AI 사용 전체 시간의 약 **6–13%**만 줄어든다. 외부 Gemini 요청이 현재 약 0.6초 STT보다 느리면 전체 시간은 늘 수도 있다.
- **한계:** 짧은 한 문장 기반 QA이며 실제 사용자 전체의 P50/P95가 아니다. 서버 전체 시간에는 녹음 길이·Mac 업로드·클라이언트 수신·커서 삽입 지연이 포함되지 않는다.
- **제안:** 큰 체감 개선을 원한다면 STT 모델 교체와 별도로 **추가 AI 호출 제거 가능성**을 검증해야 한다. 이를 위해 품질 보존 요구를 몰래 낮추지 않는다.

## 적용 경로별 효과와 정책 충돌

| 경로 | 처리 | 기대 효과, 실측 전 가설 | 기존 기능 보존 |
| --- | --- | --- | --- |
| A. 파일형 Verbatim + 기존 Codex | 녹음 종료 후 전사, 기존 스타일 적용 | 품질 비교와 adapter 계약 검증에 적합. 속도 이득은 작거나 음수일 수 있음 | 다섯 스타일 유지 가능 |
| B. Live Verbatim + 기존 Codex | 녹음 중 전사, 종료 시 확정 후 기존 스타일 적용 | 긴 발화의 종료 후 STT 대기를 줄일 가능성. AI 지연은 남음 | 다섯 스타일 유지 가능 |
| C. Live/파일형 Smart 직접 출력 | 전사 모델의 정리 결과를 출력, 별도 Codex 생략 | AI 호출 한 단계를 없애므로 가장 큰 개선 잠재력 | **기존 Original/Minimal과 동일하지 않음** |

현재 `src-tauri/src/llm/prompt.rs`의 Original은 새 문단·목록·제목 생성과 어순·말투 변경을 금지한다. Minimal도 재작성·재구성을 금지한다. 따라서 Smart를 이 두 모드에 자동 연결하는 것은 기존 약속을 어길 수 있다. 반대로 Verbatim은 군더더기를 그대로 남기므로 그것만 직접 출력해도 Original과 같지 않다.

권고는 A로 품질·API 계약을 먼저 확인하고, B와 C를 **별도 선택형 실험**으로 측정하는 것이다. C는 예를 들어 `Gemini Smart / 빠른 받아쓰기`라는 명확한 별도 출력 경로로만 제공하고 사용자 동의를 받는다. 기존 스타일·선택 텍스트 편집·번역·질문은 유지하며, Smart가 다섯 스타일과 복잡한 음성 명령을 모두 대체한다고 가정하지 않는다.

## 소스 기준 호환성·수정 범위

| 현재 파일 | 확인한 구조 | 구현 시 필요한 변경, 아직 미적용 |
| --- | --- | --- |
| `src-tauri/src/stt/mod.rs` | connect/send_audio/recv_transcript/disconnect와 Partial/Final 이벤트가 있는 provider 구조 | Gemini 전용 adapter 등록. 기존 인터페이스로 파일형·Live형의 외부 이벤트를 정규화 |
| `src-tauri/src/stt/whisper_compat.rs` | WAV multipart + Bearer 인증 + Whisper `text` 응답 계약 | **URL·모델명만 바꾸어 연결 불가.** Gemini JSON·인증·응답 parser 별도 필요 |
| `src-tauri/src/audio/capture.rs` | 16 kHz mono PCM16 출력 | Live 입력 형식과 호환. 마이크 기본값·권한 동작은 유지 |
| `src/lib/constants.ts`, `src/stores/appStore.ts` | Gemini는 LLM 선택지에만 있고 STT provider에는 없음 | STT용 타입·표시·설정·오류 안내·번역 문자열 추가. LLM Gemini 선택과 혼동 금지 |
| `src-tauri/src/credentials.rs` | provider별 OS credential 저장 | STT 전용 API key 이름 공간 사용, 평문 설정·로그 노출 금지 |
| `src-tauri/src/pipeline.rs` | 녹음 스트림·취소·세션 ID·원격 결과의 중복 다듬기 방지 | 취소 시 전송 종료, 늦게 온 결과 폐기, Final만 삽입. Smart 경로의 중복 AI 호출 방지 |
| `src-tauri/src/remote.rs` | 중앙 받아쓰기 서버가 `ultra_local::transcribe(...,"ko")`와 Codex를 직접 호출 | **Mac의 현재 원격 경로는 UI provider 선택만 바꿔도 바뀌지 않음.** 서버 backend 선택 정책을 명시적으로 추가해야 함 |

`reqwest`, `tokio-tungstenite`, `serde_json`, `base64`는 이미 Rust 의존성에 있다. 현재 범위에서 새 Google SDK가 반드시 필요한 것은 아니다. 이것은 소스 기반 설계 판단이며 adapter를 실제 구현·검증한 결과는 아니다.

파일형은 짧은 오디오의 inline 전달 가능성과 현재 request schema를 계약 테스트로 확인한다. Files API를 사용하면 업로드·후속 요청·정리의 추가 비용과 보관 수명이 생긴다. REST 응답은 Whisper의 단일 `text`가 아니라 Interaction의 model output 콘텐츠를 파싱해야 한다. [공식 REST 예시](https://ai.google.dev/gemini-api/docs/transcribe), [Google 공개 구현](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/JotCore/Sources/TranscriptionClient/GeminiClient.swift)

Live는 녹음 중 오디오를 보내고 partial은 실행창에만 표시한다. 수동 push-to-talk는 VAD를 끄고 activityStart/activityEnd를 녹음 시작·종료에 연결하여 불필요한 무음 대기를 피하는 설계가 적합하다. 종료 제어 메시지는 마지막 오디오보다 먼저 보내면 안 된다. 최종 발화까지 확정됐는지 확인한 뒤 삽입한다. 10분 한도·빈 결과·quota·연결 단절·drain timeout은 명시적으로 처리해야 한다. [Live 가이드](https://ai.google.dev/gemini-api/docs/live-api/live-transcribe)

Mac의 현재 중앙 처리 방식을 유지하면서 B를 제공하려면 **Mac → 중앙 서버로 녹음 중 오디오를 전송하는 프로토콜**까지 검토해야 한다. 지금의 녹음 종료 후 업로드 경로로는 Live의 이점을 그대로 얻지 못한다. 이 API 계약 변경과 외부 오디오 전송은 별도 승인 대상이다. 모바일에는 영구 API key를 번들하지 않고 기존 서버 중계 또는 서버가 발급한 단기 토큰을 사용한다.

## Google 공개 앱 Jot에서 배울 점

Google 소유의 [Jot macOS 저장소](https://github.com/google-gemini/jot-gemini-transcribe-macOS)를 소스 수준으로 확인했다. 조사 버전은 `8f2d7a44d533dedc432351caa41227e65b0a4cec`, 라이선스는 Apache-2.0이다. 앱을 실행하거나 코드를 복사한 것은 아니다.

1. **마이크 스레드와 네트워크 분리:** audio callback이 소켓 응답을 기다리지 않도록 ring/queue를 사용한다. 종료 제어 메시지도 오디오와 같은 순서로 처리한다. [Live 세션 구현](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/JotCore/Sources/TranscriptionClient/Live/LiveTranscriptionSession.swift)
2. **누락 감지 후에만 결과 채택:** 녹음 프레임 수와 전송 byte 수를 대조하고, partial을 확정 전사·커서·히스토리에 쓰지 않는다. 실패 시 보존한 녹음으로 batch fallback한다. 우리 앱에도 전송 완전성 검사와 마지막 발화 확정 테스트를 제안한다. [Live 통합](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/JotCore/Sources/SessionCoordinator/LiveTranscribing.swift)
3. **속도 정책을 경로별로 구분:** 정리 pass 생략과 별도 정리 pass를 분리하며, 선택한 정책을 요청마다 고정한다. 정리에는 1.5초 상한과 원문 fallback이 있지만 이를 우리 앱에 그대로 적용하면 품질 우선 약속이 달라진다. 별도 동의 없이는 이 timeout 정책을 가져오지 않는다. [전사 service](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/JotCore/Sources/TranscriptionClient/GeminiTranscriptionService.swift)
4. **결과 검증:** 과도한 확장·축약·설명문 추가 등을 검출하고 원문으로 돌아간다. 영어 중심 임계값·숫자 정규화가 있으므로 그대로 복사하지 않고 한국어 고유명사·숫자·부정·어순 검증을 설계한다. 같은 텍스트를 자기 자신과 비교하는 검사를 품질 보장으로 취급하지 않는다. [검증 gate](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/JotCore/Sources/FormattingPipeline/ValidationGate.swift)
5. **API 호환성 위험 관리:** Interactions parser·timeout·429 처리를 별도로 구현한다. 소스 주석에는 일부 옵션 조합의 과거 probe 실패 기록이 있으나 이것은 작성자의 관찰이며 우리 환경의 현재 장애로 단정하지 않는다. 최신 공식 계약과 실제 승인된 probe를 함께 검증한다. [Gemini client](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/JotCore/Sources/TranscriptionClient/GeminiClient.swift)

서로 다른 엔드포인트의 스트리밍을 혼동하지 않는다. 파일 결과를 SSE로 받는 것과 마이크 오디오를 양방향 Live API로 보내는 것은 다르다. Swift/macOS 구현을 Windows/Rust에 통째로 이식하기보다 이미 있는 queue·세션 취소·provider 구조에 필요한 개념만 반영한다. 코드 복사 시에는 Apache-2.0 고지·라이선스 의무를 별도로 확인한다.

## 승인 후 수행할 최소 A/B 계획

아래는 **제안**이며 시험을 수행했다는 뜻이 아니다.

### 시험 입력

- 우선 공개·비민감 테스트 음성 40–60개. 실제 사용자 음성은 외부 전송에 동의한 범위만 사용한다.
- 3–5초 단문, 15–30초 발화, 1–3분 긴 발화; 한국어·영어 혼용·이름·제품명·숫자·날짜·부정·자가 수정·잡음·무음을 포함한다.
- 동일 음성을 현재 Whisper, Gemini 파일형 Verbatim, Live Verbatim, 별도 Smart 경로에 입력한다. Live는 실제 녹음 속도로 replay한다.
- 최소 3회 반복하고 첫 요청·warm 요청을 분리한다. 시험 순서를 섞어 시간대·warm-up 편향을 줄인다.

### 측정 지표

- 한국어 CER와 어절 WER를 함께 측정하고 정규화 규칙을 고정한다. 내용 정확도와 출력 형식 충실도를 별도로 판정한다.
- 이름·숫자·부정·기술 용어의 중대한 오인식/누락/환각, 뜻의 변경, Original의 어순·문단 보존을 사람이 검토한다.
- 마이크 stop부터 **실제 커서 입력**까지 P50/P95, STT final 대기, AI 시간, 연결/setup, 업로드, 첫 partial 시간을 따로 기록한다.
- 텍스트가 빨리 보이는 것과 최종 입력이 빠른 것을 분리한다. 비확정 partial을 먼저 붙여 넣어 좋은 수치처럼 만들지 않는다.
- 요청별 입력·출력 토큰, 재전송·fallback 비용을 집계한다. 오디오·키·문장 전문은 일반 운영 로그에 남기지 않는다.

### 실패·회귀 조건

- 잘못된 key/401·403, 429 transient와 quota 소진, timeout, 네트워크 단절, setup 실패, 빈 전사, 오디오 누락, 10분 세션 경계를 확인한다.
- 녹음 중 Esc와 처리 중 Esc 모두에서 결과 삽입·히스토리 반영이 없고, 다음 세션이 정상 작동해야 한다.
- 전사 종료 신호가 마지막 오디오를 앞지르지 않고, 이전 발화의 final만으로 끝났다고 판정하지 않아야 한다.
- provider·모드 전환 시 해당 요청의 정책을 유지한다. 원격 결과가 다시 Codex로 들어가지 않도록 검증한다.
- fallback은 사용자가 허용한 provider·데이터 전송·비용 범위 안에서만 제한적으로 실행한다. 외부 전송처를 몰래 바꾸거나 무한 retry하지 않는다.

### 적용 문턱 제안

기존 모드 대체라면 한국어 내용·형식 품질이 baseline보다 나빠지지 않고 중대한 의미 변경이 없어야 한다. **체감 속도 개선을 이유로 적용하려면 stop-to-cursor P95 30% 이상 감소**를 예시 목표로 삼되 사용자가 최종 기준을 결정한다. 이 값은 달성 예측이나 보장치가 아니다. Smart는 별도 정책 동의를 전제로 평가한다. 통과하지 못하면 실험 선택지만 유지하거나 철회하고 기본 Whisper·Codex 경로를 보존한다.

## 최종 권고와 작업 상태

**권고:** 현재 기본 모델을 즉시 교체하지 않는다. Gemini를 선택형 시험 provider로 먼저 연결하고, 우리 한국어 음성과 동일 모드에서 검증한다. 속도 목적의 핵심 시험은 추가 AI 단계를 없애는 **별도 Smart 경로**이며, 이를 기존 Original/Minimal로 위장하지 않는다.

실사용 음성에는 유료 프로젝트의 API key와 데이터 처리 안내를 우선 권고한다. 파일형은 `store=false`, 필요 없는 화자 분리·타임스탬프 비활성, 업로드 파일 정리를 검토한다. Live는 불필요한 session resumption을 사용하지 않는다. 이는 완전 무보관을 보장하지 않는다. 무료 tier는 비민감 시험부터 검토한다.

이번 작업은 조사·검토 문서 작성만 수행했다. **앱 코드·모델·인증·설정·의존성 변경, 유료 호출, 음성 외부 전송, 설치, commit·push는 하지 않았다.** 실제 구현과 비용 발생·사용자 음성 전송은 다음 단계에서 대상·한도·범위를 확인한 후 진행한다.
