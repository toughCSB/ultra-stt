# Ultra STT — 외부 벤치마크·실사용 근거로 다시 판단한 전사 엔진 전환

조사일: 2026-10-29. 대상은 한국어 중심의 Windows·Home·Mac·Fold7 받아쓰기와 간헐적인 긴 독백이다. 원문·숫자·이름 보존을 우선하고, 종료 후 커서에 입력될 때까지의 대기를 줄이는 것이 목적이다. 아래 권고는 공개 자료와 기존 코드 검토에 따른 판단이며 우리 앱의 유료 API 실측 결과가 아니다.

## 1. 결론: 일반 전사부터 조건부 도입, 실시간 기본 전환은 보류

**권고:** `gpt-transcribe`를 일반 전사의 첫 유료 비교 후보로 선택한다. 기존 로컬 엔진을 보존하고 같은 음성으로 검증한 뒤 주력 전환 여부를 결정한다. `gpt-live-transcribe`는 한국어 바로입력의 추가 후보로 두되 현재 근거만으로 기본값으로 올리지 않는다. INT8 전환은 후순위로 유지한다.

근거는 최신 파일 모델의 외부 영어 시험과 실시간 모델의 실제 통합 경험이다. 파일 모델은 빠른 후보로 평가됐지만 경쟁 후보 모두를 이기는 결과는 아니었다. 실시간 경로에는 발화 경계와 최종 결과 처리라는 별도 구현 부담이 있다. [Superwhisper](https://superwhisper.com/benchmarks/openai-gpt-transcribe), [PERIT](https://perit.ai/research/benchmark/speech-to-text/leaderboard?open=openai/gpt-transcribe&tab=speed), [DataCamp 직접 실험](https://www.datacamp.com/tutorial/gpt-live-transcribe-api). (clm_002, clm_006)

이번 외부 조사 후 앞선 설계의 확신 수준을 낮춘 부분은 **실시간 모델의 한국어 우위**다. 신형 두 모델은 구형 GPT-4o 전사 모델과 다르며, 구형 자료가 풍부하다는 이유로 새 모델의 한국어 정확도를 입증한 것으로 읽으면 안 된다. [OpenAI 워크플로](https://developers.openai.com/api/docs/guides/transcription), [모델을 직접 통합한 설명](https://www.datacamp.com/tutorial/gpt-live-transcribe-api). (clm_001)

아래 단일 시험 수치와 개인 후기는 **미확정 부록**에 모았다. 그 구분은 원문에 수치가 없다는 뜻이 아니라, 동일 결과를 독립적으로 재현하거나 떡배님 환경에 적용해 확인하지 않았다는 뜻이다.

## 2. 떡배님의 사용 방식에서 무엇이 중요한가

이 표의 사용 현황은 이 대화와 앞선 앱 구조 검토에 근거한다. 외부 사용자의 상황을 떡배님의 실제 사용량이나 오류 빈도로 바꿔 쓰지 않았다.

| 실제 요구 | 전환으로 기대하는 이점 — 설계 판단 | 남는 비용·위험 | 결정 기준 |
|---|---|---|---|
| 한국어에 영어 제품명·개발 용어가 섞임 | 언어·용어 힌트를 모델별로 검증할 수 있음 | 영어 벤치마크 순위가 한국어 보존을 보장하지 않음 | 한영 표기·이름·숫자·부정문 별도 정답 검사 |
| 짧은 받아쓰기 후 바로 커서 입력 | 클라우드 전사와 직접 AI API의 지연 비교 가능 | 업로드와 네트워크 왕복, AI 다듬기 시간은 남음 | 종료→입력 완료의 보통/느린 경우를 모두 비교 |
| 여러 분 이야기한 뒤 끝부분까지 보존 | 원음·구간별 확정 상태를 엔진과 독립적으로 보관 | 유료 서비스도 일부 내용을 누락할 가능성을 평가해야 함 | 앞·중간·끝과 주요 의미의 누락 0건을 우선 |
| Mac·Fold7에서도 같은 품질 | SB GPU 모델 의존을 줄일 수 있음 | SB 중계를 유지하면 SB 전원·네트워크 의존은 남음 | 기기별 연결과 실제 입력까지 확인 |
| 원문/최소/깔끔하게/구조화/전문적으로 | 기존 스타일을 그대로 비교 가능 | 좋은 전사도 AI 다듬기에서 의미가 바뀔 수 있음 | 전사 원문과 최종 문장을 따로 평가 |
| 즉시 타이핑하는 바로입력 | 말하는 동안 확정 구간을 받을 수 있음 | 부분 문구 수정·중복 입력·발화 종료 판단 | final 구간 순서와 입력 대상 보호 검증 |

한국어·영어 혼용은 단일언어 낭독과 별도 과제이며, 허용하는 한글/영문 표기와 띄어쓰기 정규화도 점수에 영향을 준다. 따라서 CER, WER, 혼용 MER를 같은 퍼센트 순위로 합치지 않는다. [HiKE 원저](https://aclanthology.org/2026.findings-eacl.33.pdf), [koedesk 방법과 한계](https://koedesk.app/benchmark/). (clm_004)

이름·명령·경로 같은 중요한 값은 문장 대부분이 맞아도 한 글자 오류로 사용할 수 없어진다. 영어 업무 값 연구와 의료 전문 용어 연구도 집계 오류율과 핵심 내용 보존을 구분한다. 떡배님 평가에서는 `Ultra STT`, `Fold7`, `localhost`, 단축키 이름, 날짜·금액·단위·부정 표현을 별도 검사하는 것이 합리적이다. [VoiceCodeBench](https://www.vals.ai/benchmarks/voice-code-bench), [Appen MedTerm-90](https://www.appen.com/research/medterm-90-medical-speech-to-text-benchmark). (clm_007)

## 3. 외부 근거에서 공통으로 읽을 수 있는 것

### 속도 숫자의 시작·종료가 다르다

최초 글자가 보인 시점, 말이 끝난 뒤 최종 전사가 온 시점, 음성을 실시간으로 보내는 전체 시간이 서로 다르다. 어떤 시험에는 VAD의 침묵 대기와 네트워크가 포함되고, 어떤 표는 녹음 재생 시간까지 포함한다. 이 수치를 Ultra STT의 AI 다듬기와 커서 입력까지 완료되는 시간으로 취급하지 않는다. [VoiceThere 측정 정의](https://app.voicethere.io/docs/benchmarks), [Vals 측정 정의](https://www.vals.ai/benchmarks/voice-code-bench), [Artificial Analysis 측정 정의](https://artificialanalysis.ai/speech-to-text/streaming). (clm_003)

특히 기존 앱의 AI 구간이 그대로라면 ASR만 빨라져도 전체 대기 감소에는 한계가 있다. 이것은 앞선 로컬 조사에서 단계별 시간을 분리한 이유이며, 새 엔진 시험에서도 같은 구분을 유지한다.

### 성공 후기와 실패 후기가 모두 있다

최신 파일 모델로 기술어와 발화를 잘 보존했다는 장문 사용 기록이 있다. 반대로 같은 최신 모델의 긴 입력에서 일부 내용이 빠졌다는 직접 관찰도 있다. 표본·언어·녹음 조건이 달라 이 자료로 성공률이나 실패율을 계산할 수는 없다. [Ralf의 직접 비교](https://rdmueller.github.io/pages/blog/transcription-models-compared.html), [Ayushman의 오류 기록](https://aiwithayushman.com/blog/sarvam-vs-openai-what-broke). (clm_005)

또 다른 앱 개발자는 실제 혼용 음성에서 양쪽 모델의 승패가 섞였고, 필요한 기능의 차이까지 고려해 새 모델 채택을 보류했다. 기존 로컬 전사의 문제도 설정을 조정해 개선됐다는 경험이 있어, 모델명만으로 모든 차이를 설명하기 어렵다. [Humla 결정 기록](https://github.com/michaelwilhelmsen/humla/issues/127), [Ralf 비교](https://rdmueller.github.io/pages/blog/transcription-models-compared.html). (clm_008)

### 실시간 전사는 모델만 연결하면 끝나지 않는다

직접 만든 자막 클라이언트와 프레임워크 통합 기록 모두 세션 준비·오디오 전송·발화 commit의 순서가 실패 원인이 될 수 있음을 보여 준다. 실제 final을 확인하기 전에 결과를 입력하거나, 음성 한가운데에서 기계적으로 끊는 구현은 별도 검증 대상이다. [DataCamp 실험](https://www.datacamp.com/tutorial/gpt-live-transcribe-api), [Pipecat 이슈](https://github.com/pipecat-ai/provider-watch-reports/issues/3), [현재 공식 안내](https://developers.openai.com/api/docs/guides/realtime-transcription). (clm_006)

## 4. 선택지별 장단점과 비용 판단

| 선택지 | 떡배님에게 유리한 점 — 제안 판단 | 불리하거나 아직 확인할 점 | 권고 |
|---|---|---|---|
| 현재 로컬 FP16을 계속 주력 | 오프라인 가능, 음성을 외부로 보내지 않는 경로 보존, 음성 분당 API 비용 없음 | 현재 장애 해결과 GPU 관리가 필요 | 기존 경로를 보존하되 장애 상태를 새 엔진의 정상 비교 기준으로 삼지 않음 |
| OpenAI 파일 전사 | 가장 작은 범위로 실제 음성 품질과 비용을 비교하기 좋음 | 긴 파일 전체 처리·업로드 대기, 내용 누락 여부 실측 필요 | 첫 유료 평가 후보 |
| OpenAI 실시간 전사 | 녹음 중 처리와 바로입력에 적합한 구조 | 한국어 실제 품질·경계 처리·연결·부분 결과 정합성 검증 부담 | 기본 전환 보류, 선택적 시험 |
| 파일/실시간 선택 + 로컬 복구 경로 | 용도별로 품질·비용을 선택하고 실패 때 원본 유지 | 구성 복잡성, 자동 fallback이 중복 결제·중복 입력을 만들 수 있음 | 검증 후 목표 구조. fallback은 명시적으로 수행 |
| 클라우드 STT + 직접 AI 다듬기 API | GPU 전사와 CLI 호출 대기를 각각 비교할 수 있음 | 다듬기 모델 경로에 따른 품질·별도 과금·전송 정책 평가 필요 | STT 시험 뒤 같은 모델/프롬프트로 비교 |

사용량을 측정하기 전 월 요금을 확정하지 않는다. **분당 $0.0045와 $0.017이라는 공시 단가를 가정한 계산 예시**에서 매일 한 시간씩 30일 사용하면 각각 $8.10, $30.60이다. 실제 사용량, 세금, 재시도, AI 다듬기는 별도다. [공식 요금](https://developers.openai.com/api/docs/pricing). (clm_019)

음성이 외부 API로 전송되는 경로에는 해당 프로젝트의 데이터 통제와 보존 조건을 확인한다. 외부 연결을 쓰는 편의성과 로컬 보관의 이점을 사용자 선택으로 유지한다. [OpenAI 데이터 통제](https://developers.openai.com/api/docs/guides/your-data)

## 5. 실제 의사결정 절차

**지금 결정할 것:** OpenAI API의 제한된 비교 검증을 준비할 가치가 있다. 전면 교체나 한국어 실시간 모델의 기본 채택을 지금 확정하지 않는다. 외부 근거는 시험 후보를 좁혀 주지만 떡배님의 음성 결과를 대신하지 못한다.

제안하는 평가 순서:

1. 짧은 한국어 받아쓰기와 한영 혼용을 파일 모델로 비교한다. 전사 원문부터 검수해 AI가 오인식을 매끄럽게 숨기는 것을 구분한다.
2. 긴 독백은 음성 전체와 구간별 정답을 함께 검수한다. 네트워크 성공과 본문 반환만으로 통과시키지 않는다. 필요하면 자연스러운 발화 경계에서 나눈 처리도 별도 비교하되 지나치게 짧은 분할을 바로 기본값으로 삼지 않는다.
3. 바로입력은 별도 시험으로 분리한다. 녹음 시작 반응, 최초 표시, 확정 입력, 취소, 입력창 이동, 연결 복구를 확인한다.
4. 같은 전사문을 기존 Codex CLI와 직접 AI API에 넣어 스타일 품질과 대기를 비교한다. 개선을 위해 본문을 줄이거나 원문 스타일의 정리 기능을 빼지 않는다.
5. 통과한 경로만 기기별로 적용하고 원래 엔진으로 되돌릴 선택을 보존한다. 메모리 정밀도 변경은 이 결정 뒤에 다룬다.

통과 기준은 **중요한 값·부정·문장 구간 보존**, **체감 완료 시간 개선**, **실패 시 원음과 전사문 복구**, **유료 사용량 예측 가능성**이다. 이 네 항목 가운데 하나라도 크게 나빠지면 그 조합의 기본 전환을 보류한다. API 키가 준비되면 앞선 설계서의 소액 평가 범위부터 실행하고, 비교 결과에 따라 전환을 결정하는 것이 제 권고다.

## Confidence — 어디까지 믿을 수 있는가

- 비교적 강한 근거: 워크플로와 시간 지표의 차이, 혼용 음성의 별도 평가 필요, 핵심 값 보존과 집계 오류율의 차이. 복수의 원자료와 방법론이 있다.
- 제한된 근거: 특정 모델의 공개 속도·정확도 수치. 언어·조건·표본·집계 방식·실행 지역과 이해관계가 다르며 우리 환경에서 재실행하지 않았다.
- 약한 근거: 판매업체가 제시한 작은 표본과 개인 후기. 좋은 사례와 나쁜 사례를 모두 수집했으며 단일 경험을 전체 제품의 발생률로 바꾸지 않았다.
- 조사에 공개 웹·논문·원작성자의 기록을 사용했다. 유료 API 호출, 사용자 음성 업로드, 앱·서버 설정 변경은 수행하지 않았다.

## Unresolved — 외부 발표 수치와 실제 후기, 우리 환경 재현 미확정

원문에 아래 수치가 실렸다는 점은 확인했다. 독립 재현 또는 Ultra STT 한국어 적용 결과로 검증된 수치는 아니다. 엄격한 주장 장부에서 단일 시험·소표본·이해관계 제한이 남는 항목을 이 구역에 둔다.

### 최신 모델의 외부 측정값

| 출처·시점 | 발표한 관측 | 조건과 해석 한계 |
|---|---|---|
| Superwhisper, 2026-09-03 갱신 | `gpt-transcribe` 대표 응답 0.79초, WER 7.8% | 경쟁 받아쓰기 앱 업체의 시험. 속도는 8–15초 받아쓰기 길이, 품질은 영어 자료 여러 종류. 모든 긴 녹음이 같은 시간에 끝난다는 뜻이 아님. (clm_009) |
| PERIT, 2026-10-29 열람 | 같은 모델 영어 결과의 중앙값 1.0초, P95 1.8초 | 상업 데이터 업체 평가. 한국어·모바일 경로와 직접 대응하지 않음. Superwhisper와 지표·코퍼스가 달라 합산 순위 금지. (clm_009) |
| VoiceThere, 2026-10-19/20 | `gpt-live-transcribe` finalize→final 중앙값 602ms, p90 697ms | EU 데이터센터, 짧은 영어 합성 발화, 10회. 동일 표의 speech-end→final은 1,542/1,637ms로 시작점부터 다름. LLM·커서 입력 포함 아님. (clm_010) |
| Vals/BeSimple, 2026-09-24 | 영어 업무 값 전체 보존 성공률: 최신 실시간 모델 67.67%, Whisper Large-v3 54.33% | 사람이 녹음한 영어 300개/5.6시간. 고유한 구조 값을 모두 맞춰야 통과하는 과제다. 한국어 오류율 또는 보통 받아쓰기 성공률이 아님. (clm_011) |

출처: [Superwhisper 수치](https://superwhisper.com/benchmarks/openai-gpt-transcribe), [Superwhisper 방법](https://superwhisper.com/benchmarks), [PERIT 수치](https://perit.ai/research/benchmark/speech-to-text/leaderboard?open=openai/gpt-transcribe&tab=speed), [PERIT 방법](https://perit.ai/research/method), [VoiceThere](https://app.voicethere.io/docs/benchmarks), [Vals](https://www.vals.ai/benchmarks/voice-code-bench).

### 한국어에서 확인한 반대 근거

| 출처·모델 | 발표치 | 왜 바로 결론 내리지 않는가 |
|---|---|---|
| Pikka, `gpt-transcribe` | 한국어 CER 2.9% | FLEURS 낭독 10문장뿐인 판매사 결과다. 신뢰구간·우리 고유명사·긴 음성 검증이 없다. (clm_012) |
| Microsoft 등 VibeVoice 보고서, `gpt-live-transcribe` | 한국어 대화 CER 32.18% | 경쟁 모델 연구진의 사전공개 논문, 2.9초 청크 전송, 최대 480초 평가. 한국어별 표본 수·불확실성 구간과 독립 재현이 부족하다. (clm_013) |

[Pikka 원표](https://pikkaai.com/speech/accuracy), [VibeVoice 원문 Table 1, PDF p.6](https://arxiv.org/pdf/2609.02812). **두 행은 다른 모델·음성·조건이므로 서로 빼거나 배수를 계산하면 안 된다.** 좋은 소표본만으로 채택하거나 나쁜 경쟁사 시험만으로 폐기하지 않고, 한국어 바로입력의 실제 검증을 앞당기는 근거로 사용한다.

### 실사용 후기에 나타난 장단점

- **장문 성공 사례:** Ralf D. Müller는 독일어 소프트웨어 토론의 두 구간에서 API 전사가 기술어와 자기 정정 발화를 보존했다고 설명했다. 각 5분 구간에 업로드 포함 13.9초/12.4초를 보고했다. GPU 없는 WSL 로컬 대조이므로 떡배님의 RTX3070과 속도 배수를 비교할 수 없다. 기존 불완전 전사문을 기준으로 센 차이를 정확도 점수로 쓰지도 않는다. [2026-09-18 직접 기록](https://rdmueller.github.io/pages/blog/transcription-models-compared.html). (clm_014)
- **장문 누락 사례:** Ayushman Dash는 최신 파일 모델에 같은 힌디어·영어 혼합 구간을 반복해도 출력 길이가 달라졌고 일부 빠른 발화가 빠졌다고 보고했다. 한 10분 구간 네 결과는 535/547/747/747단어였다. 단어 수 차이 자체만으로 누락을 입증하는 것이 아니라 저자의 구간 대조 관찰로 취급한다. 우리가 같은 음성을 재생해 재현한 것은 아니다. [2026-10-22 오류 기록](https://aiwithayushman.com/blog/sarvam-vs-openai-what-broke). (clm_015)
- **혼용 언어 호평과 이해관계:** 러시아어·영어 클립을 비교한 Reddit 작성자는 영어 원형과 전체 내용 보존을 이유로 최신 파일 모델을 선택했다. 고유명사 오류는 남았고 작성자는 경쟁 받아쓰기 앱 제작자임을 밝혔다. 한국어·영어 증거 또는 중립적인 시장 순위로 쓰지 않는다. [원게시물, 조사일 상대 표시 2개월 전](https://www.reddit.com/r/ProductivityApps/comments/1w0hed8/dont_use_wispr_flow_or_superwhisper_for_bilingual/). (clm_016)

**여전히 미확정인 핵심:** 최신 두 모델이 떡배님의 실제 한국어·혼용·숫자·부정 표현·긴 녹음에서 정상 로컬 대비 얼마나 좋아지는지, Mac/Fold7 네트워크에서 어느 정도 대기가 남는지, 장기 실패 빈도와 월 실제 사용량은 이번 외부 조사로 확정하지 못했다.

## Refuted — 채택하지 않은 해석

- Vals 표의 70.34초를 “말을 멈춘 뒤 기다리는 시간”으로 읽는 해석은 해당 방법론과 맞지 않는다. 그 Latency는 실시간 음성 송신 시간을 포함한다. [Vals 정의](https://www.vals.ai/benchmarks/voice-code-bench). (clm_017)
- 낮은 전체 오류율을 모든 이름·숫자의 정확성 보증으로 읽는 해석은 채택하지 않는다. 중요한 값을 별도로 평가하는 연구에서 이 차이가 드러난다. [Vals](https://www.vals.ai/benchmarks/voice-code-bench), [Appen](https://www.appen.com/research/medterm-90-medical-speech-to-text-benchmark). (clm_018)

## 미확정 자료를 포함한 출처 장부와 이해관계

등급: B는 공식 문서·학술 원저·방법을 공개한 기관 연구, C는 문서화된 상업 비교·개발 사례, D는 프리프린트·개인 실험 글, E는 소셜 후기·근거가 약한 판촉 자료다. 등급은 개별 주장에 대한 무조건적 보증이 아니다. 보조 조사 노트의 A/B 표시는 그 노트 내부 재현성 척도이며 이 표와 동일한 등급 체계가 아니다.

| ID | 출처 | 기준 시점 | 등급·이해관계 |
|---|---|---|---|
| src_001 | [Superwhisper 모델 표](https://superwhisper.com/benchmarks/openai-gpt-transcribe) | 2026-09-03 | C, 경쟁 앱 |
| src_002 | [PERIT English](https://perit.ai/research/benchmark/speech-to-text/leaderboard?open=openai/gpt-transcribe&tab=speed) | 게시일 미상, 10-29 열람 | C, 데이터 판매사 |
| src_003 | [VoiceThere](https://app.voicethere.io/docs/benchmarks) | 2026-10-19/20 | C, 음성 플랫폼 |
| src_004 | [Vals/BeSimple](https://www.vals.ai/benchmarks/voice-code-bench) | 2026-09-24 | B, 평가 업체/산업 파트너 |
| src_005 | [Appen](https://www.appen.com/research/medterm-90-medical-speech-to-text-benchmark) | 2026-09 | B, 데이터 판매사 연구 |
| src_006 | [VibeVoice](https://arxiv.org/pdf/2609.02812) | 2026-09-10 v2 | D, 경쟁 모델 개발진 프리프린트 |
| src_007 | [HiKE](https://aclanthology.org/2026.findings-eacl.33.pdf) | 2026-03 | B, 학술 원저, 구형 모델 |
| src_008 | [koedesk](https://koedesk.app/benchmark/) | 2026-07-05 | C, Scribe 사용 업체가 비용 부담 |
| src_009 | [Pikka](https://pikkaai.com/speech/accuracy) | 2026-09-17 | E, 판매사 소표본 |
| src_010 | [Ralf](https://rdmueller.github.io/pages/blog/transcription-models-compared.html) | 2026-09-18 | D, 개인 개발자 비교 |
| src_011 | [Ayushman](https://aiwithayushman.com/blog/sarvam-vs-openai-what-broke) | 2026-10-22 | D, 개인 오류 기록 |
| src_012 | [DataCamp/Khalid](https://www.datacamp.com/tutorial/gpt-live-transcribe-api) | 2026-08-06 | C, 직접 제작·실험 |
| src_013 | [Pipecat](https://github.com/pipecat-ai/provider-watch-reports/issues/3) | 2026-09-01 | C, 통합 이슈 |
| src_014 | [Reddit/AI Dictation 제작자](https://www.reddit.com/r/ProductivityApps/comments/1w0hed8/dont_use_wispr_flow_or_superwhisper_for_bilingual/) | 정확일 미확인, 상대 표시 2개월 | E, 판촉 이해관계 |
| src_015 | [Humla](https://github.com/michaelwilhelmsen/humla/issues/127) | 2026-07/08 | C, 채택 보류 결정과 실험 |
| src_016 | [OpenAI 전사](https://developers.openai.com/api/docs/guides/transcription) | 2026-10-29 열람 | B, 공급자 기능 안내 |
| src_017 | [OpenAI 데이터 통제](https://developers.openai.com/api/docs/guides/your-data) | 2026-10-29 열람 | B, 공급자 정책 |
| src_018 | [OpenAI 가격](https://developers.openai.com/api/docs/pricing) | 2026-10-29 열람 | B, 계산의 단가 전제 |
| src_019 | [Artificial Analysis](https://artificialanalysis.ai/speech-to-text/streaming) | 2026-10-29 열람 | B, 방법론만 채택; 재인용 순위는 제외 |
| src_020 | [Superwhisper 방법](https://superwhisper.com/benchmarks) | 2026-09-03 | C, 경쟁 앱 |
| src_021 | [PERIT 방법](https://perit.ai/research/method) | 게시일 미상, 10-29 열람 | C, 자체 평가 |
| src_022 | [OpenAI 실시간 안내](https://developers.openai.com/api/docs/guides/realtime-transcription) | 2026-10-29 열람 | B, 공급자 기능 안내 |
