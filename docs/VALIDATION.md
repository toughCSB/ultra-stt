# Ultra STT 1.1.62 — 검증·성능·알려진 한계

**기준일:** 2026-10-03 (한국 시간; 주요 기능 검증은 10월 2일)

**대상:** Windows x64·macOS Apple Silicon·Android 개인 설치

[제품 안내](../README_ko.md) · [기능 사양](FEATURES.md)

## 테스트·설치 결과

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
| 집 PC 설치 | 1.1.62 설치·실행·서버 인증 통과 | 사용자 console 세션, portable runtime marker, NSIS payload 일치, 평문 토큰 없는 설정, HTTPS 인증 probe |

기본 실행에서 제외한 native live 테스트는 외부 로그인·서비스와 사용량이 필요하다. 기본 suite 숫자를 모든 제공자에 대한 실마이크 acceptance 숫자로 해석하지 않는다. UI 테스트에는 의도적인 오류 경로의 stderr와 animation mock 관련 경고가 있을 수 있으며, 통과·실패 결과와 구분한다.

### 최신 물리적 사용 확인

Windows 모드 전환·취소는 이전 사용자 확인과 앱 로그가 있다. Mac의 기존 마이크·커서 입력 문제는 이전 사용자 확인을 받았으나, **이번 1.1.62 설치 후 실제 마이크 → 평소 입력창 → 물리적 Esc 키의 최종 확인은 별도 수동 항목**이다. 실제 원격 HTTP와 native 테스트 통과가 이를 대체하지 않는다.

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

모두 2026-10-02 설치된 Windows 1.1.62의 실제 화면이다. 창 바깥과 불필요한 여백만 제외하여 직접 캡처했으며 화면 요소나 기록 내용을 합성하지 않았다. API 키·개인 전사 본문은 캡처하지 않았다. Mac의 실제 화면이라고 표시하지 않는다. 버튼 그림은 [별도 생성 자산](polish-mode-assets.md)이다.

## 알려진 한계·남은 작업

- **AI 지연:** 현재도 수 초가 걸린다. Fast 요청이 SLA는 아니며 Typeless와 동일 조건의 비교 검증은 하지 않았다.
- **Mac 최종 acceptance:** 최신 설치 후 평소 마이크·입력창·물리적 Esc 확인이 남는다.
- **Mac 원격의 고급 설정:** 현재는 모드·활성 여부만 전달한다. 로컬 선택 텍스트·번역·씬·사전 설정 전체를 전달하지 않는다.
- **기록 그래프:** 전체 DB가 아닌 현재 불러온 최대 200건 기준이다. 처리 시간은 세션의 마지막 작업 기준이다.
- **플랫폼:** Windows x64·Mac arm64 외 Linux·Mac Intel·Windows ARM은 이번 설치 검증 대상이 아니다.
- **배포:** [개인 프리뷰 릴리스](https://github.com/toughCSB/ultra-stt/releases/tag/v1.1.62-personal.1)는 Windows·Mac·Android 수동 설치용이다. Windows 미서명·Mac ad-hoc 서명·Android debug APK이며, 자동 업데이트용 서명 패키지·`latest.json`은 게시하지 않는다. 개인 Mac 서명은 notarization이 아니다.
- **집 PC:** SB 서버를 쓰도록 설치·인증·실행을 확인했다. `Ctrl+Alt+R` 토글·Minimal·커서 입력으로 설정했고, 기존 키보드·개발 Codex 로그인은 변경하지 않았다. 새 앱의 자동 시작은 끈 상태로 설치했다. 사용자가 실제 집 PC 음성→커서 입력 성공을 확인했다. 원격 받아쓰기에 집 PC의 별도 Codex 로그인은 필요 없다.
- **모바일:** Android 1.1.62-android.5는 기존 키보드를 유지하며 5색 버튼·반원 드래그·녹음 중 종료 버튼과 시간·처리 회전 표시를 제공한다. 앱 전용 Tailscale IP 연결로 휴대폰의 HTTPS·인증 확인이 통과했다. 잘못된 인증 거부 등을 포함한 기기 통신 10개 검사와 Java 80개 검사도 통과했다. 사용자가 앱 테스트 입력창에서 실제 한국어 음성 전사→커서 입력 성공을 확인했다. 모든 외부 앱의 편집기 호환성을 검증한 것은 아니다. iOS는 브라우저 복사·공유 단계다. [Android 범위·설정·검증](../android/README.md).
- **제공자·언어:** 기능 범위는 선택한 제공자에 따라 다르고, 일부 고급 UI 설명은 영어 fallback이다.
