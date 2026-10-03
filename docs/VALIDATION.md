# Ultra STT 1.1.62 — 검증·성능·알려진 한계

**기준일:** 2026-10-03 (한국 시간; 주요 기능 검증은 10월 2일)

**대상:** Windows x64·macOS Apple Silicon·Android 개인 설치

[제품 안내](../README_ko.md) · [기능 사양](FEATURES.md)

## 테스트·설치 결과

**10월 3일 데스크톱 후속 수정:** 최상단 표시 회귀 검사를 포함해 frontend 416개, Windows native 630개 통과(3 ignored), production build와 Windows 설치 파일 빌드가 통과했다. Home에서 다른 창 위에 녹음창이 표시되는 것을 사용자가 확인했다. 아래 표는 기존 릴리스의 검증 기록이며, 새 설치 현황·수동 검증 범위는 [최상단 표시 수정 기록](2026-10-03-capsule-topmost.md)을 따른다.

| 확인 항목 | 결과 | 범위 |
| --- | --- | --- |
| Frontend | 44개 파일, 414개 테스트 통과 | UI 상태, 모드 선택·저장, 캡슐 흐름, resize, 기존 기능 |
| Windows native | 629 통과, 0 실패, 3 ignored | pipeline·취소·설정·prompt·원격 헤더·기존 native 기능 |
| Mac native | 635 통과, 0 실패, 3 ignored | Mac 조건부 코드 포함 native suite |
| TypeScript / frontend production | 통과 | 타입 검사와 Vite production build |
| Windows 설치 | 1.1.62 NSIS 빌드·설치, payload 검사 | 기존 앱 백업, 개인 설정·기록 경로 보존 |
| Mac 설치 | 1.1.62 앱 빌드·서명 검사·설치 | 이전 앱 백업, 식별자·설정 보존, 기록 15건 유지 |
| 실제 Windows UI | Home·AI·일반 설정 캡처 | 설치 앱에서 모드·현재 선택 표시와 기록 그래프 확인 |
| Mac → Windows 원격 smoke | 통과 | 다섯 모드·legacy·AI 비활성화·잘못된 헤더·인증 거부 |
| 클라이언트 취소 | 통과 | Mac HTTP 연결 종료 후 설치 서버의 작업 취소 로그 확인 |
| Android 재검증 | Java 80개 검사·APK 빌드·서명 검사 통과 | Android Lint 오류 0; debug 서명, SDK XML·Gradle 경고는 남음 |
| Android .9 입력창·UI | 실기기 시나리오 18개 통과 | stale-focus 재현/비밀번호/비포커스 거부, 중앙창 상태·취소, 기존 입력창·키보드 회귀 |
| Android .9 홈 위젯 | ChatGPT 5/5, Grok 5/5 | 실제 위젯 터치 후 입력칸 재터치 없이 버튼 표시; 각각 녹음·숨김·Home 취소 확인 |
| 집 PC 설치 | 1.1.62 설치·실행·서버 인증 통과 | 사용자 console 세션, portable runtime marker, NSIS payload 일치, 평문 토큰 없는 설정, HTTPS 인증 probe |

기본 실행에서 제외한 native live 테스트는 외부 로그인·서비스와 사용량이 필요하다. 기본 suite 숫자를 모든 제공자에 대한 실마이크 acceptance 숫자로 해석하지 않는다. UI 테스트에는 의도적인 오류 경로의 stderr와 animation mock 관련 경고가 있을 수 있으며, 통과·실패 결과와 구분한다.

### 최신 물리적 사용 확인

Windows 모드 전환·취소는 이전 사용자 확인과 앱 로그가 있다. Mac의 기존 마이크·커서 입력 문제는 이전 사용자 확인을 받았으나, **이번 1.1.62 설치 후 실제 마이크 → 평소 입력창 → 물리적 Esc 키의 최종 확인은 별도 수동 항목**이다. 실제 원격 HTTP와 native 테스트 통과가 이를 대체하지 않는다.

## GitHub 자동 검사 상태 — 로컬 기능 검증과 별도

**후속 수정:** 사용자 요청으로 Typos 오탐·Lock Threads 토큰 호환성을 수정하고, 현재 개인 비공개 저장소에서 미지원인 CodeQL은 분석 미실행을 명시하도록 정리했다. [원인·수정·지원 범위](github-actions-personal.md). 별도 CI의 서식·의존성 감사 문제는 남아 있다. 아래는 수정 전 릴리스의 역사적 기록이며 현재 결과를 덮어쓰지 않는다.

아래는 **이전 개인 프리뷰 1** 소스 `df1ef499104bf4e5f89c55ee4ab7b8c025d20d22`의 GitHub 검사 기록이다. **CI 전체가 통과한 릴리스는 아니다.** 개인 프리뷰 2 역시 기존 데스크톱 소스·npm 의존성·workflow를 변경하지 않았다. 아래 결과를 새 커밋의 검사 결과로 간주하지 않는다. 최신 상태는 [GitHub Actions](https://github.com/toughCSB/ultra-stt/actions)에서 확인한다.

- [CI](https://github.com/toughCSB/ultra-stt/actions/runs/37024983654): 기존 Rust `cargo fmt --check`와 frontend Prettier 서식 검사가 실패했다. `npm audit`는 4건(High 1, Moderate 3)을 보고했고, 그 뒤 Cargo audit는 건너뛰었다. 현재 High는 `brace-expansion 5.0.9`이며 lockfile에서 개발 의존성으로 확인했다. 이를 보안 검사 통과로 표시하지 않는다.
- [Typos](https://github.com/toughCSB/ultra-stt/actions/runs/37024983650): 기존 코드·다국어 문자열의 `ine`, `Profesional`, `Profissional` 지적으로 실패했다.
- [CodeQL](https://github.com/toughCSB/ultra-stt/actions/runs/37024983759): checkout에서 이 private 저장소에 대한 `repository not found`/exit 128로 실패했으므로 코드 분석이 완료되지 않았다. 개인 설치·릴리스를 위해 저장소 공개 전환이나 workflow 권한 변경을 하지 않았다.

위 실패는 실제 설치·음성 입력 성공이나 로컬 테스트 통과와 구분한다. 개인 프리뷰의 제한으로 공개하고, 의존성 업데이트·기존 서식 정리·CI 권한 수정은 별도 작업으로 남겼다.

## 원격 경로 실측

Mac에서 같은 한국어 음성 fixture를 설치된 Windows 서버로 전송했다. 문장은 “맥에서 음성 입력을 테스트하고 있습니다.”이다. 다음은 각 구성의 **1회** 요청 완료 시간이다.

| 요청 | 완료 시간 |
| --- | ---: |
| Original + AI | 5,023 ms |
| Minimal + AI | 4,527 ms |
| Clean + AI | 4,817 ms |
| Structured + AI | 6,256 ms |
| Professional + AI | 9,438 ms |
| 헤더 없는 기존 클라이언트 | 4,512 ms |
| AI 비활성화 | 656 ms |

일곱 성공 요청 모두 fixture 문구를 보존했다. 잘못된 헤더는 400, 인증 없는 요청은 403으로 거부했다. 연결 취소 시험은 서버의 `Remote transcription cancelled by client disconnect` 로그로 확인했다.

**해석 주의:** 이 수치는 녹음 시간·단축키 반응·최종 커서 삽입을 포함하지 않는다. 단일 요청씩이므로 스타일 간 확정적인 속도 순위도 아니다. 서버 부하·클라우드·네트워크 변동이 있으며, AI 비활성화는 품질 조건이 다른 구성이지 기존 AI 품질을 그대로 유지한 속도 개선이 아니다.

## 이전 최적화 비교

더 자세한 조사·실험 과정과 보류한 후보는 [성능 조사 기록](performance-personal.md), [Codex 분리 검토](codex-isolation-personal.md)에 있다. 오래된 기록의 버전·테스트 수는 당시 시점 기준이다.

| 비교 | 관측 결과 | 해석 |
| --- | --- | --- |
| Mac 원격 2.83초 WAV, 각 5회 | High/Standard 평균 7,160 ms → 자동 Medium/Fast 구성 평균 5,201 ms | 해당 작은 표본의 관측값. 안정적인 27.4% 개선 보장 아님 |
| 최종 앱 전용 프로필, 각 5회 AI CLI | 공유 홈 평균 4,454 ms → 전용 홈 3,899 ms | STT·네트워크·커서 삽입 제외. 앞선 후보에서는 개선이 없었던 경우도 있음 |
| 같은 단문 입력 토큰 | 공유 12,503 → 전용 6,618 | 입력 토큰 약 47.1% 감소. 구독 사용량·요금 절감률과 동일하지 않음 |

실시간 부분 결과 선삽입, 더 작은 STT 모델, 상시 켜진 마이크나 다른 유료 API로의 fallback은 이번 운영 최적화에 적용하지 않았다. app-server 상주 실험도 분명한 이점이 없어 채택하지 않았다.

## 검증 재현

소스 디렉터리에서 실행한다. 새 환경에는 각 플랫폼의 native toolchain이 필요하다.

```sh
npm ci
npm test
npm run build
cargo test --locked --lib --manifest-path src-tauri/Cargo.toml
```

로컬 Cargo cache에 준비된 개인 환경은 `--offline`을 추가할 수 있다. live 테스트는 별도 인증과 사용량이 필요하므로 `--ignored`로 일괄 실행하지 않는다. 원격 통합 시험은 먼저 본인 소유 서버·Tailnet·인증을 준비하고, 비밀값은 명령 기록이나 저장소에 넣지 않는다.

스크린샷·새 문서는 앱 설정을 바꾸거나 기록 DB에 예시 데이터를 넣어서 만들지 않았다. 상세 개인 QA 로그·auth 파일·DB는 저장소에 포함하지 않고 검증 요약만 남긴다.

## 스크린샷 출처

| 파일 | 촬영 내용 |
| --- | --- |
| [ultra-stt-dashboard.png](images/ultra-stt-dashboard.png) | 홈: 다섯 모드, Minimal 체크, 날짜별 기록 건수, 현재 제공자 |
| [ultra-stt-ai-settings.png](images/ultra-stt-ai-settings.png) | AI 설정: 구독 Codex, 모델, 활성 여부, 스타일 |
| [ultra-stt-shortcuts.png](images/ultra-stt-shortcuts.png) | 일반 설정: 단축키, 녹음·출력 방식 |
| [android-recording-panel-9.png](images/android-recording-panel-9.png) | 2026-10-03 갤럭시 .9: 작은 반투명 중앙창·원본 로고·모드·타이머·종료 버튼, 빈 QA 입력칸 위 실제 촬영 |

위 Windows 세 장은 2026-10-02 설치된 Windows 1.1.62의 실제 화면이다. Android 한 장은 다음 날 실기기 테스트 중 촬영했고 개인 대화가 아닌 빈 QA 입력칸을 사용했다. 화면 요소나 기록 내용을 합성하지 않았다. API 키·개인 전사 본문은 캡처하지 않았다. Mac의 실제 화면이라고 표시하지 않는다. 버튼 그림은 [별도 생성 자산](polish-mode-assets.md)이다.

## 알려진 한계·남은 작업

- **AI 지연:** 현재도 수 초가 걸린다. Fast 요청이 SLA는 아니며 Typeless와 동일 조건의 비교 검증은 하지 않았다.
- **Mac 최종 acceptance:** 최신 설치 후 평소 마이크·입력창·물리적 Esc 확인이 남는다.
- **Mac 원격의 고급 설정:** 현재는 모드·활성 여부만 전달한다. 로컬 선택 텍스트·번역·씬·사전 설정 전체를 전달하지 않는다.
- **기록 그래프:** 전체 DB가 아닌 현재 불러온 최대 200건 기준이다. 처리 시간은 세션의 마지막 작업 기준이다.
- **플랫폼:** Windows x64·Mac arm64 외 Linux·Mac Intel·Windows ARM은 이번 설치 검증 대상이 아니다.
- **배포:** [개인 프리뷰 2](https://github.com/toughCSB/ultra-stt/releases/tag/v1.1.62-personal.2)는 Android .9와 이전 그대로인 Windows·Mac 수동 설치 파일을 제공한다. Windows 미서명·Mac ad-hoc 서명·Android debug APK이며, 자동 업데이트용 서명 패키지·`latest.json`은 게시하지 않는다. 개인 Mac 서명은 notarization이 아니다.
- **집 PC:** SB 연결과 실제 음성→커서 입력은 확인했다. 사용자가 기존 무반응 보고를 “녹음창이 다른 창 뒤에 가려진 것”으로 정정했다. 최상단 표시 수정본 설치 후 다른 창 위에 잘 뜬다는 실사용 확인을 받았다. 단축키(Ctrl+/)·설정은 유지했고 절전 복구 서비스·새 프로그램은 추가하지 않았다. 당시 정확히 어떤 창이 가렸는지는 기록이 없으며, 이 수정이 모든 절전 관련 문제를 해결했다는 의미는 아니다. 원격 받아쓰기에 집 PC의 별도 Codex 로그인은 필요 없다.
- **모바일:** Android .9는 18개 기기 시나리오 및 실제 위젯 진입 검증을 통과했다. 이전 .5에서 HTTPS·잘못된 인증 거부 등을 포함한 통신 10개 검사와 사용자 실발화 입력이 확인됐다. 최신 .9 자동 검사는 발화를 업로드하지 않았으며 서버 왕복·문장 삽입·자동 완료의 새 전체 경로 확인을 의미하지 않는다. 모든 외부 편집기를 검증한 것은 아니다. iOS는 브라우저 복사·공유 단계다. [Android 범위·설정·검증](../android/README.md).
- **제공자·언어:** 기능 범위는 선택한 제공자에 따라 다르고, 일부 고급 UI 설명은 영어 fallback이다.
