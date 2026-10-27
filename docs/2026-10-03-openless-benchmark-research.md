# OpenLess 벤치마킹 조사 — Ultra STT의 실제 빈틈과 적용 우선순위

조사일: 2026-10-03. 범위: 공개 소스·릴리스·관련 PR/이슈와 현재 Ultra STT 소스의 정적 비교. 앱 코드·설정·인증·서버는 변경하지 않았다. 음성 전송, 모델 설치, 유료 추론, 두 앱의 실기기 성능 비교는 수행하지 않았다.

## 판단

**벤치마킹할 가치가 있다. 가장 큰 효과가 예상되는 부분은 결과 복구, 입력 대상 확인, 원격 경로의 개인화 일관성이다.** 우리 앱에는 스트리밍 삽입 기반, 클립보드 대체·복원, 보안 저장소, 사용자 장면, 교정 규칙이 이미 있다. 이 기능들을 처음부터 다시 만드는 것은 우선순위가 낮다.

속도 측면에서는 **현재 주력 Codex 경로가 최종 응답만 받는 구조**라는 제약이 크다. OpenLess의 스트리밍 화면을 따라 만들거나 설정 하나를 켜는 것으로 현재 대기 시간이 해결되지는 않는다. 첫 결과가 보이는 시간과 최종 문장이 안전하게 입력되는 시간을 분리해 평가해야 한다.

아래 우선순위와 기대 효과는 **조사자의 제안**이다. 코드 존재는 확인했지만 실제 기기에서의 실패율·정확도·지연 개선은 아직 측정하지 않았다. P0는 먼저 보완할 사용자 신뢰 문제, P1은 그다음 품질·운영 개선, P2는 별도 실험이 필요한 기능을 뜻한다. 난이도는 상대 평가이며 공수 견적이 아니다.

| 순서 | 벤치마킹 대상 | 우리 앱의 구체적인 차이 | 기대 효과 | 난이도 |
| --- | --- | --- | --- | --- |
| P0-1 | 입력 실패 시 완성문 복구 카드 | 데스크톱 결과 상태와 복구 UI의 연결이 약함. Android는 삽입 실패 시 완성문을 되찾는 경로가 확인되지 않음 | 다시 말할 필요를 줄임 | 중 |
| P0-2 | 앱보다 더 세밀한 입력 대상 확인 | 데스크톱 guard는 PID·앱 식별자 위주 | 같은 앱의 다른 창·입력칸에 잘못 들어가는 위험 감소 | 중~큼 |
| P1-1 | 로컬·원격이 공유하는 교정 정책 | 원격 `PolishRequest`의 사전·교정 규칙·사용자 장면이 비어 있음 | PC·Mac·휴대폰의 용어와 말투 일관성 | 중 |
| P1-2 | 단계별 timeout·재시도·기록 | 일반 HTTP LLM은 30/60초, 주력 Codex는 60초. 원격에는 별도 중첩 제한 | 긴 문장 처리 실패와 원인 불명 대기 감소 | 중 |
| P2-1 | 수정한 단어를 확인받고 사전에 추가 | 수동 교정 등록은 있으나 자동 수정 관찰은 확인되지 않음 | 반복 오인식 수정 부담 감소 | 큼 |
| P2-2 | 실제 delta부터 화면·입력까지 이어지는 스트리밍 | 현재 Codex adapter와 원격 응답은 final-only | 조건이 맞으면 첫 결과 표시 지연 감소 | 큼 |

## 비교 기준과 버전

| 대상 | 확인한 기준 | 의미 |
| --- | --- | --- |
| Ultra STT | `opentypeless`, package `1.1.62`, HEAD `d6c2434ffed64bf111e038d6d8aea80ccf2a3c8e` + 현재 작업 파일 | 기존 README·캡슐 관련 미커밋 변경을 보존한 채 현재 코드를 읽음 |
| OpenLess Stable | `v1.3.18-tauri`, 2026-08-21, commit `ed4a1408e25e66fddb203cc758d653c777af6488` | 안정 릴리스의 실제 코드. annotated tag object와 commit SHA를 구분 |
| OpenLess Beta | `v2.0.0-Beta.4-tauri`, 2026-09-30, commit `6f9af89583d542ca55e4dbe8b326a47629fc3957` | 조회 시 기본 브랜치 `beta`의 HEAD. 새 공통 Core·다중 플랫폼 학습 기능 포함 |

공개 GitHub API, [Stable 릴리스][release-stable], [Beta.4 릴리스][release-beta]와 해당 SHA 소스를 대조했다. `main` HEAD `08d70601f993244d9be7e61e214b8973136df931`은 Stable tag commit과 같지 않으므로 릴리스 근거에는 tag가 가리키는 commit을 사용했다. 코드 링크는 모두 SHA로 고정했다.

조사 종료 시 Ultra STT HEAD는 별도 진행 중이던 작업의 커밋으로 `8609461b67d4257198908df40f7e47bcb7714ad6`이 되었다. 시작 commit과의 diff는 캡슐 최상단 표시·README·검증 문서·이미지 관련이며, 아래 비교한 pipeline·LLM·remote·입력 guard·Android 삽입 코드는 그 사이 변경되지 않았다. 이 조사에서 커밋은 만들지 않았다.

우리 저장소의 [7월 gap 분석](2026-07-06-openless-gap-analysis-spec.md), [후속 보완 사양](2026-07-06-openless-followup-completion-spec.md)은 조사 이력을 확인하는 데만 사용했다. 당시의 “Android 보류”, “로컬 엔진 없음”을 현재 상태로 옮겨 적지 않았다. 구현 여부는 아래 실제 호출부를 기준으로 판단했다.

## P0-1. 입력 실패 시 완성문을 돌려주는 화면

**OpenLess에서 확인한 동작:** Stable은 삽입 실패와 스트리밍 도중 중단을 분류하고, 일부만 입력됐더라도 별도로 보존한 **전체 결과**를 복구 카드에 넘긴다. 카드에서 복사할 수 있고, 마우스를 올리면 자동 닫힘을 늦춘다. 복사는 포커스 없는 WebView의 `navigator.clipboard` 대신 backend를 사용한다. [실패 판정·전체 결과 선택][stable-recovery], [복구 카드][stable-card], [배경 PR #943][pr943]

**우리 상태:** `InsertResult`와 부분 삽입·클립보드 fallback은 이미 있다. 하지만 `pipeline:insert_result`는 store에 보관되고, 이를 직접 보여주는 복구 패널은 확인되지 않았다. 오류 캡슐은 2.5초 후 사라진다. 기록에서 복사는 가능하지만 사용자가 기록을 찾아가야 한다. `History`의 상태 라벨 분기에는 `failed`가 빠져 있다. [결과 이벤트](../src/hooks/useTauriEvents.ts#L135), [오류 캡슐](../src/components/Capsule/CapsuleError.tsx#L12), [기록 표시](../src/components/History/index.tsx#L93)

Android는 최종 문장을 받은 뒤 입력창·커서가 맞는지 다시 확인한다. 이 보호 자체는 유지해야 한다. 다만 삽입 불가 시 `cancelWork`/`finishWork`로 끝나고, **이미 생성된 문장을 복사·보관하는 별도 경로는 확인되지 않았다.** 서버 측 원격 처리도 raw/final을 응답할 뿐 기록 저장은 하지 않는다. Mac 데스크톱 클라이언트에는 별도 history 저장 경로가 있으므로 “원격 기록이 전부 없다”는 뜻은 아니다. [Android 삽입](../android/app/src/main/java/com/ultrastt/androidvoice/VoiceAccessibilityService.java#L614), [서버 결과 반환](../src-tauri/src/remote.rs#L150)

**제안:** 기존 상태 모델에 `완성 결과`, `확인된 입력 부분`, `복사 가능 여부`를 연결하고 작은 결과 패널을 제공한다. Android부터 적용하면 사용자가 다시 말해야 하는 경우를 줄일 수 있다. 입력 대상이 바뀌었을 때 자동으로 새 대상에 재삽입하지 않고, 사용자가 복사할 수 있게 한다. 오래 남는 기록을 새로 도입하기 전에는 메모리 내 임시 보존으로도 개선 가능하다.

명시적 취소와 전달 실패를 구분해야 한다. 사용자가 녹음을 취소한 것은 조용히 끝내고, 완료된 결과를 전달하지 못한 경우만 복구 대상으로 삼는다. 복구 카드의 20초 자동 닫힘은 OpenLess의 선택이며 우리 필수 조건이 아니다. 휴대폰에서는 읽다가 사라지지 않도록 별도 UX를 정하는 편이 적합하다.

**완료 기준 제안:** 합성 문장으로 입력 거부·처리 중 포커스 이동·부분 입력·클립보드 실패를 각각 재현하고, 생성이 끝난 문장은 정확히 한 번 복구할 수 있어야 한다. 새 녹음의 결과와 섞이지 않고, 이미 들어간 문장을 자동 중복 입력하지 않아야 한다.

## P0-2. 같은 앱 안의 다른 입력칸도 구분하기

**OpenLess에서 확인한 구현:** Beta의 Windows 대상 캡처는 최상위 창뿐 아니라 포커스된 자식 창과 process/thread identity를 함께 기록한다. macOS 문서 접근·전달 감시에는 AX element 동일성 확인 경로가 있다. 플랫폼마다 보장 범위가 다르므로 모든 브라우저 입력칸을 완벽히 구분한다고 일반화할 수는 없다. [Windows 대상 구조와 검증][beta-target], [macOS element 확인][beta-mac-focus]

**우리 상태:** 데스크톱 `TargetAppGuard`의 비교 대상은 `process_id`와 `native_identity`이다. 같은 앱의 다른 창이나 같은 프로세스 안의 다른 입력칸은 이 정보만으로 구분되지 않을 수 있다. 이는 **정적 코드에서 확인한 보장 범위의 한계**이며 이번 조사에서 잘못 입력되는 현상을 실제 재현한 것은 아니다. Android는 입력창·내용·선택 범위를 더 세밀하게 비교하는 기존 보호가 있다. [데스크톱 guard](../src-tauri/src/app_detector/types.rs#L108), [Android 검사](../android/app/src/main/java/com/ultrastt/androidvoice/VoiceAccessibilityService.java#L614)

**제안:** Windows는 창·컨트롤, macOS는 접근 가능한 focused element를 세션 시작 시 캡처하고, 최종 입력 및 각 streaming chunk 직전에 다시 확인한다. 확인 불가 시 P0-1의 결과 복구로 넘긴다. OpenLess의 원래 창 재활성화 동작까지 그대로 따를 필요는 없다. 사용자가 옮긴 포커스를 자동으로 되돌릴지는 별도 제품 선택이다.

**완료 기준 제안:** 다른 앱 이동뿐 아니라 동일 브라우저의 두 창, 동일 앱의 두 편집 영역, 화면 잠금, 창 종료, 빠른 다음 녹음을 시험한다. native 식별자가 같다고 HTML 입력칸도 같다고 단정하지 않는다. 기존 Android 보호는 회귀 검증 대상으로 유지한다.

## P1-1. 기기별로 달라지는 사전·교정 규칙 줄이기

**OpenLess에서 확인한 설계:** Beta 원격 adapter가 공통 dictation backend를 시작하고, 그 세션은 같은 `DictationContext`에 교정 규칙을 snapshot으로 담아 처리한다. 오디오를 받는 위치와 글을 다듬는 정책을 분리한 점이 참고할 만하다. OpenLess의 “휴대폰 마이크 → PC 입력”과 우리의 “PC 처리 → 원래 클라이언트 입력”은 출력 방향이 다르므로 원격 프로토콜 자체를 이식하는 제안은 아니다. [원격 adapter][beta-remote], [공통 context와 교정 규칙][beta-context], [ASR 뒤 교정 적용][beta-correction]

**우리 상태:** 데스크톱은 저장된 사전·교정 규칙을 `PolishRequest`에 넣지만, `remote.rs:166–179`는 다음 값을 비워 만든다.

- `dictionary`, `correction_rules`: 빈 목록.
- `mapped_scene_prompt`, `active_scene_prompt`, `polish_custom_prompt`: 빈 문자열.
- 요청한 다섯 기본 스타일과 다듬기 on/off는 별도로 전달한다.

따라서 기본 모드가 같아도 **PC에서 등록한 고유명사·교정 규칙을 원격 처리에서도 적용한다고 보장할 수 없다.** [원격 요청 구성](../src-tauri/src/remote.rs#L166), [데스크톱 요청 구성](../src-tauri/src/pipeline.rs#L2433)

**제안:** 전체 `openless-core`를 들여오는 대신, 우리 코드에 공통 요청 구성·정책 snapshot을 재사용하는 작은 경계를 만든다. 현재 개인 서버의 소유자 범위에서 서버 내부 사전과 교정 규칙부터 일치시킨다. 클라이언트에 사전을 전송하거나 주변 입력창 내용을 서버로 보내야만 가능한 개선은 아니다. 사용자 장면·앱별 지침은 원격에서 의도한 작업과 일치하는지 정한 뒤 확장한다.

여러 사용자가 서버를 공유하게 된다면 사용자 간 사전 혼합을 막는 범위 정의가 먼저다. 이번 제안은 인증·권한·API 계약 변경을 승인하거나 구현한 것이 아니다. 단순 사전 적용만으로도 현재 Codex effort 선택이 달라질 수 있으므로 품질과 지연을 함께 비교한다. [용어가 있을 때의 Codex 처리](../src-tauri/src/llm/codex.rs#L201)

**완료 기준 제안:** 같은 합성 전사와 동일 기본 모드에 대해 로컬·원격의 요청 사전/교정 snapshot을 비교한다. `울트라 에스티티 → Ultra STT`, 한영 혼용 이름, 숫자·부정문을 포함한다. 확률적인 LLM 문장 전체의 완전 일치보다 필수 용어·의미·스타일 보존을 기준으로 삼는다. AI 다듬기를 끈 경로에서 교정도 적용할지는 명시적으로 정한다.

## P1-2. 정상적인 긴 처리와 멈춘 요청을 구분하기

**OpenLess에서 확인한 구현:** Stable에 이미 첫 본문 대기와 이후 chunk 대기를 나누는 정책이 있다. 첫 글자 예산은 `max(30, ceil(문자 수 × 0.05) + 30)`초, 이후 chunk idle은 20초이며 비스트리밍에는 별도 총 예산을 쓴다. timeout을 무조건 재시도하지 않는 분기도 있다. [Stable 시간 정책][stable-timeout], [Stable 재시도 조건][stable-retry], [PR #966][pr966]

**우리 상태:** 시간 로그와 홈의 STT·AI·전체 시간 표시는 이미 있다. 다만 경로별 timeout은 서로 다르다.

| 경로/단계 | 현재 코드의 제한 | 주의점 |
| --- | --- | --- |
| 일반 OpenAI 호환 LLM | 요청 30초 또는 60초 | 첫 본문/본문 중단을 별도로 구분하는 정책은 확인되지 않음 |
| Codex CLI | 인증 확인을 포함한 작업 예산 60초 | final-only. 일반 HTTP LLM의 timeout 정책과 별개 |
| 원격 local STT HTTP | 한 요청 60초 | 서비스 초기 시작 대기는 별도 경로 |
| 원격 서버 | 업로드 후 STT+다듬기 120초 | 내부 단계 제한도 그대로 작동 |
| Android HTTP | connect 10초, write 20초, read 135초, call 150초 | call 시간은 연결·업로드를 포함해 서버 예산과 시작점이 다름 |

근거: [HTTP LLM](../src-tauri/src/llm/protocol.rs#L128), [Codex](../src-tauri/src/llm/codex.rs#L172), [local STT](../src-tauri/src/stt/ultra_local.rs#L18), [원격 HTTP](../src-tauri/src/remote/http_upload.rs#L197), [Android HTTP](../android/app/src/main/java/com/ultrastt/androidvoice/HttpTransport.java#L16), [기존 홈 계측](../src/components/HomePage/index.tsx#L115).

**제안:** 고정 시간을 일괄 늘리기보다 작업 전체 deadline 안에서 STT·다듬기·전달 예산을 정렬한다. 실제 streaming provider에는 첫 **본문**과 마지막 **본문 진전** 시각을 구분해 적용한다. Codex final-only 경로에는 길이에 따른 총 예산과 취소·프로세스 정리부터 검토한다. 긴 요청 허용은 성공률 개선이며 단문 속도를 높이는 최적화는 아니다.

OpenLess 구현도 그대로 답은 아니다. Beta idle은 `response.chunk()`를 받을 때마다 새로 시작하므로 첫 글자 이후 heartbeat만 계속 오면 본문이 멈춘 상태를 늦게 감지할 수 있다. 우리 정책은 본문 진전 기준과 전체 상한을 함께 검토해야 한다. [실제 stream loop][beta-idle]

우리 OpenAI adapter에는 초기 `send()` timeout 재시도가 최대 3회 요청으로 이어지는 분기가 있다. 응답이 늦다는 이유만으로 재시도하면 공급자 측 중복 작업·과금이 생길 가능성이 있어, 연결 전 실패와 처리 여부가 불명확한 timeout을 구분하는 편이 낫다. **중복 과금이 실제 발생했다고 확인한 것은 아니다.** 이미 본문이 나온 뒤의 실패를 처음부터 다시 생성·입력하는 동작은 피해야 한다. [우리 재시도](../src-tauri/src/llm/openai.rs#L166)

**완료 기준 제안:** mock server로 느리지만 계속 나오는 본문, reasoning만 오는 상태, heartbeat만 오는 상태, 중간 끊김, 늦게 도착한 이전 세션, 취소를 구분한다. 운영 비교는 원문 대신 session ID·글자 수·각 단계 시간·상태만으로도 가능하다. 기존 계측을 확장해 remote 전송부터 실제 입력까지 연결하고, 기록의 요청별 모델/단계 시간 표시는 필요할 때 추가한다. OpenLess는 history 상세에 ASR/LLM 모델과 시간을 표시한다. [OpenLess 기록 표시][beta-history]

## P2-1. 사용자가 고친 단어를 확인받고 기억하기

**OpenLess에서 확인한 구현:** Stable의 수정 관찰은 macOS 중심이며, Beta.4는 Windows UIA·Android 접근성을 포함하는 관찰 adapter와 별도 동의 설정을 갖춘다. 성공한 입력 뒤 한정된 기간을 관찰하고, 후보를 보여 준 다음 수락한 단어만 사전에 넣는다. 기본 비활성·대상 편집기 의존성은 릴리스에도 명시되어 있다. [Stable 관찰 범위][stable-learning], [Beta.4 설명][release-beta], [Windows 관찰][beta-learning-win], [관찰 시작·수락][beta-learning-core]

**우리 상태:** 기록에서 사용자가 직접 교정 규칙을 만드는 기능은 있다. 출력 이후 편집을 자동 관찰해 후보를 제시하는 구현은 이번 소스 검색에서 확인되지 않았다. [기존 수동 교정](../src/components/History/index.tsx#L81), [교정 관리](../src-tauri/src/commands/dictionary.rs#L97)

**제안:** 가장 작은 단계는 기록의 결과를 사용자가 고칠 때 `이 단어 기억하기`를 제시하는 것이다. 그다음 지원되는 입력칸만 대상으로 자동 관찰을 실험한다. `이 표기를 사전에 추가`와 `앞으로 A를 B로 항상 치환`은 다른 기능이므로 같은 버튼으로 묶지 않는다. OpenLess Beta의 후보 수락은 replacement를 vocabulary에 넣으며 모든 수정 쌍을 전역 치환 규칙으로 저장하는 것은 아니다.

자동 관찰은 기존 앱의 문서 읽기 범위를 넓힌다. 로컬 관찰·짧은 유효기간·비밀번호 칸 제외·관찰 중 동의 철회·다른 칸 이동 시 중단을 포함해 설계해야 한다. 한국어에서는 조사 변화, 띄어쓰기, 문장 재작성, IME 조합 중 텍스트를 단어 교정으로 오인하지 않는 검증이 필요하다. 주변 문맥을 LLM으로 보내는 기능은 별개의 선택이며 이번 개인화 일치에 필수적이지 않다.

**완료 기준 제안:** 이름 수정은 후보가 되고, 단순 조사/문장 재작성/다른 입력칸의 글은 후보가 되지 않아야 한다. 수락 전에는 사전이 바뀌지 않고, 취소·만료·중복 수락이 안전해야 한다. Windows/macOS/Android 각각 지원되는 편집기 목록을 실기기로 확인한다.

## P2-2. 스트리밍은 실제 처리 경로부터 연결해야 한다

**우리 코드의 핵심 사실:** `CodexProvider::polish`는 `_on_chunk`를 사용하지 않고, `parse_final`은 turn 완료와 최종 agent message를 검증한다. pipeline도 `codex-cli`에 streaming insert를 활성화하지 않는다. 원격은 Codex 완료 후 한 번 `{raw,text}`를 반환하고 Android도 최종 응답을 한 번 삽입한다. [Codex 최종 검증](../src-tauri/src/llm/codex.rs#L94), [callback 미사용](../src-tauri/src/llm/codex.rs#L209), [pipeline 분기](../src-tauri/src/pipeline.rs#L2386), [원격 응답](../src-tauri/src/remote.rs#L203)

OpenLess가 배울 만한 것은 실제 provider delta, 삽입 worker, 부분 입력 상태, 완료 결과 조정을 연결한 구조다. 우리 앱에도 non-Codex 경로의 worker와 부분 입력 복구가 이미 있다. 전체 Core를 다시 만들기보다 적용 가능한 경로를 구분하는 것이 먼저다. [OpenLess stream 제어][beta-stream], [우리 기존 worker](../src-tauri/src/pipeline.rs#L455)

**제안:** 먼저 `stt_partial`, `polish_delta`, `final`, `error`, `cancelled`처럼 의미가 다른 이벤트를 분리할 수 있는지 설계한다. 현재 모델/인증을 유지한 채 공식 도구에서 안정적인 delta가 가능한지 검증하고, 불가능하면 final-only를 유지한다. 출력 검증 전의 문장을 외부 앱에 넣는 위험이 큰 경로에서는 **캡슐 미리보기만 먼저 표시하고 최종 검증 후 입력**하는 선택도 있다. 이 선택은 기다리는 체감만 개선하며 최종 커서 입력 시간은 줄이지 않을 수 있다.

Android에는 곧바로 글자마다 `ACTION_SET_TEXT`를 반복하기보다 미리보기부터 평가하는 편이 적합하다. 기존 입력창·커서 보호, 사용자 타이핑과의 충돌, 느린 서버에서의 취소를 유지해야 한다. 프로토콜·클라이언트 호환성이 바뀌므로 별도 구현 범위다.

OpenLess의 Codex OAuth 구현은 Stable에서 `https://chatgpt.com/backend-api`를 사용한다. 우리 앱 전용 공식 CLI와 다른 인증·통신 경로이므로 이를 속도 개선을 위한 대체재로 그대로 가져오는 것은 권고하지 않는다. [OpenLess의 해당 상수][stable-timeout]

이미 [10월 2일 지연 조사](2026-10-02-ai-polish-latency-review.md)에 로컬 소형 LLM·Codex effort 비교와 상주 프로세스 시험 한계가 정리돼 있다. 이번 코드 비교로 특정 모델이 더 빠르다는 새 근거를 얻지는 못했다. 해당 실험을 다시 한다면 동일한 품질 기준과 현재 설정을 고정해야 한다.

## 이미 있거나 지금 따라갈 필요가 적은 부분

| 대상 | 현재 판단과 근거 |
| --- | --- |
| 보안 저장소 | 우리 `credentials.rs`에 OS vault와 기존 키 마이그레이션이 있음. “plaintext이므로 처음부터 도입”이라는 7월 과제를 반복하지 않음. [우리 구현](../src-tauri/src/credentials.rs#L247) |
| 스트리밍 기반·클립보드 fallback | 이미 존재. 적용 provider와 복구 UX가 다음 과제. [삽입 계층](../src-tauri/src/output/mod.rs#L63) |
| 채팅창 안전한 줄바꿈 | 우리 일반 keyboard 경로는 Shift+Return을 사용함. OpenLess의 같은 기능을 신규 과제로 제안하지 않음. 명시적 Windows SendInput의 newline 모드는 별도 설정이므로 대상 앱별 회귀 시험은 유효. [우리 keyboard](../src-tauri/src/output/keyboard.rs#L225) |
| 사용자 스타일·장면 | 기존 scenes와 프롬프트 불변 규칙을 유지. 마켓플레이스·GitHub 로그인·게시/검수 backend를 추가할 실익은 개인 앱에서 작음. [우리 prompt](../src-tauri/src/llm/prompt.rs#L169) |
| 다양한 ASR 엔진 일괄 도입 | 모델 카탈로그가 많다고 한국어 정확도나 SB GPU 대비 속도가 우월한 것은 아님. 현재 중앙 Whisper 구조와 사용 목적을 먼저 유지. [OpenLess 카탈로그][beta-models] |
| 모델 관리 UX | “다운로드됨”, “서버 접근 가능”, “모델 로드됨”, “실제 전사 통과”를 구분하는 점은 참고 가능. 우리 custom endpoint의 `ready`는 설정 유효성이고 실제 연결 검사는 별도. [우리 진단](../src-tauri/src/commands/stt.rs#L310) |
| 전체 공통 Core 재작성 | 단일 처리 정책·세션 snapshot·취소 경계는 참고. Linux 전용 egui와 대형 Core 전체는 현재 Windows/Mac/Android 개인 서버 앱에 비해 도입 비용이 큼. [OpenLess 구조][beta-architecture] |
| 자체 IME·코딩 에이전트·클라우드 동기화 | 현재 음성 입력 목적과 별도 기능. 기존 삼성 키보드/Gboard 유지 방식과 앱 전용 Codex 격리를 보존 |

## OpenLess 자체의 한계도 벤치마킹 자료다

조회 시 다음 이슈는 열린 상태였다. **사용자 보고이며 이번 조사에서 재현하거나 원인을 확정한 것은 아니다.**

- Stable Windows에서 TSF 삽입 뒤 원래 입력기의 언어 모드가 복원되지 않는다는 보고가 있다. 자체 IME를 도입하면 OS 상태 복구까지 관리해야 한다는 경고 사례다. [#1033](https://github.com/Open-Less/openless/issues/1033)
- Stable Windows Foundry 모델의 idle unload 후 느린 재시작과 긴 발화 잘림 보고가 있다. 로컬 엔진이라는 이유만으로 낮은 지연을 가정하면 안 된다. [#1031](https://github.com/Open-Less/openless/issues/1031)
- Stable macOS 배포물에 MLX 리소스가 없어 모델 로드 시 앱이 종료된다는 보고가 있다. 소스 테스트와 최종 설치 파일의 모델 실행 검증은 별도다. [#1098](https://github.com/Open-Less/openless/issues/1098)

우리 배포에도 이 사례를 반영해 **설치된 패키지에서 실제 모델 로드·첫 녹음·취소·입력기 원상복귀**를 확인하는 것이 유용하다. 새 모델을 많이 번들하는 것보다 현재 지원 경로의 검증 범위를 명확히 하는 편이 먼저다.

## 라이선스와 재사용 경계

**버전별 구분이 중요하다.** Stable `v1.3.18-tauri`의 실제 LICENSE는 MIT이고, 조사한 2.0 Beta의 LICENSE는 AGPL-3.0이다. Beta README도 1.x 공개 릴리스에는 기존 MIT가 유지됨을 명시한다. [Stable LICENSE][license-stable], [Beta LICENSE][license-beta], [전환 설명][license-transition]

우리 저장소는 MIT 표기이므로 현재 Beta 소스를 그대로 가져와 기존 MIT 코드처럼 취급해서는 안 된다. 이번 권고는 동작·실패 조건·설계 원칙을 참고하는 방향이다. 실제 코드 재사용이 필요하다면 먼저 **MIT인 1.3.18에서 해당 구현이 존재하는지**, 파일별·하위 의존성의 별도 조건, 저작권 고지 포함 범위를 확인한다. 2.0 변경을 함께 섞어 가져오면서 1.x 라이선스만 적용할 수 있다고 가정하지 않는다. 이 문서는 특정 재사용물의 법적 검토 완료를 뜻하지 않는다.

## 실행한다면 가장 작은 다음 단위

첫 구현 단위로 **완료된 결과의 복구 패널**을 권고한다. 기존 삽입 보호를 유지하면서 데스크톱 `InsertResult`와 Android 전달 실패를 사용자에게 보이는 복구 경로로 연결한다. 큰 모델 교체나 원격 프로토콜 교체 없이도 효용을 검증하기 쉽다.

이후 데스크톱 대상 확인 강화와 서버 내부 사전·교정 요청 구성의 공통화를 각각 분리해 진행한다. timeout·streaming 실험은 다음 기준으로 비교한다.

| 평가 | 제안하는 검증 방식 |
| --- | --- |
| 한국어 품질 | 기존 합성 corpus에 고유명사·숫자·부정·자가 정정·반복 강조·한영 혼용을 포함. 필수 정보 훼손 여부를 먼저 판정 |
| 속도 | 같은 입력·모델·프롬프트·네트워크, cold/warm 분리. 녹음 종료→첫 표시와 녹음 종료→최종 입력의 P50/P95를 따로 측정 |
| 전달 안전 | 다른 앱/같은 앱 다른 칸/사용자 타이핑/취소/부분 출력/늦은 결과를 시험. 오입력·중복 입력·완성 결과 유실은 시험군에서 0건을 목표 |
| 플랫폼 | Windows 설치본, Mac 원격 클라이언트, Galaxy Android에서 각 경로 확인. 소스 단위 테스트 통과를 실기기 성공으로 대체하지 않음 |

**이번 결과물은 조사와 적용 판단이다. 앱 코드 변경·새 모델 적용·새 성능 실측은 하지 않았다.** 복구 UI·개인화 공통화·timeout 변경의 구체적 구현은 다음 작업 범위로 정할 수 있다.

[release-stable]: https://github.com/Open-Less/openless/releases/tag/v1.3.18-tauri
[release-beta]: https://github.com/Open-Less/openless/releases/tag/v2.0.0-Beta.4-tauri
[pr943]: https://github.com/Open-Less/openless/pull/943
[pr966]: https://github.com/Open-Less/openless/pull/966
[stable-recovery]: https://github.com/Open-Less/openless/blob/ed4a1408e25e66fddb203cc758d653c777af6488/openless-all/app/src-tauri/src/coordinator/dictation.rs#L4581-L4618
[stable-card]: https://github.com/Open-Less/openless/blob/ed4a1408e25e66fddb203cc758d653c777af6488/openless-all/app/src/components/InsertFallbackCard.tsx#L35-L137
[stable-learning]: https://github.com/Open-Less/openless/blob/ed4a1408e25e66fddb203cc758d653c777af6488/openless-all/app/src-tauri/src/host_document/mod.rs#L376-L400
[stable-timeout]: https://github.com/Open-Less/openless/blob/ed4a1408e25e66fddb203cc758d653c777af6488/openless-all/app/src-tauri/src/polish.rs#L22-L92
[stable-retry]: https://github.com/Open-Less/openless/blob/ed4a1408e25e66fddb203cc758d653c777af6488/openless-all/app/src-tauri/src/polish.rs#L1506-L1549
[beta-target]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/openless-all/app/src-tauri/src/selection.rs#L35-L78
[beta-mac-focus]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/openless-all/app/src-tauri/src/host_document/macos.rs#L402-L415
[beta-remote]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/openless-all/app/src-tauri/src/core_adapters.rs#L1345-L1382
[beta-context]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/openless-all/app/crates/openless-core/src/api.rs#L6917-L6942
[beta-correction]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/openless-all/app/crates/openless-core/src/dictation_engine.rs#L613-L619
[beta-idle]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/openless-all/app/crates/openless-core/src/polish.rs#L1002-L1039
[beta-history]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/openless-all/app/src/pages/History.tsx#L788-L848
[beta-learning-win]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/openless-all/app/src-tauri/src/host_document/windows.rs#L231-L314
[beta-learning-core]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/openless-all/app/crates/openless-core/src/api.rs#L5240-L5338
[beta-stream]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/openless-all/app/crates/openless-core/src/streaming_insert.rs
[beta-models]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/openless-all/app/crates/openless-core/src/local_asr_catalog.rs#L7-L94
[beta-architecture]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/docs/architecture.md
[license-stable]: https://github.com/Open-Less/openless/blob/ed4a1408e25e66fddb203cc758d653c777af6488/LICENSE
[license-beta]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/LICENSE
[license-transition]: https://github.com/Open-Less/openless/blob/6f9af89583d542ca55e4dbe8b326a47629fc3957/README.md#L430-L433
