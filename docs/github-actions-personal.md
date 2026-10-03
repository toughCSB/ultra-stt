# 개인 저장소의 GitHub 자동 검사

GitHub의 `Run failed` 메일은 저장소 자동 작업이 실패했다는 뜻입니다. 설치된 Ultra STT가 충돌했다는 알림이 아닙니다. 다만 검사 실패를 무시하거나 앱이 안전하다고 단정해서도 안 됩니다.

## 2026-10-03 알림 세 건의 원인과 수정

| 작업 | 원인 | 조치 |
| --- | --- | --- |
| Typos | PowerShell의 정상 비교 연산자 `-ine`와 스페인어 `Profesional`, 포르투갈어 `Profissional`을 영어 오타로 판단 | 세 단어만 허용 사전에 등록. 전체 검사·소스 파일을 제외하지 않음 |
| Lock Threads | 기존 v5의 입력 검증이 GitHub 토큰 길이를 100자로 제한 | 길이 검증을 수정한 공식 v6.0.2의 commit `89ae32b08ed1a541efecbab17912962a5e38981c`로 고정. 토큰 교체 없이 기존 GITHUB_TOKEN 사용 |
| CodeQL | `contents: read` 누락으로 비공개 저장소 checkout 실패. 그와 별개로 현재 개인 비공개 저장소는 CodeQL 코드 스캔 이용 조건 미충족 | 읽기 권한을 명시하고, 지원되지 않는 구성에서는 실제 분석 job을 건너뛰며 실행 요약에 **분석 미실행**을 표시 |

Lock Threads는 60일간 활동이 없는 닫힌 이슈·PR을 잠그는 기존 관리 작업입니다. 기간은 바꾸지 않았고, 현재 사용하지 않는 Discussions는 처리 대상에서 제외했습니다. 기존 `issues: write`, `pull-requests: write` 범위는 그대로입니다. 새 개인 토큰이나 PC 설치 프로그램은 필요하지 않습니다. 수동 실행도 가능하도록 `workflow_dispatch`를 추가했습니다.

## CodeQL 보류의 의미

공식 지원 범위는 공개 저장소 또는 GitHub Code Security를 활성화한 적격 조직의 저장소입니다. 저장소를 공개하거나 유료 서비스를 신청하지 않았습니다. 현재 비공개 저장소에서는 안내 job만 실행되고 `analyze-javascript`는 **skipped**여야 합니다. 안내 job의 성공은 취약점 검사의 통과를 뜻하지 않습니다.

나중에 적격 조직으로 이전하고 Code Security를 활성화한 경우에만 저장소 변수 `CODEQL_ENABLED=true`로 분석을 명시적으로 켤 수 있습니다. 이 수정은 그 변수를 만들거나 켜지 않습니다. 실제 분석을 켰을 때의 실패는 숨기지 않으며, `continue-on-error`로 무시하지 않습니다. 공개 저장소에서는 분석이 자동으로 실행됩니다.

## 검증과 남은 범위

- 수정 전 실패 로그: [Typos](https://github.com/toughCSB/ultra-stt/actions/runs/37102810011), [Lock Threads](https://github.com/toughCSB/ultra-stt/actions/runs/37096001065), [CodeQL](https://github.com/toughCSB/ultra-stt/actions/runs/37102809970).
- 수정 후에는 새 main 커밋의 Typos, CodeQL 안내/분석 job 상태, 수동 Lock Threads 실행을 각각 확인합니다. 과거 실패 기록과 이미 받은 메일은 없어지지 않습니다.
- 별도 **CI** workflow의 Rust/Frontend 서식과 의존성 감사 실패는 이 세 알림 수정과 별개의 남은 작업입니다. 해당 검사를 끄거나 성공으로 위장하지 않았습니다. [기존 검증 기록](VALIDATION.md).
- 앱 실행 코드, APK·데스크톱 설치본, 비공개 설정, 기존 릴리스·태그는 변경하지 않습니다.

## 공식 근거

- [Lock Threads v6.0.2 변경 기록 — 토큰 검증 수정](https://github.com/dessant/lock-threads/blob/v6.0.2/CHANGELOG.md)
- [Lock Threads 입력·처리 대상 설정](https://github.com/dessant/lock-threads/tree/v6.0.2)
- [CodeQL 지원 범위](https://docs.github.com/en/code-security/concepts/code-scanning/codeql/codeql-code-scanning)
- [CodeQL Action — private 저장소의 contents: read 필요](https://github.com/github/codeql-action/blob/main/README.md)
