# CI 실패 원인과 재발 방지 — 2026-10-07

## 현재 상태

- 기준: 사용자 소유 저장소 `toughCSB/ultra-stt`, `main`의 `d6deacd61085532aaf135417b5c1ac878186b754`.
- [실패 실행](https://github.com/toughCSB/ultra-stt/actions/runs/37484752544)의 서식 오류와 npm 취약점을 현재 개발 폴더에서 수정했다. 별도 사본을 덮어쓰지 않았다.
- 아래 결과는 **로컬 Windows 소스 검증**이다. 수정본의 GitHub Windows/macOS/Linux CI는 아직 실행하지 않았으며, 원격 반영은 사용자 승인 대기다.
- 설치된 SB/Home/Mac/Android 앱, 인증·설정·기록 및 실행 중인 STT 서버는 변경하지 않았다. 버전·릴리스도 변경하지 않았다.

## 무엇 때문에 실패했나

1. 프런트엔드 30개 파일의 Prettier 서식 오류와 Rust 서식 오류가 후속 테스트보다 먼저 실패했다.
2. 재검사 시점의 npm audit에는 취약점 6개(높음 3개, 중간 3개)가 있었다. 사용자에게 전달된 이전 시점의 5개와 구분한다. `brace-expansion` override가 취약한 `5.0.9`에 고정돼 있었다.
3. 서식 오류를 해결한 뒤에는 기존 Rust 경고도 `clippy -D warnings`를 막았다. 불필요한 `mut`, 미사용 인자, 동일 타입 캐스트와 `io::Error` 생성 방식을 동작 변경 없이 정리했다. 빠졌던 WebSocket 보안 테스트의 `#[test]`도 복원했다.
4. 기존 통합 테스트는 개인 Python 환경과 고정 포트 `17845`에 의존했다. 앱 실행 중에는 포트가 충돌하고, Python이 없는 CI에서는 의도한 경로를 검증하지 못할 수 있었다.

## 최소 의존성 변경

기존 lockfile에서 다음 13개 패키지 항목만 갱신/추가했다. 관계없는 최신 버전 갱신은 제외했다.

| 패키지 | 적용 버전 |
| --- | --- |
| `brace-expansion` | `5.0.12` |
| `vitest` | `4.1.11` |
| `@vitest/expect`, `mocker`, `pretty-format`, `runner`, `snapshot`, `spy`, `utils` | `4.1.11` |
| `@humanfs/node` | `0.16.8` |
| `@humanfs/core` | `0.19.2` |
| `@humanfs/types` | `0.15.0` |
| `source-map-js` | `1.2.2` |

`package.json`의 `brace-expansion` override는 `^5.0.12`로 수정했다. 실제 설치 버전은 lockfile로 고정하며, 향후 명시적 갱신에서 안전한 호환 패치를 받을 수 있다. 다른 override는 유지했고 npm audit에 남은 취약점은 없다. `npm audit fix --force`나 감사 제외 규칙은 사용하지 않았다.

**Rust도 사용자 승인 후 패치:** `wayland-scanner`를 `0.31.10 → 0.31.11`로 갱신하고, 취약한 `quick-xml 0.39.4` 항목을 제거해 기존 안전 버전 `0.41.0`만 사용하도록 했다. crates.io의 공식 의존성과 checksum을 확인했으며 다른 Rust 패키지는 갱신하지 않았다. CI의 `RUSTSEC-2026-0194`/`0195` 제외 인자와 오래된 예외 설명을 삭제했다. 로컬에 없는 `cargo-audit`는 새로 설치하지 않았으므로 전체 Rust 보안 감사 통과 여부는 GitHub 실행 후 확인해야 한다.

## 단일 검증 명령과 훅

```sh
npm ci
npm run verify
npm run hooks:install
```

`npm run verify` 순서:

1. Prettier → TypeScript → ESLint → 검증 실행기 회귀 테스트 → Vitest.
2. `cargo fmt --check` → 프런트엔드 빌드 → `cargo clippy --locked ... -- -D warnings`.
3. 테스트 전용 `process_fixture` 빌드 → 전체 `cargo test --locked`.
4. `npm audit --audit-level=moderate`.

실패·도구 부재·프로세스 중단은 성공으로 처리하지 않는다. 자동 설치·자동 서식 수정·앱 교체는 하지 않는다. npm 12에서 상위 npm의 `allow-scripts` 환경 변수가 하위 audit에 CLI 권한처럼 전달되어 실패하는 문제는 **audit 자식 프로세스에만** 해당 변수를 제거해 해결한다. 전역 npm 설정은 수정하지 않으며 audit에는 lifecycle script 실행 금지도 명시한다.

`hooks:install`은 별도 패키지 없이 저장소 로컬 `core.hooksPath`를 설정한다. 기존 훅이 있으면 덮어쓰지 않고 실패한다. 훅은 커밋되지 않은 추적 파일 변경이 있으면 먼저 중단하고, 그 밖에는 전체 `verify`를 실행한다. 사용자 환경에 설치하는 훅은 강제 보안 경계가 아니므로 GitHub CI도 유지한다.

CI는 같은 실행기를 `--frontend`, `--native --target <triple>`, `--security`로 나눠 호출한다. 기존 job 이름과 Windows/macOS/Linux x64/ARM64 행렬을 유지했다. Rust 보안 감사는 기존 별도 CI 단계이며 로컬 `verify`에 포함된 npm 감사와 혼동하지 않는다.

## 통합 테스트 격리

- 실제 개인 Python/Whisper 서버 대신 빌드된 테스트 helper를 임시 폴더에서 실행한다.
- 운영 포트 `17845` 대신 OS가 배정한 임시 포트를 사용하고, 외부 listener의 생존과 종료 후 포트 재사용을 확인한다.
- helper 시작 증거를 확인해, 실행 파일 부재 때문에 실패한 것을 정상 테스트 통과로 오인하지 않는다.
- 테스트 실행 파일 위치에서 fixture 경로를 계산하므로 `--target` 빌드에서도 작동하도록 했다.

이 검사는 네이티브 어댑터의 실패 처리/소유권 회귀 검사다. **실제 Python 서버·음성 인식·마이크·커서 입력 E2E 검증은 아니다.**

## 실제 통과한 검사

Windows에서 `npm run verify`가 종료 코드 `0`으로 완료됐다.

| 검사 | 결과 |
| --- | --- |
| Prettier / TypeScript / ESLint | 통과 |
| 검증 실행기 Node 테스트 | 7 통과 |
| Vitest | 439 통과 |
| Rust fmt / clippy (`-D warnings`) | 통과 |
| Rust 라이브러리 | 659 통과, 기존 3 ignored 유지 |
| 프로세스 / 어댑터 / UIA helper 통합 테스트 | 각각 5 / 1 / 3 통과 |
| 프런트엔드 production build | 통과; 기존 번들 크기·동적 import 경고는 남음 |
| npm audit (moderate 이상 실패) | 취약점 0 |
| Git pre-push의 미커밋 변경 거부 | 확인; 실제 push는 실행하지 않음 |

로컬 기록은 저장소 밖 `qa/ci-verify-all.log`에 있다. macOS/Linux 실행, 수정본 GitHub 행렬, Rust 보안 감사, 실제 앱 동작 검증은 위 결과에 포함하지 않는다.

## 문서만 변경될 때 CI 비용

검토 결과, 이번 수정에서는 workflow 전체를 `paths-ignore`로 생략하지 않는다. 필수 체크인 workflow가 경로 필터로 생략되면 Pending으로 남아 병합을 막을 수 있다. [GitHub 공식 설명](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/skip-workflow-runs).

이 저장소의 필수 체크 설정은 API 접근 제한으로 확정하지 못했다. 따라서 먼저 전체 플랫폼 CI가 정상 통과하는 기준을 확보한다. 향후 비용 최적화는 workflow와 기존 job 이름을 항상 유지하고, 변경 경로를 판별한 뒤 **job 내부의 무거운 단계만** 조건부 실행하는 방식으로 할 수 있다. 경로 판별 실패 시 전체 검사를 실행하고 workflow·스크립트·lockfile 변경은 반드시 전체 검사 대상으로 포함해야 한다. 이번에는 체크 생략을 구현하지 않았다.

## 남은 승인과 검증

1. 사용자 승인 후 관련 변경만 커밋·원격 반영. 기존 미추적 조사 문서 3개는 제외한다.
2. GitHub의 프런트엔드·보안·Windows/macOS/Linux x64/ARM64 결과를 끝까지 확인하고, 로컬 통과와 구분해 기록한다.

## 보안 근거

- [RustSec RUSTSEC-2026-0194](https://rustsec.org/advisories/RUSTSEC-2026-0194.html), [RUSTSEC-2026-0195](https://rustsec.org/advisories/RUSTSEC-2026-0195.html): 안전 버전 `quick-xml >=0.41.0`.
- [crates.io wayland-scanner 0.31.11 의존성](https://crates.io/api/v1/crates/wayland-scanner/0.31.11/dependencies): `quick-xml ^0.41`.
