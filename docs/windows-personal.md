# Ultra STT Windows 개인 설치

Windows 개인 설치는 기존 Python 환경, Whisper large-v3 모델, 별도로 로그인한 Codex 앱 프로필과 로컬 `data` 설정/기록을 재사용한다. 다른 컴퓨터에 배포할 때는 각 기기에서 필요한 모델·인증·런타임을 별도로 준비해야 한다.

소스 폴더에서 기존 도구만으로 빌드/설치한다.

```powershell
powershell -NoProfile -File scripts/build-windows.ps1 -Install
```

스크립트는 프로젝트 Cargo cache와 Visual Studio의 CMake를 사용하고 프런트엔드/네이티브 빌드 후 NSIS 설치 파일을 만든다. 기존 앱 실행 파일을 프로젝트 `runtime/app-backups`에 백업하고, 정확한 프로젝트/설치 경로의 Ultra STT 프로세스만 닫는다. `-Install`은 현재 사용자 설치를 수행하고 payload와 runtime marker를 확인한 뒤 설치 앱을 실행한다. 설정, 기록과 토큰을 지우지 않는다.

설치 경로는 `%LOCALAPPDATA%/Ultra_STT/Ultra_STT.exe`다. 바탕 화면/시작 메뉴 바로가기도 이 파일을 실행한다. 옆의 `server-root.txt`로 기존 개인 runtime의 절대 경로를 읽고 필수 파일을 검증한 뒤 설정을 초기화한다. 외부에서 명시한 `ULTRA_STT_ROOT` 또는 `ULTRA_STT_DATA_DIR`은 자동 덮어쓰지 않는다.

Tauri의 NSIS bundle은 컴파일 산출물의 `__TAURI_BUNDLE_TYPE_VAR_UNK` marker를 `NSS`로 패치한다. 원본 EXE와 설치 EXE의 raw SHA256이 다른 것만으로 잘못된 설치라고 판단하지 않는다. 빌드 helper는 이 정확한 marker만 NSIS 형태로 정규화해 전체 payload를 검증한다.

2026-09-30 실제 설치와 UI 동작은 프로젝트 `qa/windows-install-receipt.json`, `qa/windows-history-activity.png`, `qa/windows-history-date-filter.png`에 기록했다. 기존 Windows 기록 13건을 보존했으며, 현재 그래프는 날짜별 실제 불러온 기록 수를 보여 준다. 최대 200건이라는 기존 조회 범위를 화면에도 명시한다.

로그는 프로젝트 `runtime/windows-updated.log`, `runtime/windows-updated.err`다. 모델의 최초 준비가 백그라운드에서 완료된 뒤 받아쓰기를 사용하면 첫 요청에서 모델을 로드하는 대기를 피할 수 있다. Ctrl+Alt+R로 녹음하고 완료된 문장이 목표 커서로 들어오는지 확인한다. 다른 앱으로 초점이 바뀌면 기존 안전한 클립보드 fallback을 유지한다.

실측 및 남은 AI 지연은 [성능 점검](performance-personal.md)을 참고한다. Mac 빌드/권한 안내는 [macOS 개인 설치](macos-personal.md)에 있다.
