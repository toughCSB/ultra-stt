# 2026-11-14 의존성 유지보수 결정

사용자 결정: 필수 수정이나 실제 성능 개선이 확인되지 않은 업데이트는 보류하고, 확인된 보안 취약점만 최소 범위로 수정한다.

## 적용하는 보안 수정

- 기존 개발 의존성 `minimatch`에 `^11.0.3` override를 적용한다. GHSA-g3hv-75wg-m249의 취약한 버전을 개발 도구가 계속 선택하지 않도록 한다.
- 해당 버전이 요구하는 `@isaacs/nonrecursive-glob-match`는 전이 개발 의존성이다. 사용자에게 범위를 설명했고 2026-11-14 유지·마무리 승인을 받았다.
- 이 경로는 ESLint/typescript-eslint의 파일 패턴 처리에 사용된다. 전사 엔진·다듬기 모델·프롬프트·녹음 정밀도 변경이나 처리 속도 향상을 뜻하지 않는다.
- 보안 검사나 푸시 전 검증을 생략하지 않는다. 앱 버전은 이미 올린 데스크톱 1.1.73, Android 1.1.73-android.14를 유지한다.

## 보류하는 PR

| PR                                                                   | 보류 이유                                                                   |
| -------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| [#1 reqwest](https://github.com/toughCSB/ultra-stt/pull/1)           | TLS 기본 구현 변경에 대한 실제 통신 회귀 검증과 적용 필요성이 확인되지 않음 |
| [#3 uuid](https://github.com/toughCSB/ultra-stt/pull/3)              | 회귀는 발견되지 않았으나 필수 수정·앱 성능 개선 근거 없음                   |
| [#5 serde_json](https://github.com/toughCSB/ultra-stt/pull/5)        | 같은 이유로 보류                                                            |
| [#6 tokio-tungstenite](https://github.com/toughCSB/ultra-stt/pull/6) | 기존 Message 생성 코드가 새 타입과 맞지 않아 컴파일 오류 재현               |
| [#7 anyhow](https://github.com/toughCSB/ultra-stt/pull/7)            | 회귀는 발견되지 않았으나 필수 수정·앱 성능 개선 근거 없음                   |
| [#12 개발 도구](https://github.com/toughCSB/ultra-stt/pull/12)       | 서식 검사 실패, 지원 Node 범위 변경, 기존 취약점 미해결                     |
| [#15 26개 업데이트](https://github.com/toughCSB/ultra-stt/pull/15)   | TypeScript와 typescript-eslint의 peer 조건 충돌로 설치 실패                 |

보류는 PR을 머지하지 않고 열어 두는 결정이다. 승인 리뷰·댓글 게시·닫기·자동 머지는 하지 않는다. 향후 보안·호환성상 필요가 확인되면 당시의 새 PR head를 다시 검토한다. 큰 버전 업데이트를 전사 속도 최적화로 간주하지 않는다.

전체 PR 검토 증거는 프로젝트 상위 `qa/pr-review-20261114/REVIEW.md`와 개별 로그에 있다. 이 문서의 일부 검증은 최소 재현 또는 프런트엔드 검사이며, 모든 PR의 전체 native build·마이크 실사용 통과를 의미하지 않는다.
