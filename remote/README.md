# Ultra_STT 원격 입력 사용법

서버 PC에만 Whisper 모델과 Python 환경을 둡니다. 다른 Windows PC, Mac,
Android, iPhone에는 모델이나 Codex 로그인을 복사하지 않습니다.

1. 서버 PC에서 기존 `opentypeless.exe`를 트레이 메뉴로 종료하고 최신
   `Ultra_STT.exe`를 실행합니다. 서버에는 별도의 Codex 앱 프로필과 로컬
   Whisper 런타임이 준비되어 있어야 합니다.
2. 서버 PC와 사용할 기기에서 **같은 소유자 계정의 Tailscale**을 연결합니다.
   서버 PC에서 `scripts/enable-remote.ps1`을 실행하면 Tailnet 내부 전용
   HTTPS 주소가 출력됩니다. 이 스크립트는 기존 Serve 포트가 아닌
   `10443`만 설정하며 인터넷 공개용 Funnel은 사용하지 않습니다.
3. 다른 기기의 브라우저에서 출력된 주소를 열고 마이크를 허용합니다.
   **녹음 시작 → 완료하고 정리 → 문장 복사/공유** 순서로 사용합니다.
   취소 버튼은 이번 요청을 버리고 서버의 진행 중인 작업을 끊습니다.

현재 원격 화면은 브라우저에서 복사·공유하는 방식입니다. 다른 앱의 입력칸에
직접 타이핑하는 Windows/Mac 키보드나 iPhone 시스템 키보드로 설치되는 것은
아닙니다. iPhone 서드파티 키보드는 마이크 접근이 허용되지 않습니다.
서버 PC가 꺼지거나 Tailnet 연결이 끊기면 원격 입력도 사용할 수 없습니다.

## Mac 데스크톱의 다듬기 모드

Mac의 `ultra-remote` 프리셋은 선택한 모드와 AI 활성 여부를 서버로 전달합니다.
녹음 중 선택한 모드는 녹음 종료 시 확정하며, 처리 시작 후 변경은 다음 녹음에 적용됩니다.
Mac에서 취소하여 HTTP 연결을 끊으면 서버도 진행 중인 처리를 중단합니다.

기존 Bearer 인증과 JSON 응답은 유지합니다. 선택 헤더는
`x-ultra-stt-polish-style` (`original`, `minimal`, `clean`, `structured`, `professional`)과
`x-ultra-stt-polish-enabled` (`true`, `false`)입니다. 헤더가 없는 기존 클라이언트는
종전처럼 `clean` + AI 활성 상태로 처리됩니다. 잘못된 값이나 중복 헤더는 HTTP 400으로 거절합니다.
이 헤더는 개인 서버 프리셋에서만 전송하며 다른 STT 제공자에는 추가하지 않습니다.
