# 개인 설치 성능 점검

## 기준과 보존 조건

2026-09-30 측정. 최초에는 **품질 유지 우선, 불필요한 대기만 제거**를 선택했고 아래 초기 측정은 High를 유지했다. 이후 사용자는 Fast 사용량 증가와 품질 비교 통과 시 **일반 받아쓰기 Medium + Fast / 복잡한 작업 High + Fast**를 승인했다. 모델, 인증과 출력 안전장치는 유지한다. 두 단계의 측정을 혼동하지 않는다.

- STT: 전체 Whisper large-v3, CUDA float16, beam size 5, VAD 유지.
- 다듬기: `gpt-6-luna`, 기존 ChatGPT 구독 인증 유지. 최신 자동 추론 정책은 아래 후속 조사 항목에 있다.
- Mac: 기존 인증된 tailnet 연결로 Windows에서 STT와 다듬기를 수행한다. 결과를 Mac에서 다시 다듬지 않고 기존 커서 입력 경로로 보낸다.
- 테스트 요청은 기존 앱 경로를 이용했다. 토큰, 개인 녹음과 전사 내용은 성능 보고서에 넣지 않았다.

## 적용한 개선

1. 앱 시작/설정 warm-up에서 정확한 managed Windows 로컬 STT 구성만 미리 준비한다. 기존에는 단순 HTTP 예열로 GPU 모델이 실제 준비되지 않았다. 설치본에서 모델 준비는 5594 ms였으며 첫 전사 요청 밖에서 진행됐다. 앱을 켠 직후 모델 준비가 끝나기 전에 녹음을 완료하면 여전히 준비를 기다릴 수 있다.
2. 로컬 STT `reqwest::Client`를 재사용한다. 실제 HTTP 요청 두 개가 TCP 연결 하나를 이용하는 회귀 테스트가 있다. 인증, 프록시 금지, 리디렉션 금지, 요청 제한과 GPU 프로세스 취소 동작은 유지한다.
3. STT, Codex 로그인 확인, 실행 및 원격 전체 시간을 안전한 메타데이터로 구분해 측정한다. 홈페이지에는 마지막 완료 작업의 기존 timing 이벤트를 표시한다. 녹음 시간은 전체 처리 시간에 포함하지 않는다.

홈페이지의 Windows STT/다듬기/전체와 Mac 원격 인식·다듬기/전체는 같은 분해 기준이 아니다. Mac에서는 서버 내부 두 단계의 합이 STT 요청 시간에 포함된다. 서버 로그를 사용해야 내부 STT와 AI를 별도로 비교할 수 있다.

## 실측 결과

Mac에서 동일한 2.83초 mono PCM16 16 kHz 한국어 WAV를 기존 서버에 순차 전송했다. 시간은 요청 전송부터 완성된 다듬기 결과 수신까지다. 녹음, 전역 단축키와 커서 삽입 시간은 별도이며 이 수치에 포함되지 않는다.

| 배치 | 성공 | 평균 | 중앙값 | 최소–최대 |
| --- | --- | --- | --- | --- |
| 변경 전 warm | 5/5 | 6279 ms | 5813 ms | 5417–7966 ms |
| 설치 후 첫 배치 | 5/5 | 7214 ms | 5129 ms | 4715–15870 ms |
| 설치 후 warm | 5/5 | 5815 ms | 5458 ms | 4991–6821 ms |

warm 평균은 관측상 약 7.4%, 중앙값은 약 6.1% 감소했다. 표본이 작고 원격 AI 응답 편차가 있으므로 인과관계나 반복 가능한 개선율을 확정할 수 없다. 변경 후 결과 10개는 문장 전체의 의미/문자 보존 검사까지 통과했으며 구두점 차이는 허용했다. 변경 전 5개는 성공 응답/기대 구절 확인으로 측정했다.

warm 서버 내부에서 STT는 625–745 ms, 다듬기는 4203–6069 ms였다. 로그인 확인은 121–159 ms 정도이며 나머지 대부분이 Codex 실행/AI 응답이다. 첫 배치의 15870 ms 결과는 다듬기 14748 ms, STT 1039 ms였다. 이 이상값을 제외하고 전체 지연이 해결됐다고 보고하지 않는다.

보고서:

- 프로젝트 루트 `qa/latency-before.json`
- 프로젝트 루트 `qa/latency-after-cold.json`
- 프로젝트 루트 `qa/latency-after-warm.json`
- 설치 앱 서버 메타데이터: 프로젝트 루트 `runtime/windows-updated.log`

## 재현

기존 서버와 로그인 상태, 모델 및 품질 옵션을 유지한다. 토큰 파일을 직접 출력하지 않는다. GPU 메모리가 제한되므로 병렬 테스트나 두 번째 large-v3 프로세스를 띄우지 않는다.

```sh
node qa/benchmark-remote.mjs AUDIO.wav TOKEN_FILE ENDPOINT REPORT.json 5
```

프로젝트 루트의 스크립트는 HTTP 성공, 오류 응답 부재와 기대 구절을 확인한다. `faithful`은 정해진 한국어 fixture의 전체 문장을 구두점/공백을 제외하고 비교한다. 다른 음성을 넣으면 이 고정 fixture 검사 결과를 품질 지표로 사용하지 않는다. 실패도 보고서에 기록되며 실패 배치는 성공-only 통계와 따로 판독해야 한다.

## 채택하지 않은 실험

10개 합성 텍스트의 사실 보존 corpus는 음성 인식이나 경쟁 앱 비교 테스트가 아니다. PS7로 실행한다. 다음 옵션은 QA용이며 앱 설정이나 전역 Codex 설정을 바꾸지 않는다.

```powershell
pwsh -NoProfile -File qa/check-cleanup.ps1 -Model gpt-6-luna
pwsh -NoProfile -File qa/check-cleanup.ps1 -Model gpt-6-luna -InstructionsFile qa/codex-dictation-instructions.md
```

| 실험 | 입력 토큰 | 전체 10문장 배치 시간 | 판단 |
| --- | --- | --- | --- |
| 기존 지침 | 16852 | 12497 ms | 유지 |
| 음성용 기본 지침 | 13476 | 13251 ms | 더 빠르지 않아 미적용 |
| 기존 지침 + host-skill skip 시안 | 16852 | 12916 ms | 입력 감소 없음, error 이벤트, 미적용 |
| 음성 지침 + host-skill skip 시안 | 13476 | 14172 ms | 입력 감소 없음, error 이벤트, 미적용 |

기본/음성 지침 출력의 이름, 금액, 날짜, 부정, 의도적 강조, 불확실성, 인용 명령과 할 일 수는 확인했다. 하지만 지연 개선 근거가 없었다. 개발 단계 skill-skip 플래그의 warning이 JSONL error 이벤트로 기록돼 엄격한 QA 검증도 실패했다. 운영 어댑터가 이런 이벤트를 무시하도록 약화하거나 전역 설정에서 경고를 숨기지 않는다.

## 조사와 남은 한계

공식 OpenTypeless [provider index](https://www.opentypeless.com/en/docs/provider-index)는 스트리밍 STT와 여러 LLM 제공자를 지원한다. [원본 저장소](https://github.com/tover0314-w/opentypeless)는 streaming polish 경로를 설명한다. 이는 현재 전체 음성 전송 + Codex CLI 최종 답변 경로와 구성상 다르지만, 같은 음성/모델/장비로 경쟁 앱을 측정한 것은 아니므로 특정 배수의 속도 차이를 주장할 수 없다.

[faster-whisper 공식 저장소](https://github.com/SYSTRAN/faster-whisper)는 batching 성능과 같은 beam size로 비교해야 한다는 조건을 제시한다. 긴 음성의 batch throughput을 짧은 단일 받아쓰기의 응답 지연으로 그대로 적용할 수 없다. 현재 STT는 1초 미만이며 지연의 대부분은 AI다. 모델 축소, beam 감소나 무조건적인 양자화는 이번 품질 유지 요청의 해결책으로 적용하지 않았다.

[Codex 공식 설정 문서](https://learn.chatgpt.com/docs/config-file/config-reference)의 `model_instructions_file`을 자식 프로세스에 한정해 실험했지만 속도 이득이 입증되지 않았다. 기존 모델/High를 유지하면서도 훨씬 큰 개선을 보장하려면 녹음 중 처리 또는 검증된 저지연 다듬기 경로에 대한 별도 구현·품질 평가가 필요하다. 새 제공자, 인증 방식, 의존성 또는 사용자 설정 변경은 이번 설치에서 수행하지 않았다.

**초기 설치 시점의 상태:** 제거 가능한 모델 첫 준비와 HTTP 연결 재생성은 개선했다. AI 응답의 수 초 지연 및 15초대 이상값은 남아 있었다. Typeless 수준 속도 달성이나 최신 Mac 실음성 커서 입력 재검증 완료로 표시하지 않는다.

## 후속 소스 조사와 승인된 Fast / 자동 추론 정책

VoiceInk, Handy, Voxa, Flow, OpenTypeless, Whispering(Epicenter), OpenDictate의 실제 STT→다듬기→출력 호출부와 Codex 내부 구현을 조사했다. 고정 full SHA, 함수/행 permalink, 라이선스, 후보별 실패·취소·출력 차이는 프로젝트 루트의 다음 보고서에 있다.

- `qa/research/20260930/native-apps.md`
- `qa/research/20260930/providers-and-commercial.md`

Flow 저장소는 Wispr Flow 제품의 소스가 아니다. Spokenly는 [공식 설명](https://spokenly.app/open-source)에서 앱 소스 비공개를 명시한다. Typeless의 OSS 라이선스 페이지는 의존성 공개이지 전체 앱 소스 공개의 증거가 아니다. Wispr Flow, Typeless, VoiceDash는 공식 전체 제품 소스를 확인하지 못했으며, 내부 구조를 읽었다고 주장하지 않는다. GPL/AGPL 구현을 복사하거나 새 의존성을 설치하지 않았다.

공개 앱의 유효한 패턴은 모델 사전 준비, HTTP 연결 재사용, 녹음 중 STT, 최종 결과 한 번 삽입, 짧은 다듬기 프롬프트와 추론 부담 축소였다. 사전 준비/연결 재사용은 이미 적용했다. 녹음 중 불안정한 부분 전사를 바로 다듬거나 붙이는 방식은 취소·문맥·부분 삽입 복구와 정확도에 영향을 줘 이번 설치에 도입하지 않았다. 현재 짧은 음성 STT는 1초 미만으로 AI가 주된 병목이다. 다른 앱의 모델/제공자가 다르므로 앱 간 속도 비교로 해석하지 않는다.

### 실제 적용 정책

- 같은 `gpt-6-luna`, 같은 ChatGPT 구독 인증, 같은 사전/교정/문맥/스타일/선택 원문/번역 프롬프트를 유지한다.
- 일반 `DictateInsert`, 선택 원문 없음, 번역 비활성, 사용자/장면 지침 없음, 사용자 사전 및 활성 교정 규칙 없음, clean/minimal 스타일, 전사 1024자 이하일 때만 Medium으로 처리한다. 1024자는 보수적인 길이 경계이지 검증된 모델 한계가 아니다.
- 사용자 사전 또는 활성 교정 규칙, 선택 편집·번역·질문·검색·초안, structured/professional 스타일, 사용자/장면 지침, 긴 전사는 High를 유지한다. 공개 질문 호출 `run_text`도 High다. 사전/교정 분류는 전용 홈의 반복 품질 검사에서 표기 누락을 발견해 추가했다. 현재 실제 Windows DB의 사전·교정 규칙은 각각 0건으로 기본 받아쓰기 경로는 Medium이다.
- 모든 요청에서 자식 프로세스에만 `service_tier="priority"`를 전달한다. 조사한 [Codex ServiceTier 구현](https://github.com/openai/codex/blob/2e5fea64eefcaa19f48458b2386011b619f69c70/codex-rs/protocol/src/config_types.rs#L528-L551)은 `fast`와 `priority`를 Fast로 해석하고 wire 값은 `priority`로 보낸다.
- `skills.include_instructions=false`로 음성 정리에 사용하지 않는 스킬 목록 주입을 제외한다. 전역 AGENTS 지침까지 없애는 설정은 아니다. 전역 Codex 파일, 인증 파일과 세션은 수정하지 않는다.
- 매 요청 ChatGPT 로그인 확인, 60초 전체 제한, 소유한 프로세스 취소, read-only sandbox, 도구 실행 금지, 완료된 최종 답변만 출력하는 정책을 유지한다. 자동 모델/제공자/품질 fallback은 추가하지 않는다.

[공식 Fast 설명](https://learn.chatgpt.com/docs/agent-configuration/speed): 구독 포함 사용량은 Standard 대비 2.5배, 구매 크레딧은 2배다. 이는 **속도 배수**가 아니며 계정/모델의 가용성에 따라 달라진다. 사용자는 이 영향을 알고 승인했다. 전역 Fast 설정은 바꾸지 않았다.

### 품질 비교와 후보 판정

- 같은 합성 10문장 배치에서 High + Fast는 12036 ms, Medium + Fast는 8627 ms였다. 두 출력 모두 이름·금액·날짜·부정·강조·불확실성·인용 명령·할 일 개수를 수동 확인했다. 이는 한 번의 배치 비교이며 단일 발화/음성 인식 품질/안정적인 개선율의 증거는 아니다. 로그: `qa/cleanup-gold-fast-high.log`, `qa/cleanup-gold-fast-medium.log`.
- 실제 운영 `CodexProvider::polish`와 원래 전체 앱 프롬프트로 일반 받아쓰기 5건(문장 전체, 금액 정정, 수신자 정정, 취소 금지, 기술 식별자) 및 High 번역/선택 편집 2건을 순차 실행해 7/7 의미 검사를 통과했다. 일반 5건 시간은 4134, 4743, 6431, 4601, 4189 ms. 합성 자료만 전송했다. 로그: `qa/windows-live-hybrid-quality.log`.
- 운영 품질 검사는 `cargo test --lib llm::codex::tests::live_hybrid_polish_preserves_synthetic_facts -- --ignored --exact --nocapture`로 재현한다. 기존 구독 로그인과 `ULTRA_STT_ROOT`가 필요하며 Fast 사용량을 소모하기 때문에 기본 테스트에서는 제외한다.
- 스킬 목록 제외는 교차 순서의 동일 5문장 비교에서 기존 평균 6499 ms / 중앙값 6713 ms, 제외 평균 6101 ms / 중앙값 5664 ms, 양쪽 10/10 사실 보존이었다. 입력 토큰 절약은 확인했지만 단독 지연 개선은 작고 표본도 작다. `qa/cleanup-context-paired.json`.
- Codex app-server 유지 실험은 초기화 376 ms, 발화 5474/6504/4529 ms로 명확한 속도 이점이 없었다. 현재 CLI와 다른 전역 설정/도구 주입 위험까지 생겨 운영 경로로 바꾸지 않았다. 짧은 voice-only 기본 지침도 빨라지지 않아 미적용이다. 오류 이벤트를 숨기는 개발 단계 플래그도 미적용이다.

최종 설치본의 실제 Mac→Windows 요청 측정과 설치 증거는 `HANDOFF.md`의 후속 항목에 기록한다. 작은 corpus 통과가 모든 실사용 품질이나 Typeless 동급 속도를 보장하지 않는다.

### 최신 설치본 실제 원격 경로

같은 2.83초 한국어 WAV를 Mac에서 기존 Windows endpoint에 순차 전송했다. 전후 각각 5건 모두 원문 전체 보존 검사를 통과했다.

| 구성 | 5회 시간(ms) | 평균 | 중앙값 |
| --- | --- | --- | --- |
| 이전 High / Standard | 6173, 5121, 6328, 11240, 6940 | 7160 ms | 6328 ms |
| 설치된 자동 Medium / Fast / 스킬 목록 제외 | 5428, 5299, 5242, 5317, 4717 | 5201 ms | 5299 ms |

관측 평균 27.4%, 중앙값 16.3% 감소다. 전후 표본이 작고 클라우드 지연이 달라 안정적인 개선율을 보장할 수 없다. 이전의 11240 ms 이상값을 숨기지 않았다. 설치된 서버 로그는 5회 모두 `effort=Medium service_tier="priority"`를 확인했고 내부 STT는 618–878 ms, AI는 3937–4627 ms였다. **아직 AI에 수 초가 걸리며 Typeless 동급 속도는 입증되지 않았다.**

보고서: `qa/latency-before-research.json`, `qa/latency-after-hybrid.json`; 설치 서버 로그: `runtime/windows-updated.log`. 요청 시간에 녹음·단축키·커서 삽입은 포함되지 않는다. Windows/Mac 모두 실제 설치 위치에서 실행 중이고 기존 기록은 각각 13건이다. Windows 612 / Mac 617 native tests와 별도 운영 프롬프트 7건 검사가 통과했다. 최신 실마이크→커서 입력의 사용자 확인은 아직 대기 중이다.

### 후속 전용 홈 실험

사용자는 앱 전용 공식 ChatGPT 로그인과 개발 지침 분리 운영을 승인했다. 전역 개발 Codex 파일은 변경하지 않고 `runtime/codex-app-home`을 자식 프로세스에만 지정한다. 모델은 `gpt-6-luna` 그대로다. 준비/전환/복구는 `scripts/configure-codex-profile.ps1`과 `docs/codex-isolation-personal.md`를 따른다.

운영과 같은 단문 프롬프트를 교차 순서로 각각 5회 실행했다. 앞선 후보도 숨기지 않고 남겼다.

| 지침 후보 | 공유 홈 평균 / 중앙값 | 전용 홈 평균 / 중앙값 | 입력 토큰 공유 / 전용 |
| --- | --- | --- | --- |
| v1 | 4908 / 4852 ms | 4942 / 4784 ms | 12503 / 6528 |
| v2 | 5303 / 4979 ms | 4591 / 4477 ms | 12503 / 6563 |
| 최종 v3 | 4454 / 4604 ms | 3899 / 3832 ms | 12503 / 6618 |

최종 입력 토큰 47.1% 감소와 해당 표본의 평균 12.5% 감소를 관측했다. v1에서는 평균이 오히려 소폭 느렸으므로 고정 개선율이나 Typeless 동급 응답을 보장하지 않는다. 로그인 확인을 포함한 AI 단독 CLI 비교이고 STT·네트워크·커서 삽입은 별도다. `qa/codex-profile-comparison.json`과 각 `codex-profile-paired[-v2|-v3].log`가 원자료다.

짧은 지침에서 Medium의 사용자 사전 표기가 반복해서 누락돼 먼저 전환을 보류했다. 명확한 사전/교정 우선순위와 다른 용어의 일반 예시를 추가하고, 사전/교정 요청은 High로 처리해 운영 경로의 합성 11건을 두 번 모두 통과했다. 단문 금액 검사에서 정답 `13만 7,500원`을 거절한 QA 형식 오류도 쉼표 정규화로 수정했다. 실패 로그는 보존했다. 최종 품질 근거: `qa/codex-isolated-quality-high-terms.log`, `qa/codex-isolated-quality-high-terms-repeat.log`.

입력 토큰 절약과 구독 사용량 절약률은 같지 않다. 캐시 입력·출력·추론·Fast 배율까지 영향을 준다. [공식 가격/사용량 설명](https://learn.chatgpt.com/docs/pricing). 최종 설치된 Mac→Windows 경로의 측정과 실제 사용자 UI 확인은 `HANDOFF.md` 후속 항목을 따른다.
