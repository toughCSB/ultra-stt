# 개인 설치 아이콘과 트레이 동작

2026-09-30. 원래 브랜드 PNG는 보존하고, 내장 `image_gen`으로 전파/배경 무늬를 제거한 중앙 녹색 마이크·검정 배경 아이콘을 편집했다. 요청한 모델은 GPT-Image-2.5 Sunburst이며 반환 메타데이터로 실제 모델명을 확인할 수 없었다. 생성 이미지가 주 시각물이고 후처리는 공식 Tauri CLI의 플랫폼 크기/형식 변환과 RGBA 패킹뿐이다.

- 기준 이미지: `src-tauri/icons/ultra-stt-microphone.png`.
- Windows 앱/작업 표시줄: 생성된 PNG/ICO bundle 리소스. main window에 bundle 기본 small icon을 설정하고, `windows_brand.rs`가 현재 EXE의 공유 아이콘 리소스 32512를 `WM_SETICON / ICON_BIG`으로 설정한다. Tao의 일반 `set_window_icon`은 small icon만 설정하므로 큰 아이콘을 별도로 지정한다. `LoadIconW`의 공유 리소스는 삭제하지 않는다.
- Mac 앱/Dock: 생성된 `icon.icns`. 시스템 트레이는 양쪽 모두 64×64 검정 불투명 RGBA의 같은 마이크다. 전파 모양은 없다.
- 재생성: `pwsh -NoProfile -File scripts/build-brand-icons.ps1`. 기존 프로젝트 Tauri CLI만 사용하며 새 패키지를 설치하지 않는다.
- 이전 아이콘 백업: 프로젝트 루트 `runtime/app-backups/icons-20260930-205552/icons`.
- 기존 `app-icon.svg`와 `tray-icon.svg`는 과거 소스로, 최신 개인 빌드의 기준 파일이 아니다. 홈페이지·캡슐·브랜드 원본 PNG는 이번 변경으로 덮어쓰지 않았다.

트레이 왼쪽 버튼 release마다 숨김→표시, 표시→숨김으로 전환한다. 최소화된 창은 먼저 복원한다. Windows의 추가 DoubleClick 알림은 무시해 같은 클릭을 중복 처리하지 않는다. 왼쪽 클릭 메뉴 자동 팝업은 끄고 오른쪽 메뉴는 유지한다. 창 숨김은 앱 종료/녹음 중단이 아니다. 메뉴의 창 보이기/숨기기도 같은 함수를 사용한다.

Windows 설치본의 EXE 리소스와 실제 main window 큰 아이콘은 프로젝트 루트 `qa/check-installed-tray.ps1`로 읽기 전용 검사한다. 녹색 픽셀과 추출 PNG를 확인하며 창 표시 상태나 마우스 위치를 변경하지 않는다. 이것은 실제 Explorer 작업 표시줄 픽셀이나 트레이 클릭 성공 검사를 대신하지 않는다.

최초 아이콘 업데이트 후에도 PE 리소스의 `resource.lib`가 9월 29일 생성본으로 재사용돼 설치 EXE에 옛 아이콘이 남았다. `build.rs`에 `cargo:rerun-if-changed=icons`를 추가해 재발을 방지한다. Explorer 전체 종료·전역 icon cache 삭제·사용자 고정 아이콘 삭제는 하지 않는다.

최신 Windows 스크린샷에서 옛 아이콘이 계속 보여 원인을 추가 추적했다. 시작 메뉴 `Ultra_STT.lnk`의 대상 EXE는 맞았지만 `IconLocation`이 비어 있었다. `scripts/build-windows.ps1`은 설치 후 이 단일 바로가기의 대상 경로를 검증하고, 기존 `.lnk`를 설치 백업에 보존한 뒤 아이콘을 설치 EXE의 resource 0으로 지정하고 그 바로가기에만 `SHChangeNotify(SHCNE_UPDATEITEM)`를 보낸다.

작업표시줄 아이콘 검증은 실행 중인 창의 `AppUserModelID`, `RelaunchIconResource`와 실제 작업표시줄 표시를 함께 확인해야 한다. 실행 파일에 아이콘이 포함됐거나 창의 `ICON_BIG`이 맞다는 사실만으로는 고정된 작업표시줄 아이콘까지 갱신됐다고 볼 수 없다.

64px tray 아이콘은 검정 배경에 전파가 없는 더 큰 마이크다. 확인한 녹색 픽셀 경계는 x=12..51, y=3..59이며 SHA256은 `e7f5e88f7179e274f0f2244fea282e60c868f585c110604d83d3f972124950c2`다. 최신 Mac 앱도 stable identity `com.ultrastt.personal`로 서명·설치되고 실행 중이며 원본 사용자 설정과 기록을 보존한다.

초기 트레이 QA는 외부 `ShowWindow(0)`로 창을 숨겼다. 이 호출은 Tao의 내부 visible 상태를 갱신하지 않아 다음 `show()`가 무시될 수 있으므로 해당 실패 로그를 제품 실패나 통과 증거로 사용하지 않는다. 실제 tray 클릭은 이번 세션에서 독립적으로 구동·관찰하지 않았다. 앱의 `Tray window toggle requested target_visible=...` 로그는 요청 수락이며 실제 화면 표시 성공 자체를 뜻하지 않는다.
