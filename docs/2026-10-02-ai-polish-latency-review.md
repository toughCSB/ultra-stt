# AI 다듬기 지연 개선 — 경쟁 앱·로컬 LLM·Codex 구형 모델 검토

- 기준일: 2026-10-02. 공식 제품 문서·공급사의 공개 사례·모델 제작자 문서·공개 소스와 현재 Ultra STT 소스·기존 QA 결과를 조사했다.
- 범위: 조사와 적용 방안 검토. 모델 설치·다운로드·추론 호출·설정 변경·앱 변경·commit·push는 수행하지 않았다.
- 용어: 사용자 발화의 RRM은 문맥상 텍스트를 정리하는 LLM으로 해석했다. STT 모델의 교체와 AI 다듬기 LLM 교체는 별도 문제다.

## 결론부터

**사용자의 방향은 타당하다.** 단순 받아쓰기를 매번 높은 추론 강도의 개발용 에이전트로 처리할 필요는 없다. 다만 오래된 모델이면 무조건 빠르거나, 로컬 소형 모델이면 다섯 스타일·한국어 교정·선택 편집·번역을 모두 같은 품질로 수행한다는 보장은 없다.

권고 순서는 다음과 같다.

1. 구독·인증을 유지하면서 **현재 `gpt-6-luna`의 Low + Fast**를 일반 받아쓰기에 한정하여 품질·지연 비교한다. 아직 설정하지 않았다.
2. 구독 경로의 구형 비교군은 **`gpt-5.6-luna` Low + Fast**다. 현재 앱 전용 모델 캐시에 있지만 속도 우위는 미측정이다.
3. 로컬은 **Ollama + Qwen3-4B-Instruct-2507 Q4**를 첫 후보로 삼고, **Qwen3.5-4B non-thinking**을 품질 비교군으로 둔다. 기존 Ollama adapter를 재사용할 수 있다.
4. 추가 API 과금을 허용한다면 **`gpt-4o-mini` 직접 API**를 비교한다. Original/Minimal처럼 원문과 결과가 비슷한 작업에는 **Predicted Outputs**를 따로 시험한다.
5. 통과한 단순 받아쓰기 경로만 전환하고, 전문 용어 규칙·복잡한 편집·번역·질문은 검증된 기존 경로를 유지한다.

**중요한 제외:** `GPT-4o`는 현재 확인한 ChatGPT 로그인 Codex 모델 목록에 없고, `gpt-4.1-nano`는 2026-10-23 API 종료 예정이므로 신규 적용 후보에서 제외한다. Superwhisper의 공개 `S1-mini`는 영어 전용이므로 한국어 기본 모델로 채택하지 않는다.

## 경쟁 앱에서 실제 확인한 구조

이 절은 경쟁 앱의 공식 문서, 공급사의 직접 사례 연구, 공개 소스만 다룬다. **모델 이름이 공개되지 않은 부분은 미공개**로 둔다. `streaming`은 음성 전송·전사 중간 결과·LLM 토큰 출력 중 무엇을 뜻하는지 구분한다. 업체의 지연 주장이나 자체 벤치마크를 이 프로젝트의 실측치로 취급하지 않는다.

| 앱 | 공개된 다듬기 모델 / 구조 | 실행 위치와 streaming | 다듬기 pass / 지연 근거 |
| --- | --- | --- | --- |
| **Typeless** | 제3자 LLM 공급사로 OpenAI 등을 예시하지만 **다듬기 모델 ID와 ASR 모델 ID는 미공개**. [데이터 정책](https://www.typeless.com/data-controls) | 전사는 cloud. 음성과 제한된 앱·주변 텍스트를 cloud에서 처리한다고 명시. streaming 방식은 미공개. [데이터 정책](https://www.typeless.com/data-controls) | 별도 LLM pass인지, ASR과 통합인지 미공개. 정책의 “low-latency”는 수치나 측정 방법 없는 제품 설명. [데이터 정책](https://www.typeless.com/data-controls) |
| **Wispr Flow** | ASR은 자체 모델 **Canto**를 2026-09-17 전체 배포했다고 발표. 다듬기는 별도 **fine-tuned Llama 계열**이라고 Baseten 사례가 명시하지만 크기·정확한 모델 버전은 미공개. [Canto 발표](https://wisprflow.ai/whats-new), [Baseten 사례](https://www.baseten.co/resources/customers/wispr-flow/) | cloud ASR·LLM. Baseten/AWS 전용 배포에서 다듬기 추론. 입력 음성 또는 출력 토큰의 사용자 화면 streaming 방식은 자료만으로 확정할 수 없음. [기술 글](https://wisprflow.ai/post/technical-challenges), [Baseten 사례](https://www.baseten.co/resources/customers/wispr-flow/) | ASR → Llama 다듬기 단계가 공개됨. Baseten의 **p99 end-to-end <700ms**, Llama **100+ tokens <250ms**는 공급사가 보고한 측정 주장. 공개 원시 trace·동일 조건의 독립 재현은 확인되지 않음. [Baseten 사례](https://www.baseten.co/resources/customers/wispr-flow/) |
| **Superwhisper** | STT 모델 뒤 **선택적 LLM**인 `Super Mode`. 공식 모델 목록에 cloud `S1-Language`, Claude·GPT·Gemini 계열과 on-device 모델이 명시. 자체 `S1-mini`는 로컬 다듬기용으로 공개. [모델 목록](https://superwhisper.com/models), [S1 발표](https://superwhisper.com/blog/s1) | STT와 LLM 각각 cloud/local 선택 가능. cloud 호출은 Superwhisper proxy 경유, 로컬 LLM은 `llama.cpp`라고 설명. 실시간 전사 기능은 변경 이력에 있으나 특정 다듬기 모델의 토큰 streaming과 동일시할 근거는 없음. [모델 목록](https://superwhisper.com/models), [변경 이력](https://ai.superwhisper.com/changelog) | 일반적인 `Super Mode`는 STT → 선택적 LLM의 **두 단계**. 모델 페이지의 speed는 8개 데이터셋 기반 **자체 ASR 측정**이며 로컬 수치는 Apple M4 기준이다. LLM 다듬기 지연이나 말 멈춤→삽입 전체 시간의 측정치는 아님. [모델 목록](https://superwhisper.com/models) |
| **Spokenly** | mode별 STT 모델과 **별도 AI Instructions/provider** 선택. STT 예시는 GPT-4o Transcribe·Deepgram Nova·Soniox·Parakeet·Whisper·Apple Speech Analyzer, AI provider는 내장·OpenAI·Anthropic·OpenAI 호환 BYOK. 내장 다듬기 **기본 모델 ID는 미공개**. [Modes 문서](https://spokenly.app/docs/modes) | 로컬 STT 및 cloud STT 모두 지원. `Local Only Mode`에서 로컬 LLM endpoint 사용 가능. live transcription 지원 사실은 문서에 있으나 AI 다듬기 단계의 streaming은 확인되지 않음. [Local Only 문서](https://spokenly.app/docs/local-only-mode), [Dictionary 문서](https://spokenly.app/docs/custom-dictionary) | AI Instructions를 쓰면 전사 뒤 AI 재작성 단계가 있으며 생략 가능. 공식 스크립트 문서는 짧은 받아쓰기에서 이 단계를 건너뛰는 예시를 제공한다. 앱 전체 지연 실측치 미확인. [Modes 문서](https://spokenly.app/docs/modes), [Bash Scripts 문서](https://spokenly.app/docs/modes/bash-scripts) |
| **OpenTypeless (외부 upstream)** | STT 뒤 선택적 LLM polish. BYOK OpenAI 호환 API, 관리형 cloud, Ollama/로컬 endpoint를 선택할 수 있다. README 추천 조합은 Groq `whisper-large-v3-turbo` + Google `gemini-2.5-flash`이며 **고정 기본 모델의 증거는 아니다**. [upstream README 고정 SHA](https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/README.md) | upstream `pipeline.rs`는 STT 완료 뒤 LLM polish를 호출하고 LLM chunk의 점진적 삽입 경로를 둔다. 이는 **LLM 출력 streaming**이며 STT와 LLM이 한 모델/한 pass라는 뜻은 아니다. [upstream pipeline 고정 SHA](https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/src-tauri/src/pipeline.rs#L1972), [upstream cloud LLM 고정 SHA](https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/src-tauri/src/llm/cloud.rs#L114) | STT → 선택적 LLM의 두 단계. upstream의 timing log 항목은 계측 코드일 뿐, 공개된 대표 지연 측정값은 아님. 이 행의 source는 **`tover0314-w/opentypeless`의 `72914e8db005d1811f46f2dc50bfd91d1660ae01`**이며, 현재 작업 트리 `toughCSB/ultra-stt` origin의 실행 결과를 증명하지 않는다. [upstream pipeline 고정 SHA](https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/src-tauri/src/pipeline.rs#L2012) |
| **VoiceDash** | 공식 페이지는 STT + AI 편집을 설명한다. 개인정보 정책은 OpenAI API를, DPA는 OpenAI와 Groq를 처리업체로 명시하므로 **실제 요청별 공급사·모델 ID는 확정 불가**. [기능](https://voicedash.ai/features/), [개인정보 정책](https://voicedash.ai/privacy-policy/), [DPA](https://voicedash.ai/dpa/) | 공식 개인정보 정책상 음성이 OpenAI로 전송되는 cloud 경로가 있다. “real-time”이라는 표현은 있으나 음성·전사·LLM 중 어느 단계의 streaming인지 미공개. [개인정보 정책](https://voicedash.ai/privacy-policy/), [기능](https://voicedash.ai/features/) | 별도 AI pass 여부와 검증 가능한 전체 지연 수치 미공개. “instantly”/“real-time”은 이 자료에서는 마케팅 표현으로만 취급. [FAQ](https://voicedash.ai/faq/), [기능](https://voicedash.ai/features/) |
| **Google Gemini Jot 공개 데모** | `google-gemini/jot-gemini-transcribe-macOS`의 공개 소스 기준: 기본 `gemini-3.5-transcribe`가 filler 제거·자가 정정 정리·목록 형식을 맡고, 선택적 톤 다듬기는 `gemini-3.5-flash-lite`. **Google의 상용 Jot 제품 전체를 대표한다는 뜻은 아니다.** [README 고정 SHA](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/README.md), [GeminiClient 고정 SHA](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/JotCore/Sources/TranscriptionClient/GeminiClient.swift#L17) | 로컬 캡처·FLAC 변환 뒤 사용자 Gemini API key로 cloud 호출. 기본 경로는 완료 응답을 기다리는 `interactions` 호출이며, LLM 토큰을 화면에 streaming한다고 볼 근거는 없음. [README 고정 SHA](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/README.md), [GeminiClient 고정 SHA](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/JotCore/Sources/TranscriptionClient/GeminiClient.swift#L287) | **기본 한 API pass**, 선택적 앱별 톤 pass를 켜면 두 번째 호출 뒤 검증·실패 시 원문 fallback. 소스의 cleanup `1.5s`는 hard deadline이고 `0.3s`는 주석에 적힌 probe median이지, 공개 재현 가능한 앱 end-to-end 벤치마크가 아니다. [서비스 고정 SHA](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/JotCore/Sources/TranscriptionClient/GeminiTranscriptionService.swift#L17), [기본 설정 고정 SHA](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/JotCore/Sources/Settings/SettingsStore.swift#L135) |

**비교 시 주의:** Wispr의 공급사 보고 `p99 <700ms`, Superwhisper의 자체 ASR speed, Jot 소스 주석의 cleanup probe는 측정 대상과 공개 검증 수준이 각각 다르다. 동일한 “말 멈춤→최종 삽입” 지표로 순위를 매길 근거가 없다. [Baseten 사례](https://www.baseten.co/resources/customers/wispr-flow/), [Superwhisper 모델 방법론](https://superwhisper.com/models), [Jot 서비스 고정 SHA](https://github.com/google-gemini/jot-gemini-transcribe-macOS/blob/8f2d7a44d533dedc432351caa41227e65b0a4cec/JotCore/Sources/TranscriptionClient/GeminiTranscriptionService.swift#L33)

### 결정적인 발견: 작은 전용 모델과 최적화된 추론

Wispr Flow의 공개 구조는 **로컬 PC에서 일반 Llama를 실행하는 방식이 아니라**, 받아쓰기 정리에 fine-tuning한 Llama를 Baseten/AWS의 전용 추론 환경에 배포하는 방식이다. TensorRT-LLM 및 다단계 inference 최적화를 사용한다. 정확한 Llama 크기·버전·가중치는 공개 자료에서 확인하지 못했다. 따라서 일반 Llama를 내려받는 것만으로 같은 품질·속도가 재현되는 것은 아니다. [공급사 사례](https://www.baseten.co/resources/customers/wispr-flow/)

Superwhisper는 실제 로컬 정리용 **`S1-mini`**, Qwen3-0.6B 기반 모델의 가중치를 공개했다. 양자화 파일 약 462 MiB, **v1은 영어만 지원**, 일반 지시를 따르는 chat model이 아니라 정해진 입력 형식·스타일 제어로 작동하는 전사 정규화 모델이다. Apache-2.0 기반 추가 명명 조건과 NOTICE 의무도 있다. **한국어 전체 기능의 즉시 대체재는 아니지만, 장기적으로 한국어 전용 소형 정리 모델을 만들 때 참고할 구조다.** [제작자 모델 카드](https://huggingface.co/superwhisper/s1-mini)

이 두 사례가 보여 주는 것은 “범용 대형 모델을 High로 호출”하는 대신 **단일 목적에 맞춘 모델·출력·서빙**을 사용한다는 점이다. 구체적인 speed 수치는 공급사 주장과 자체 평가이며 한국어 Ultra STT의 측정 결과가 아니다.

## 우리 앱: 지연 원인과 이미 해 본 최적화

### 현재 실행 계약

`src-tauri/src/llm/codex.rs`를 확인한 결과:

- 앱 전용 `CODEX_HOME`, 사용자 설정 무시, 도구·skills 지침·project 문서 제외, ephemeral CLI 실행을 이미 사용한다. 개발 Codex의 모든 사용자 지침을 지금도 무제한으로 가져오는 구조라고 설명하면 틀리다.
- 허용 모델이 **`gpt-6-luna`로 제한**되어 있다. 설정의 모델 문자열만 다른 값으로 바꾸면 adapter에서 거부한다.
- 짧은 일반 Original/Minimal/Clean은 Medium, 선택 편집·번역·장면 지침·사전/교정 규칙·복잡한 스타일 등은 High다. 모든 요청의 서비스 tier는 현재 priority다.
- 매 다듬기 요청마다 `login status`와 `exec` 프로세스를 실행한다. 인증 사전 확인과 모델 처리 시간은 따로 기록하지만 `exec_ms`는 프로세스 시작·네트워크·서버 대기·추론 등을 합친 값이다.

기존 `../runtime/windows-updated.log`의 짧은 한국어 QA에서는 인증 preflight 약 **81–97ms**, Medium exec 약 **3.6–4.3초**, High 복잡 스타일 exec 약 **5.5–8.7초**였다. 인증 확인만 없애서 수 초가 사라질 것처럼 말할 수 없다. STT 약 0.6초보다 AI 단계가 더 크다는 기존 판단은 유지한다. 이는 새로운 벤치마크가 아니며 실제 사용자 전체 분포도 아니다.

### 중요한 반례: 상주 Codex만으로 이미 해결되지 않았다

`../qa/app-server-probe.json`에는 기존 상주 app-server 시험 **5,474 / 6,504 / 4,529ms**가 남아 있다. 각 결과는 사실 보존을 통과했지만 명확한 속도 개선은 없었다. `reasoningOutputTokens`가 세 요청 모두 **0**이었다는 점도 중요하다. 적어도 이 단문 시료에서는 표시상 High의 긴 사고 토큰만이 지연 원인이라고 단정할 수 없다.

`../HANDOFF.md`에는 initialization 376ms 및 격리 조건 차이 때문에 기존 verified CLI를 유지했다고 기록되어 있다. 이 시험은 현재 전용 프로필·Fast와 같은 조건의 대규모 A/B가 아니므로 app-server의 무효를 일반화하지는 않는다. 그러나 **같은 방법을 다시 제안하면서 새로 발견한 확실한 해결책이라고 말하지 않는다.**

공식 app-server는 상주 프로세스와 turn 취소를 지원한다. 재검토한다면 현재 프로필·동일 모델·동일 effort/tier·동일 프롬프트로 비교하고, 요청마다 독립 thread를 사용하여 받아쓰기 내용이 다음 요청에 섞이지 않게 해야 한다. 인증 토큰을 추출해서 비공개 endpoint를 직접 호출하는 방식은 사용하지 않는다. [공식 app-server 문서](https://learn.chatgpt.com/docs/app-server)

### 품질을 무턱대고 낮출 수 없는 이유

`../HANDOFF.md`의 분리 프로필 인계와 `../qa/codex-isolated-quality-terminology.log`에는 사전 표기 관련 실패가 있다. `../qa/codex-isolated-quality-high-terms.log`에서는 High와 terminology 우선 지침 적용 뒤 통과했다. 따라서 **모든 요청을 Low나 tiny 모델로 일괄 전환하는 것은 이전 실패를 되살릴 수 있다.** 일반 받아쓰기와 복잡·용어 민감 요청을 나누어 비교해야 한다.

## Codex 구형 모델: 무엇을 실제로 선택할 수 있나

### ChatGPT 구독 Codex와 유료 API는 별개

“코덱스 4 옴니”라는 별도 모델 ID는 확인하지 못했다. 사용자가 말한 대상은 **GPT-4o의 o = omni**를 의미할 가능성이 있다. GPT-4o/4o-mini가 API에 있다는 사실은 ChatGPT 로그인 Codex에서 선택할 수 있다는 뜻이 아니다. [GPT-4o-mini 공식 문서](https://developers.openai.com/api/docs/models/gpt-4o-mini), [Codex 모델 안내](https://learn.chatgpt.com/docs/models)

현재 앱 전용 `../runtime/codex-app-home/models_cache.json`의 공개 필드를 확인했다. 캐시 `fetched_at=2026-10-02T07:51:20.324576Z`, `client_version=0.160.0`. 인증 파일·토큰은 조회하지 않았으며 새 모델 호출도 하지 않았다.

| 구독 Codex 캐시에 보이는 모델 | 조사 판단 |
| --- | --- |
| `gpt-6-luna` | 현재 앱 모델. 단순 정리의 첫 비교는 Low + Fast |
| `gpt-5.6-luna` | 구형 소형 비교 후보. 현재보다 빠르다는 증거는 아직 없음 |
| `gpt-5.6-terra`, `gpt-5.6-sol`, `gpt-5.5` | 목록에는 있으나 오래됐다는 이유만으로 빠른 정리 모델로 추천하지 않음 |
| `gpt-6-sol`, `gpt-6.1-sol`, `gpt-6-astra` | 복잡 작업 후보. 일반 받아쓰기 속도 목적의 첫 선택은 아님 |
| `gpt-4o`, `gpt-4o-mini`, `gpt-4.1-nano` | 해당 캐시에 없음. 구독 경로에서 사용 가능하다고 주장하지 않음 |

공식 안내는 Luna를 반복적·집중된 작업에 적합한 효율 모델로 설명하며, Low를 짧고 명확한 작업에 맞는 옵션으로 안내한다. 모델 목록은 계정·클라이언트·rollout에 따라 달라진다. 캐시 등재는 실제 요청 성공이나 서비스 최속 순위를 보증하지 않는다. [공식 모델 안내](https://learn.chatgpt.com/docs/models)

**API와 CLI 설정 차이:** 공식 `gpt-6-luna` API는 `reasoning.effort=none`을 지원한다. 그러나 이번에 확인한 구독 Codex 캐시의 Luna effort는 **low/medium/high/xhigh/max**이며 `none`은 없다. API 문서의 `none`을 CLI에도 된다고 임의 적용하지 않는다. [API 모델 문서](https://developers.openai.com/api/docs/models/gpt-6-luna)

### 오래된 API 모델의 실용성

| API 후보 | 장점 | 한계·상태 |
| --- | --- | --- |
| `gpt-4o-mini` | 빠른 집중 작업용 소형 모델. 직접 HTTP로 호출하여 Codex agent의 프로세스·문맥 경로를 우회할 수 있음 | API key·별도 청구 필요. 한국어 품질와 최종 지연 실측 필요 |
| `gpt-4.1-nano` | 비추론·저지연 모델로 공개되었음 | **2026-10-23 종료 예정**, 신규 적용 추천하지 않음 |
| GPT-4/3.5 및 옛 Codex 계열 | 일부 단순 정리를 할 능력은 있을 수 있음 | 종료·비권장 상태와 계정 접근 문제. “구형=최속” 근거 없음 |

출처: [GPT-4o-mini](https://developers.openai.com/api/docs/models/gpt-4o-mini), [GPT-4.1-nano](https://developers.openai.com/api/docs/models/gpt-4.1-nano), [API 종료 일정](https://developers.openai.com/api/docs/deprecations).

GPT-4o-mini 공개 Standard 가격은 입력 **$0.15 / 100만 토큰**, 출력 **$0.60 / 100만 토큰**이다. 단순 계산 예시로 요청당 입력 1,000 + 출력 200토큰이면 약 **$0.00027**, 동일 요청 1,000회 약 **$0.27**다. 실제 프롬프트 길이·출력·서비스 옵션·재시도로 달라지며 구독에 포함된 무료 호출이라는 뜻이 아니다. 현재 Codex 입력 토큰량을 그대로 이 예시와 동일시하지 않는다. [공식 가격 포함 모델 문서](https://developers.openai.com/api/docs/models/gpt-4o-mini)

### 단순 정리에 특히 맞는 Predicted Outputs

Original/Minimal은 결과의 상당 부분이 이미 STT 원문에 있다. 직접 API의 `prediction`에 원문을 제공하여 변경 없는 출력 부분을 빠르게 생성하는 **Predicted Outputs**는 작업 특성에 맞는 후보다. 공식 지원 모델에 GPT-4o-mini가 포함된다. 결과를 원문으로 강제하는 기능은 아니며 재구성·번역처럼 수정이 큰 작업에서는 이점이 작을 수 있다. [공식 가이드](https://developers.openai.com/api/docs/guides/predicted-outputs)

주의할 계약 두 가지:

- 거절된 prediction 토큰도 출력 요금으로 청구될 수 있어 지연·비용을 함께 비교한다.
- 현재 앱의 직접 OpenAI builder는 `max_completion_tokens`를 항상 넣지만, 해당 가이드는 prediction 사용 시 이 필드가 미지원이라고 명시한다. **그냥 prediction 한 줄을 추가하면 안 된다.** 별도 request 계약·오류·출력 상한 정책을 검증해야 한다.

현재 기본 OpenAI 모델 문자열이 4o-mini라는 사실은 설치된 개인용 Codex 경로에서 실제로 그 모델을 사용한다는 뜻이 아니다. 소스 `src/lib/constants.ts`와 런타임 provider 선택을 구분한다.

## 간단한 로컬 LLM 방안

### 실제 하드웨어 확인

이번 조사에서 읽기 전용으로 확인했다.

| 환경 | 확인값 | 적용 판단 |
| --- | --- | --- |
| Windows | Ryzen 5 7600, 사용 가능 물리 RAM 약 63.2 GiB, RTX 3070 **8 GiB VRAM** | 4B Q4 모델을 평가할 사양은 있지만 동시 작업의 GPU 메모리 여유가 핵심 |
| Windows GPU 조회 시점 | free **1,678 MiB**, utilization 21% | 2.5–3.4GB 모델을 지금 완전 GPU 상주할 여유가 있다고 판단할 수 없음 |
| Mac | `ssh mac`의 sysctl 결과 **Apple M1 Pro / 16 GiB** | 4B Q4의 로컬 후보. OS·다른 앱·KV cache를 포함한 여유와 실측 필요 |
| 로컬 실행 도구 | Windows/Mac PATH에서 Ollama가 확인되지 않음 | 다른 위치 설치 여부는 단정하지 않음. 설치·기동·모델 다운로드는 하지 않았음 |

GPU free는 순간값이다. RAM 64GB가 GPU VRAM 8GB를 대체하는 것도 아니다. CPU offload로 실행은 가능할 수 있지만 지연이 오히려 늘 수 있다. 기존 Whisper·작업 앱을 임의 종료하여 메모리를 확보하지 않는다.

### 후보 선택

| 후보 | 크기·실행 특성 | 권고 |
| --- | --- | --- |
| **Qwen3-4B-Instruct-2507** | 전용 non-thinking instruct. Ollama `qwen3:4b-instruct-2507-q4_K_M` 파일 **2.5GB**, Apache-2.0 | **가장 단순한 로컬 첫 비교군**. Thinking 오작동 위험이 작은 경로 |
| **Qwen3.5-4B** | Ollama `qwen3.5:4b` 파일 **3.4GB**, thinking/non-thinking 선택, Apache-2.0 | 품질 비교군. Thinking 비활성 동작과 런타임 호환성 확인 후 시험 |
| EXAONE-4.0-1.2B | 한국어 포함 초소형 후보, 제작자가 한국어 1.2B의 낮은 temperature를 안내 | 개인 평가 보조 후보. **NC 라이선스**이므로 공개·상용 배포 기본값으로 삼지 않음 |
| Superwhisper S1-mini | Qwen3-0.6B를 전사 정규화에 특화, 약 462 MiB | **영어 전용 v1**. 한국어 대체 후보 제외, fine-tuning 구조만 참고 |

출처: [Qwen3 Instruct 모델 카드](https://huggingface.co/Qwen/Qwen3-4B-Instruct-2507), [Qwen3 Q4 파일](https://ollama.com/library/qwen3:4b-instruct-2507-q4_K_M), [Qwen3.5 모델 카드](https://huggingface.co/Qwen/Qwen3.5-4B), [Qwen3.5 Q4 파일](https://ollama.com/library/qwen3.5:4b), [EXAONE 제작자 카드](https://huggingface.co/LGAI-EXAONE/EXAONE-4.0-1.2B), [S1-mini 제작자 카드](https://huggingface.co/superwhisper/s1-mini).

Qwen3 계열은 한국어를 포함한 다국어 학습을 공개했다. 하지만 일반 모델 카드의 능력 점수와 한국어 **원문 보존형 교정 품질**은 별도다. 위 파일 크기는 다운로드 크기이며 실제 상주 RAM/VRAM 소요량은 KV cache·context·runtime 등으로 더 커진다. [Qwen3 제작자 설명](https://qwenlm.github.io/blog/qwen3/)

### 가장 간단한 운영 모양

```text
기존 마이크 → 기존 STT → localhost의 상주 Ollama → 현재 커서 삽입
                              └ 일반 Original/Minimal 먼저 시험
복잡한 편집·번역·용어 민감 요청 → 검증된 기존 Codex 경로 유지
```

로컬 LLM만 쓰는 경우 **AI 다듬기 텍스트**는 PC/Mac에서 처리된다. STT가 외부 또는 중앙 서버라면 전체 음성 파이프라인이 완전 오프라인이라는 뜻은 아니다.

기존 앱에 `ollama` 선택지, keyless credential 판단, `http://localhost:11434/v1` OpenAI 호환 adapter가 있다. 새 엔진을 앱 안에 번들하는 대신 이를 재사용하는 방식이 가장 간단하다. 최초 설치·2.5GB 모델 다운로드·필요한 자동 실행은 별도 승인을 받고 진행한다.

속도를 위해 모델을 **상주·예열**하고 cold/warm을 분리한다. Ollama 기본 모델 유지 시간은 5분이며 native API `keep_alive` 또는 서버 환경 설정으로 제어할 수 있다. 현재 앱의 OpenAI 호환 요청에는 해당 필드가 없으므로 그대로 전달 가능하다고 가정하지 않는다. [Ollama FAQ](https://docs.ollama.com/faq), [OpenAI 호환 문서](https://docs.ollama.com/api/openai-compatibility)

Qwen3.5는 모델 이름만 바꾸고 기본 thinking을 켜 두면 오히려 느려질 수 있다. 최신 Ollama는 native `think`와 호환 API `reasoning_effort`를 제공하지만 지원 값은 `/api/show`로 확인해야 한다. 현재 앱 request builder에는 effort 제어가 없어 **명시적으로 non-thinking 요청을 보낼 수 있는지** 구현·검증해야 한다. 최초 non-thinking 전용 Qwen3 Instruct를 권고한 이유다. [Ollama chat 계약](https://docs.ollama.com/api/chat), [호환 effort 규칙](https://docs.ollama.com/api/openai-compatibility)

Mac 전용 최적화 대안은 Apple Silicon의 MLX-LM이다. 다만 Windows와 동일한 설치·운영 경로를 원하면 Ollama부터 비교하는 편이 단순하다. 현시점에서는 MLX를 새로 설치하지 않았다. [MLX-LM 프로젝트](https://github.com/ml-explore/mlx-lm)

### 현재 수준 전체를 낼 수 있는가

**미확인이다.** 로컬 4B는 군더더기·문장부호·명백한 오타 같은 제한적 정리의 유망 후보다. 그러나 다음은 별도 통과가 필요하다.

- Original의 어순·말투·문단·의도적 반복 보존과 불확실한 고유명사를 추측하지 않기.
- 자가 정정과 의도적 대조 구분, 부정 표현·금액·날짜·이름·영어 혼용 보존.
- 사전 canonical spelling과 교정 규칙 준수.
- 다섯 스타일의 차이, 선택 텍스트 수정·번역·질문·사용자 장면 지침.
- 입력의 지시문을 실제 명령으로 따르지 않는 transcript/selected-text 경계.

특히 복잡 작업까지 4B로 한꺼번에 바꾸는 것은 추천하지 않는다. 속도 목표의 첫 범위는 **일반 받아쓰기**이며 기능 범위를 줄이거나 규칙 기반 정리만으로 대체하는 것은 별도 동의가 필요하다.

## 우리 소스에 적용할 때의 최소 변경

| 변경 대상 | 필요한 일, 아직 미적용 |
| --- | --- |
| `src-tauri/src/llm/codex.rs` | 승인된 Low 비교 옵션·모델 allowlist를 추가하되 현재 default와 High 예외를 보존. 계정 catalog와 effort 계약 확인 |
| `src-tauri/src/llm/protocol.rs`, `openai.rs` | provider별 non-thinking/Predicted Outputs 계약을 분리. 대충 모든 provider에 같은 파라미터 추가 금지 |
| `src/lib/constants.ts`, 설정 UI | 로컬 모델 이름·성능/비용/오프라인 범위를 정확히 표시. 오래된 기본값 문자열과 실제 사용 모델 구분 |
| `src-tauri/src/pipeline.rs` | 작업 종류에 맞는 engine routing, 세션/취소/오류/출력 상한 유지. 짧은 스트리밍 체감과 최종 완료 지연을 별도로 계측 |
| `src-tauri/src/remote.rs` | 현재 중앙 Mac 경로의 Codex 하드코딩을 명시적 backend 선택으로 연결. **UI에서 Ollama를 선택하는 것만으로 Mac 원격 경로는 바뀌지 않음** |

두 가지 Mac 운영 선택이 있다. 기존 중앙 처리를 유지하고 Windows에서 로컬 다듬기까지 제공하거나, 중앙 STT의 raw 결과를 Mac에서 로컬 LLM으로 정리하는 방식이다. 후자는 현재 요청/응답·중복 다듬기 방지 정책을 다시 검토해야 한다. localhost Ollama를 인터넷·Tailnet에 인증 없이 직접 공개하지 않는다. API/auth/배포 설정 변경은 승인 뒤 진행한다.

**출력 제한 최적화:** 현재 일반 LLM 요청은 `max_tokens=4096`이다. 단문에는 넉넉한 상한이지만, 상한만 256으로 낮춘다고 실제 30토큰 응답 생성이 자동으로 빨라지는 것은 아니다. 입력에 비례한 안전한 한도·length 종료 감지·충분한 여유를 설계하고 잘린 문장을 붙여 넣지 않는다. 모델이 실제 생성하는 추론·출력 토큰과 request 수를 줄이는 것이 핵심이다. [공식 지연 최적화](https://developers.openai.com/api/docs/guides/latency-optimization)

## 승인 후 비교할 최소 실험

1. **Baseline:** 현재 Luna Medium/High + Fast와 현재 프롬프트를 고정한다.
2. **구독 후보:** Luna Low + Fast, 5.6 Luna Low + Fast를 동일한 단순 받아쓰기 corpus로 비교한다. 사전·선택 편집·번역 등은 별도 복잡군으로 유지한다.
3. **로컬 후보:** Qwen3 Instruct Q4와 Qwen3.5 non-thinking Q4를 Windows/Mac에서 각각 cold/warm, STT 동시 실행 상태로 시험한다. GPU offload·모델 load·prompt eval·generation 시간을 분리한다.
4. **선택적 API 후보:** 4o-mini 직접 호출의 일반 요청과 Predicted Outputs를 비교한다. API key·지출 상한·외부 전송 범위를 승인받은 뒤에만 수행한다.
5. 기존 synthetic quality corpus에 이름·금액·부정·정정·의도적 반복·언어 혼용·인젝션 경계 등 **40–60개 한국어 사례**를 보강하고, 같은 입력을 최소 3회 반복한다. 의미·스타일 품질과 AI-only/stop-to-cursor P50/P95를 각각 평가한다.
6. 예시 목표는 **단문 다듬기 P95 1초 안팎 또는 baseline 대비 50% 감소**이며 달성 보장치가 아니다. 숫자·이름·부정 등 중대한 의미 변경이 발생하면 해당 경로는 기본값으로 채택하지 않는다.
7. Esc 중단, 모델 unload, 연결 단절, quota, timeout, 빈 응답, 길이 잘림, 늦게 도착한 이전 세션 결과를 시험한다. fallback에 의해 두 번 입력하거나 사용자 허용 없이 다른 외부 서비스로 전송하지 않는다.

API latency leaderboard의 초당 토큰 수만으로 이 실험을 대체할 수 없다. 짧은 문장에서는 연결·queue·prefill·한국어 tokenization·prompt·hardware 상황이 중요하다. 현재 사용자에게 “이 모델이 확실한 최속”이나 “로컬이 현재 품질을 모두 유지한다”는 결과는 아직 제시할 수 없다.

## 최종 판단

- **빠른 앱이 모두 로컬 모델을 쓰는 것은 아니다.** 전용 fine-tuned cloud 모델과 작은 로컬 모델이 모두 존재한다.
- **기존 구독 안에서의 첫 시도는 Luna Low + Fast**다. 구형 비교는 5.6 Luna이며 검증 없이 더 빠르다고 단정하지 않는다.
- **간단한 로컬 첫 시도는 Ollama + Qwen3-4B-Instruct-2507 Q4**다. Mac 사양은 후보에 적합해 보이지만 실행 속도는 미측정이고, Windows는 현재 GPU 여유부터 고려해야 한다.
- **옛 OpenAI 모델을 원한다면 4o-mini 직접 API가 현실적인 비교군**이다. 구독 Codex 모델 변경과 다른 인증·비용 경로다. 4.1 nano는 곧 종료되어 제외한다.
- 품질 우선 요구를 유지하므로 검증을 통과한 작업 종류만 전환한다. 장기적인 최적화는 공개·비민감 synthetic 데이터로 한국어 전용 소형 cleanup 모델을 평가·fine-tuning하는 방향이지만, 지금 바로 추가할 필수 작업은 아니다.

현재 앱 동작·설정·인증은 그대로다. 이 문서와 앞선 Gemini 검토 문서는 **조사 결과이며 실제 새 모델의 품질·속도 실측 보고서가 아니다.**
