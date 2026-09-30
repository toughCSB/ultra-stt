# Ultra STT — Voice Keyboard visual system

## Direction
Terminal green. The app sits on a near-black canvas with a faint green cell
grid, one neon green accent (#00e676), Pretendard everywhere, and generated
artwork for the brand and for the voice-to-text idea. No soft SaaS pastels,
no 3D gel, no purple/blue AI gradient.

## Canvas
--color-bg-primary #050806 with an inline 256px SVG tile of 29px cells
(#070d09 base, #0a1410 dim, #0f2417 bright) at background-attachment fixed.

## Surfaces
Panels are rgba(13,21,16,0.72) with backdrop blur, a 1px rgba(255,255,255,0.07)
border, 16px radius, and a 2px green gradient line across the top edge.

## Type
Pretendard variable (jsDelivr) for everything, weight 400 for body and 900
(Black) for the wordmark, display headings and sidebar items. JetBrains Mono
stays as --font-mono for the terminal figure.

## App icon
src/assets/brand/app-icon.svg is the master: a 512 viewBox rounded square on a
green gradient (#178a60 to #063a26, deliberately green rather than near-black so
it reads on a dark taskbar), a mint gradient microphone with its holder, stem and
base, and four concentric wave arcs. Both gradients use gradientUnits=
userSpaceOnUse: with the default objectBoundingBox mode a plain line has a
zero-width or zero-height box, the gradient cannot be resolved, and the stem and
base silently fail to paint. That is what clipped the microphone in the first
version.

It is rasterised once and expanded with `npm run tauri icon`, producing
icon.ico, icon.icns, the PNG ladder, the Windows Square logos and the Android
mipmaps. Window, taskbar entry and installer all use it.

The tray uses a separate glyph. src-tauri/icons/tray-icon.svg is a bright green
microphone with no background, drawn to fill a 64x64 box. Tauri in this version
cannot take a PNG directly, so icons/tray-icon.rgba holds the raw 64x64 RGBA
bytes and lib.rs rebuilds it with Image::new_owned. The square app icon is
unreadable in a 16-32px tray slot.

## Generated artwork
All from GPT images 2.5 sunburst, in src/assets/brand:
- voice-keyboard-hero.png and voice-keyboard-flow.png remain the supporting
  illustrations. The earlier bar emblem and the title banner were retired when
  the microphone icon became the primary mark.
- The sidebar wordmark and the onboarding welcome now use the app icon itself.
- voice-keyboard-hero.png — microphone emitting sound waves that resolve into
  glowing text lines with a cursor, cropped to 1669x789. Sits under the home
  title and on the remote companion page.
- voice-keyboard-flow.png — square mic → waveform → text-lines flow. Used on
  the onboarding welcome.

## Brand
The displayed name is "Ultra STT" with the subtitle "Voice Keyboard" in all
ten locales and in the three window titles. productName and mainBinaryName
remain Ultra_STT so the executable and installer file names keep matching the
launcher script and every path reference.

## Components
- term-eyebrow, term-display, term-figure, term-chip / term-chip-muted,
  term-dot, term-rule.
- Home: terminal panel opening with a flex row of the 88px app icon and the
  eyebrow, the 30px Ultra STT title, the Voice Keyboard subtitle and the
  description, then the voice-keyboard hero image, then the capability chips;
  below it the readout panel with the day count as a term-figure, a LIVE dot
  and a 24-bar activity strip.
- Onboarding welcome: the 88px app icon, "Ultra STT" in Black, "Voice Keyboard"
  in tracked accent capitals, the flow illustration, then capability chips.
- Sidebar: 236px wide, compact wordmark lockup plus a green-dot "local engine" row.

## Evidence
qa/remote-hero.png and qa/remote-terminal.png are headless Chrome captures of
the styled remote page with the artwork in place. The desktop window cannot be
captured in this environment.
