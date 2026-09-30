# Ultra STT 전용 Codex 분리 검토

2026-09-30. 최초 검토와 승인 후 분리 실험의 기록이다. 사용자가 전용 로그인을 완료했고 품질을 유지하면 분리 운영하겠다고 승인했다. 전역 Codex 파일·인증 파일·토큰은 수정하거나 복사하지 않았다. 아래 최초 검토의 '현재'는 분리 이전 경로를 뜻하며 최신 운영 결과는 `HANDOFF.md`의 후속 항목을 따른다.

## 현재 실행 범위

앱은 별도 native `codex exec --ephemeral` 프로세스를 띄운다. 개발 대화 세션을 이어 쓰거나 현재 대화 이력을 보내지 않는다. `--ignore-user-config`로 사용자 `config.toml`을 무시하고 모델/Fast/추론/도구 차단/권한을 앱이 명시한다. 프로젝트 지침은 `project_doc_max_bytes=0`, 모델에게 보이는 스킬 이름·설명 목록은 `skills.include_instructions=false`로 제외한다. 이름·설명 목록을 제거했다는 뜻이지 스킬을 실행해서 처리하던 음성이 있었다는 뜻이 아니다. 도구 실행은 별도 가드로 금지한다.

하지만 같은 `CODEX_HOME`을 사용하므로 그 홈의 `AGENTS.override.md` 또는 `AGENTS.md`가 별도 전역 로더를 통해 남는다. Codex 모델의 기본 개발용 지시문도 남는다. 따라서 **기본 세팅 전체를 그대로 가져오는 것은 아니지만 완전히 앱 전용으로 분리된 상태도 아니다.**

확인한 공식 소스 SHA: `2e5fea64eefcaa19f48458b2386011b619f69c70`.

- [전역 지시문 로더](https://github.com/openai/codex/blob/2e5fea64eefcaa19f48458b2386011b619f69c70/codex-rs/codex-home/src/instructions/mod.rs#L40): `CODEX_HOME`의 파일을 직접 읽는다.
- [프로젝트 지침 로더](https://github.com/openai/codex/blob/2e5fea64eefcaa19f48458b2386011b619f69c70/codex-rs/core/src/agents_md.rs#L56): 프로젝트 max bytes 제한은 이 경로에 적용된다.
- [공식 AGENTS 안내](https://learn.chatgpt.com/docs/agent-configuration/agents-md): 독립 프로필에 `CODEX_HOME`을 지정할 수 있다.

## 가장 단순하고 안전한 완전 분리

1. Ultra STT만 사용하는 홈(예: 프로젝트 runtime의 `codex-app-home`)을 따로 만든다. executable은 기존 공식 native CLI를 공유해도 되며 npm/CLI를 중복 설치할 필요가 없다.
2. 그 홈에는 음성 정리·번역·선택 편집·질문에 필요한 짧은 지침만 둔다. Skills/MCP/plugins/hooks/memories를 넣지 않고 앱의 read-only/tool 차단을 유지한다.
3. 기본 개발용 모델 지시문은 `model_instructions_file`로 앱 전용 지침으로 교체하는 후보를 시험한다. 이 설정은 기존 지침에 덧붙이는 것이 아니라 **모델 기본 지시문 교체**이므로 기존 사전·교정·문맥·정정·부정·불확실성·선택 편집·번역·질문 품질 검사를 모두 재실행한다. 짧아졌다는 이유만으로 채택하지 않는다.
4. 사용자 승인 후 새 홈에서 같은 ChatGPT 계정으로 공식 로그인한다. 이전 홈의 `auth.json` 복사/심볼릭 링크/토큰 추출은 하지 않는다. native 로그인 검사와 exec 모두 같은 앱 홈을 사용해야 한다. 별도 로그인은 별도 구독이나 무료 추가 한도를 의미하지 않는다.
5. 전후 동일 corpus에서 전체 지연·입력 토큰·품질을 비교해 검증 후 전환한다. 기존 개발 Codex 세션/설정/로그인은 그대로 보존한다.

[인증 파일](https://github.com/openai/codex/blob/2e5fea64eefcaa19f48458b2386011b619f69c70/codex-rs/login/src/auth/storage.rs#L162)은 홈에 속하고, [keyring account](https://github.com/openai/codex/blob/2e5fea64eefcaa19f48458b2386011b619f69c70/codex-rs/login/src/auth/storage.rs#L245)도 홈 경로 해시로 구분된다. 그래서 완전 분리와 기존 로그인 재사용은 자동으로 동시에 성립하지 않는다. [공식 인증 설명](https://learn.chatgpt.com/docs/auth).

## 추가 벤치마킹 후보와 한계

공개 앱의 녹음 중 STT/모델 예열/연결 재사용/최종 결과 한 번 삽입/짧은 프롬프트 패턴은 기존 두 조사 보고서에 source permalink와 함께 기록돼 있다. 경쟁 앱을 설치해 같은 계정/장비/음성으로 실제 응답 시간을 측정한 것은 아니다. 제품 광고 속도와 우리 실측을 직접 비교하지 않는다.

- 최우선 후보는 **앱 전용 홈과 앱 전용 기본 지시문의 분리 비교**다. 현재 남는 수 초 구간이 AI이고, 전역 개발 지침을 읽어야 할 이유가 없다. 토큰 감소가 지연 감소를 보장하지 않으므로 확인이 필요하다.
- login preflight를 최종 STT와 겹치면 기존 측정상 약 121–159ms를 숨길 가능성이 있다. 두 작업의 취소/로그인 실패를 함께 처리해야 하며 큰 폭의 개선 근거는 아니다. 현재 인증 확인은 생략하지 않는다.
- 녹음 중 외부 AI 선전송이나 부분 결과 삽입은 사용자 데이터 전송 시점·정확도·취소·커서 복구를 바꾼다. 별도 설계/승인/품질 검증 대상이다.
- 항상 켜진 마이크로 녹음 준비를 줄이는 방식은 개인정보와 장치 점유를 바꾸므로 이번 작업에 조용히 도입하지 않는다.
- 더 작은 STT/다른 LLM/API provider/무조건 AI 생략은 품질·모델·인증 보존 조건과 다르므로 미적용이다.

## 전용 홈 운영과 복구

Windows 중앙 서버의 전용 홈은 프로젝트 루트 `runtime/codex-app-home`이다. 공식 native CLI는 기존 executable을 공유하지만 자식 프로세스의 `CODEX_HOME`만 교체한다. ChatGPT 공식 로그인과 `login status`를 사용하며 인증 파일을 복사하거나 읽어 추출하지 않는다. 기존 개발 로그인도 별도로 정상 확인했다.

전용 `AGENTS.md`와 `model-instructions.md`는 각각 `src-tauri/resources/codex-app-AGENTS.md`, `codex-app-instructions.md`에서 준비한다. Codex 자체가 만드는 내부 DB/cache/built-in skills 디렉터리는 남을 수 있으나 개발 홈의 Skills/MCP/plugins/hooks/memories를 복사하지 않고 기능 및 모델 스킬 목록 차단을 유지한다. `--ephemeral`은 영구 대화 이력 재사용을 막으며 자체 진단 DB까지 없앤다는 뜻은 아니다.

전환·복구 명령은 앱 디렉터리에서 실행한다.

```powershell
pwsh -NoProfile -File scripts/configure-codex-profile.ps1 -Login
pwsh -NoProfile -File scripts/configure-codex-profile.ps1 -Profile isolated
pwsh -NoProfile -File scripts/configure-codex-profile.ps1 -Profile shared
```

첫 명령은 최초 준비 또는 전용 재로그인 때만 필요하다. `shared`는 기존 인증·설정·모델을 삭제하지 않고 종전 실행 경로로 복구한다. 마커는 프로젝트 루트 `runtime/codex-profile`이며 요청 시작 시 읽고 로그인 검사와 exec에 같은 홈을 사용한다. 운영 빌드에서는 상속된 `ULTRA_STT_CODEX_PROFILE`이 마커를 덮어쓰지 못한다. 이 환경변수는 native test에서만 비교용으로 허용한다. 전용 인증 실패는 명시적 오류이며 조용히 개발 홈으로 되돌리지 않는다.

Mac의 현재 `ultra-remote` 경로는 이 Windows 서버에서 다듬으므로 Mac에 두 번째 Codex 로그인은 필요 없다. 일반 받아쓰기는 Medium + Fast, 사전 또는 활성 교정 규칙·선택 편집·번역·질문·별도 스타일 등 품질 민감 작업은 High + Fast다. 짧은 지침의 초기 사전 표기 누락을 숨기지 않고 canonical spelling 우선순위를 보완하고 사전/교정 추론 강도를 유지하도록 조정했다.

최종 단문 교차 비교에서 입력 토큰은 12,503 → 6,618로 약 47.1% 줄었다. 앞선 두 후보의 결과도 보존하며 속도는 클라우드 변동이 커 고정 개선율을 보장하지 않는다. 입력 토큰 감소가 구독 사용량/크레딧의 같은 비율 감소를 뜻하지는 않는다. 캐시 입력·출력·추론 토큰과 Fast 배율도 영향을 준다. [공식 사용량·가격 설명](https://learn.chatgpt.com/docs/pricing).
