<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" width="96" height="96" alt="Ultra STT emerald microphone" />
</p>

<h1 align="center">Ultra STT</h1>
<p align="center"><strong>Your voice. Your words. Your cursor.</strong></p>
<p align="center">
  A personal voice keyboard for Windows and macOS.<br />
  Local Whisper recognition, optional AI cleanup, and direct text insertion.
</p>
<p align="center">
  <strong>v1.1.62</strong> · Windows x64 · macOS Apple Silicon · MIT
</p>
<p align="center">
  English · <a href="README_ko.md">한국어</a> ·
  <a href="docs/FEATURES.md">Feature &amp; technical specification</a> ·
  <a href="docs/VALIDATION.md">Validation &amp; limitations</a>
</p>

<p align="center">
  <img src="docs/images/ultra-stt-dashboard.png" width="860" alt="Ultra STT 1.1.62 on Windows: five illustrated cleanup modes and interactive 14-day history" />
</p>
<p align="center"><em>Actual installed Windows app, captured October 2, 2026. Minimal is selected.</em></p>

> **Project status:** This is the user-owned repository at [toughCSB/ultra-stt](https://github.com/toughCSB/ultra-stt), currently private. Version 1.1.62 has been built and installed on Windows x64 and an Apple Silicon Mac. No downloadable GitHub release is published at this snapshot. Linux code is inherited, but this personal version has not been validated on Linux. Mobile currently means a browser recorder, not an Android/iOS keyboard app.

## Why Ultra STT?

Speak without leaving your text editor, choose how much rewriting you want, and keep a searchable local record.

- **Voice to cursor:** global shortcuts, hold/toggle capture, keyboard insertion or clipboard output.
- **Five cleanup styles:** Original, Minimal, Clean, Structured, and Professional.
- **Three quick-switch surfaces:** tray menu, illustrated Home cards, and the floating capsule.
- **Useful history:** dated activity counts, click-to-filter, raw/final text search, copy, and correction creation.
- **Quality-conscious performance:** a warm CUDA recognizer, reusable HTTP clients, Fast Codex requests, and task-sensitive reasoning.
- **Separate AI profile:** app-only instructions and ChatGPT login, independent of development Codex settings.
- **Mac + Windows server:** record on Mac, recognize and clean on the Windows host, then insert on Mac—without a second cleanup call.

## A simple daily workflow

1. Configure a speech provider, input microphone, and output strategy in **Settings**.
2. Enable AI cleanup if wanted, then select a style on **Home** or in the tray menu.
3. Put the cursor in the destination text field.
4. Use your configured dictation shortcut; speak, then release the key or toggle again.
5. The capsule shows capture and processing states. Press **Esc** or its **Esc button** to cancel.
6. Open **History** to find, copy, or compare the result.

Shortcuts are customizable; existing installations retain their own bindings. Source defaults are:

| Workflow | Windows | macOS |
| --- | --- | --- |
| Dictation | `Ctrl+/` | `Fn` |
| Ask Anything | `Ctrl+.` | `Fn+Space` |
| Translation | `Ctrl+Shift+/` | `Fn+LeftShift` |
| Cancel active capture/processing | `Esc` | `Esc` |

Windows defaults to hold-to-talk; macOS defaults to toggle. Each workflow can have up to three bindings. The screenshot shows configured shortcuts, not a mandatory setup.

## Choose how much AI changes

| Style | Behavior | Useful for |
| --- | --- | --- |
| **Original** | Correct clear recognition/spelling/spacing errors and meaningless fillers; preserve wording, order, tone, emphasis, and layout. No automatic outlines or paragraph restructuring. | Faithful dictation |
| **Minimal** | Preserve the speaker's words; add punctuation and natural sentence breaks, remove obvious fillers. | Everyday fast dictation |
| **Clean** | Lightly improve phrasing and word order without changing facts or information density. | Messages and notes |
| **Structured** | Organize multiple distinct items into an outline, with headings when useful; do not invent facts. | Tasks and meeting points |
| **Professional** | Concise work communication without empty pleasantries or unnecessary expansion. | Emails and reports |

**Original is still AI cleanup**, not raw STT. Disable AI cleanup to receive the recognizer's text without that step. Explicit editing, translation, scenes, and custom instructions have their own operation rules.

Click a Home card, choose **AI cleanup mode** in the tray context menu, or open the capsule's mode picker. The current choice has a checked state and green highlight; selection persists and synchronizes between surfaces. A change during recording applies when that recording stops. Once processing has started, a new choice applies to the next recording.

## Screenshots

| ChatGPT subscription cleanup | Shortcut and output controls |
| --- | --- |
| <img src="docs/images/ultra-stt-ai-settings.png" width="440" alt="Actual Ultra STT AI settings: Codex CLI, gpt-6-luna and Minimal style" /> | <img src="docs/images/ultra-stt-shortcuts.png" width="440" alt="Actual Ultra STT shortcut settings, toggle capture and keyboard output" /> |

These are real Windows captures, cropped to the app content—not generated UI mockups. No API keys or transcription bodies are shown. The illustrated mode assets are AI-generated; their provenance is documented [here](docs/polish-mode-assets.md).

## Features

| Area | Implemented capabilities |
| --- | --- |
| Capture | Selectable microphone, global hotkeys, hold/toggle mode, floating capsule, idle auto-hide, cancellation |
| Output | Keyboard simulation, automatic clipboard paste, copy-only option, target-app guard, fallback warnings |
| AI | Five cleanup styles, custom instructions, application-aware writing, per-app mappings |
| Editing | Spoken instructions for selected text: rewrite, fix, shorten, summarize, or translate |
| Ask Anything | One-shot voice question, floating answer note, copy support; not a persistent chat |
| Translation | Dedicated shortcut and configurable target languages |
| Dictionary | Local terms, import/export, enabled correction rules, create a correction from history |
| Scenes | Built-in/custom writing scenes and import/export |
| History | SQLite storage, raw/final text search, date filtering, copy, clear with confirmation, insertion-status diagnostics |
| Home | Today count, interactive 14-day activity, current provider/output settings, last processing timing |
| Desktop | Dark/light/system themes, auto-start, tray show/hide toggle, branded microphone taskbar/tray icons |
| Remote | Private-tailnet browser recorder and Mac desktop `ultra-remote` preset |
| Localization | Ten UI locales; some advanced copy still falls back to English |

Advanced workflows depend on the selected providers. The personal Mac remote path forwards cleanup style and AI enablement, **not** all client-side scene, dictionary, selected-text, or translation settings. See [scope and limits](docs/FEATURES.md).

### What the history bars mean

Each bar is a **day's loaded transcription count**, not microphone volume or decorative animation. Dates and counts are visible, including empty days. Clicking a bar opens History filtered to that date.

The chart uses the latest loaded records, **up to 200**, rather than pretending to count the complete database. The processing card shows the last operation in the current session, excluding speaking time. On the Mac remote path, STT and server AI share a combined remote-processing measurement.

## Technical snapshot

| Layer | Current implementation |
| --- | --- |
| Desktop shell | Tauri 2.11.2, Rust 2021; Rust minimum 1.87 |
| UI | React 19, TypeScript 5.8, Vite 8, Tailwind CSS 4, Zustand 5, i18next |
| Audio / output | `cpal` capture, Rust output adapters, Windows SendInput / macOS native paste |
| Persistence | SQLite history/dictionary; local app configuration and credential-vault integration |
| Personal recognition | `faster-whisper 1.2.1` + `CTranslate2 4.8.1`, Whisper large-v3, CUDA float16, beam size 5, VAD |
| Personal cleanup | `codex-cli`, ChatGPT subscription authentication, `gpt-6-luna` |
| Codex service tier | Every request made through this adapter asks for `service_tier="priority"` (Fast) |
| Reasoning | Medium for simple short dictation; High for complex/terminology-sensitive operations |
| Private remote transport | Tailscale Serve HTTPS, same-owner checks, existing Bearer authentication |
| Validated desktop targets | Windows x64 and macOS arm64 |
| Mobile status | Browser microphone → result → copy/share; no native mobile keyboard yet |

No minimum VRAM/RAM benchmark is claimed. The personal GPU route needs a compatible NVIDIA CUDA environment and separately downloaded model files. Other STT integrations include Apple Speech, Whisper-compatible endpoints, Deepgram, AssemblyAI, Aliyun Qwen3, and Volcengine; availability and credentials are provider-dependent, not all personally acceptance-tested.

## Performance without quality shortcuts

The current path preserves the large-v3 recognizer and subscription model:

- Warm the local recognizer at startup and retain its model after successful inference.
- Reuse HTTP clients and skip a second local AI pass for server-cleaned Mac results.
- Use an isolated `CODEX_HOME` and short, app-specific instructions instead of development instructions.
- Request Fast for all Codex operations. Use Medium only for ordinary dictation of at most 1,024 characters with Original/Minimal/Clean and no selected text, translation, scenes, custom prompt, dictionary, or enabled corrections; otherwise retain High.
- Abort owned processing on cancellation; a disconnected remote client cancels its server work.

Cloud AI **still takes seconds**. Fast is a requested service tier, not a guaranteed response time, and provider usage limits still apply. Input-token savings do not translate directly into the same subscription-usage savings. No Typeless-equivalent speed claim is made. [Measured results and methodology](docs/VALIDATION.md).

## Build from source

Clone the user-owned repository using an account that can access this private repo:

```sh
git clone https://github.com/toughCSB/ultra-stt.git
cd ultra-stt
npm ci
npm run tauri dev
```

Requirements: Node `20.19+` on the 20.x line or `22.12+`, Rust 1.87+ stable, and the platform toolchain (Windows Visual Studio C++ Build Tools/WebView2; macOS Xcode Command Line Tools). The personal Python recognizer requires Python 3.11 and the dependencies declared in `local_stt/pyproject.toml`.

### Standard desktop bundle

```sh
npm run tauri build
```

Artifacts are written under `src-tauri/target/release/bundle/`. Speech models, credentials, Python environments, and personal settings are **not bundled in the repository**.

### Prepared personal installations

The Windows helper uses a prepared parent-directory runtime/cache. It is not a one-command model installer for a fresh clone:

```powershell
powershell -NoProfile -File scripts/build-windows.ps1 -Install
```

On macOS:

```sh
sh scripts/build-macos.sh --install
```

Both helpers back up the previous app before replacement. The Mac helper verifies a stable local ad-hoc signature with identifier `com.ultrastt.personal`; this is **not Apple notarization**. Grant Microphone and Accessibility permissions for capture and cursor insertion.

For the Windows app-only Codex profile, use the existing official CLI and log in separately:

```powershell
pwsh -NoProfile -File scripts/configure-codex-profile.ps1 -Login
pwsh -NoProfile -File scripts/configure-codex-profile.ps1 -Profile isolated
```

Do not copy authentication files from the development profile. The current Mac remote setup uses the Windows host's profile and needs no second Codex login. Detailed guides: [Windows](docs/windows-personal.md), [macOS](docs/macos-personal.md), [Codex isolation](docs/codex-isolation-personal.md), [remote setup](remote/README.md).

## Privacy and boundaries

- Local recognition runs on the selected local host. The Mac remote setup sends audio to the Windows host across the private tailnet.
- Enabling Codex cleanup sends relevant text and writing instructions to the configured ChatGPT service. Selected text is included only in workflows that explicitly use it.
- App classification happens locally. Raw window titles are not sent as app-context metadata.
- Credentials use available OS secure storage; existing vault-failure fallback behavior is documented in the platform guides. Do not assume every setting file is secret-free.
- Models, authentication files, databases, personal runtime directories, private logs, and `.env` files must stay out of Git.
- This repository does not provide an Ultra STT cloud subscription or included provider quota. Upstream cloud integrations are separate from the personal deployment.
- Remote setup uses Tailscale Serve, **not public Funnel**. Keep the server awake and connected.

## Current validation & next steps

The 1.1.62 snapshot has **414 frontend tests**, **629 Windows native tests**, and **635 Mac native tests** passing; three live/provider-dependent native tests are excluded from each default run. Production builds and personal installs completed on both validated desktop targets. The Mac-to-Windows smoke test exercised all five modes, AI disabled, legacy requests, invalid headers, missing authentication, and client cancellation.

Latest physical Mac microphone → cursor → Esc acceptance remains a separate manual check, not something unit tests prove. Native Android/iOS input integration, permissions, and device testing remain future work—**not just packaging**. [Validation details](docs/VALIDATION.md).

## Documentation & credits

- [Feature and technical specification](docs/FEATURES.md)
- [Validation snapshot, measurements, and limitations](docs/VALIDATION.md)
- [Personal performance investigation](docs/performance-personal.md)
- [Brand/icon notes](docs/brand-personal.md) · [Generated mode artwork](docs/polish-mode-assets.md)
- [Contributing](CONTRIBUTING.md) · [Security](SECURITY.md) · [Issues](https://github.com/toughCSB/ultra-stt/issues)

Ultra STT builds on [OpenTypeless](https://github.com/tover0314-w/opentypeless). Upstream contributors' attribution and the [MIT license](LICENSE) are retained. Other inherited translated READMEs and older design documents may describe upstream behavior rather than this personal snapshot; this README and its Korean edition describe Ultra STT 1.1.62.
