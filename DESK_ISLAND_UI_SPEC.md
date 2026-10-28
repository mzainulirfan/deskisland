# Desk Island — UI Design Specification for AI Agents

> **Purpose:** This document is the source of truth for designing and implementing the Desk Island interface for Windows 10/11.
> **Stack:** Svelte 5 + TypeScript + Tauri 2 + Rust + Tailwind CSS.
> **Visual reference:** Use the previously supplied Desk Island concept image in this conversation as the primary visual target. Recreate its layout and visual language, not the exact pixels or any unrelated Windows UI.

## 1. Product vision

Desk Island is a compact, floating desktop companion inspired by the interaction pattern of a mobile Dynamic Island. It lives near the top-center of the active monitor and expands into contextual cards when the user interacts with it or an activity needs attention.

The design must feel:
- Dark, polished, modern, and lightweight.
- Compact in idle state; informative when expanded.
- Smooth and responsive, without distracting desktop users.
- Consistent across all states, with a black pill, subtle border, soft shadow, and carefully controlled accent colors.
- Native to a desktop utility, but not a clone of any specific commercial product.

## 2. Design principles

1. **Idle first:** default state occupies minimal screen space.
2. **Contextual expansion:** expand only the relevant area for the active event.
3. **One visual system:** use the same radii, borders, typography, spacing, and motion across every panel.
4. **Readable at a glance:** important values and titles must be legible at normal desktop viewing distance.
5. **No fake functionality:** controls must work or be clearly presented as disabled/unavailable.
6. **Desktop-safe:** don't obscure app title bars, menus, taskbar content, or important work areas.
7. **Motion with purpose:** animations communicate state changes; they must not be ornamental or continuous without reason.
8. **Accessible:** support keyboard interaction, visible focus, adequate contrast, reduced motion, and meaningful labels.

## 3. Overall visual direction

### Canvas and placement
- Transparent Tauri window; no standard OS title bar.
- Main island horizontally centered near the top edge of the selected monitor.
- Recommended top offset: 12–20 px, configurable.
- Idle pill target size: approximately 300–340 px wide and 54–64 px high for the full status layout. A compact variant may be 160–220 px wide.
- Expanded island: width adapts to content, typically 420–620 px for one activity card.
- Multi-card/system panel: 700–960 px wide, capped at around 90% of available screen width.
- Never assume the CSS viewport starts at the physical screen's top edge; position the native Tauri window using a supported window-position API.
- Respect display scaling, multiple monitors, and monitor work areas.

### Shape and surface
- Main pill: near-black surface (`#050608` to `#0B0D12`).
- Large radius: fully rounded for idle pill; 24–32 px for expanded panels.
- 1 px low-contrast cool border (`rgba(255,255,255,0.10)`).
- Soft, broad shadow; avoid a harsh glow around the whole window.
- Optional backdrop blur only if it works reliably in the target Windows/Tauri configuration. A solid dark surface must remain attractive without blur.
- Transparent outside the UI; don't render a full-screen opaque background.

### Color tokens

Use these tokens as a starting point. Keep the overall interface mostly neutral; accent colors indicate information categories.

| Token | Value | Use |
|---|---|---|
| `--di-bg` | `#07090D` | Main surface |
| `--di-surface` | `#10141B` | Cards and raised controls |
| `--di-surface-hover` | `#171D27` | Hover state |
| `--di-border` | `rgba(255,255,255,.10)` | Borders |
| `--di-text` | `#F5F7FA` | Primary text |
| `--di-muted` | `#9AA4B2` | Secondary text |
| `--di-subtle` | `#667085` | Tertiary labels |
| `--di-blue` | `#22B8FF` | Network/download |
| `--di-purple` | `#A855F7` | Media/RAM |
| `--di-green` | `#34D399` | Healthy/connected/battery |
| `--di-amber` | `#F59E0B` | Storage/caution |
| `--di-red` | `#F43F5E` | High usage/error/urgent notification |

Do not use all accent colors in every component. Most surfaces remain black/charcoal.

### Typography
- Use the system UI font stack or a bundled font with reliable licensing.
- Primary labels: 13–15 px.
- Secondary labels: 11–13 px.
- Important metrics: 22–28 px.
- Main clock: 28–36 px in the expanded system panel; 14–16 px in the idle pill.
- Use tabular numerals for clocks, percentages, and transfer rates.
- Avoid oversized headings and unnecessary uppercase labels.

### Spacing and radii
- Base spacing scale: 4, 8, 12, 16, 20, 24, 32 px.
- Compact card padding: 14–16 px.
- Expanded panel padding: 20–24 px.
- Inner card radius: 16–20 px.
- Controls: 40–44 px minimum hit target where practical.
- Use consistent gaps; avoid crowded icon/text pairs.

## 4. Component and state model

Implement separate reusable Svelte components. Keep state and system data separate from presentation.

Suggested components:
- `Island.svelte` — shell, dimensions, transitions, and state routing.
- `IdleView.svelte` — compact status pill.
- `MusicView.svelte` — active media card.
- `NotificationView.svelte` — notification preview.
- `DownloadView.svelte` — download progress.
- `SystemPanel.svelte` — clock, status chips, and metric cards.
- `VolumePanel.svelte` — master and per-app volume where supported.
- `ProcessPanel.svelte` — active process list.
- `QuickSettingsPanel.svelte` — shortcuts to supported Windows settings/actions.
- `BatteryIndicator.svelte`, `NetworkIndicator.svelte`, `MetricCard.svelte`, `IconButton.svelte` — shared components.

Suggested UI states:
- `idle`
- `idle-activity` (small media/download/notification indicator)
- `music-expanded`
- `notification-expanded`
- `download-expanded`
- `system-expanded`
- `volume-expanded`
- `processes-expanded`
- `quick-settings-expanded`

Represent state with a discriminated TypeScript union rather than a collection of unrelated booleans, so incompatible panels cannot be open simultaneously.

Example conceptual type:

```ts
type IslandView =
  | { kind: 'idle' }
  | { kind: 'activity'; activity: 'music' | 'download' | 'notification' }
  | { kind: 'music' }
  | { kind: 'notification'; notificationId: string }
  | { kind: 'download'; downloadId?: string }
  | { kind: 'system' }
  | { kind: 'volume' }
  | { kind: 'processes' }
  | { kind: 'quick-settings' };
```

## 5. Required UI states

### A. Idle pill — default

**Purpose:** show essential status without demanding attention.

Layout, left to right:
1. Small Windows/Desk Island mark (simple four-pane mark or custom geometric glyph).
2. Current local time.
3. Small monitor/display status icon if available.
4. Wi-Fi/network status.
5. Battery icon and percentage if the device exposes battery data.

Rules:
- Use a horizontal black capsule with subtle border.
- Keep icon sizes consistent (16–18 px).
- Use a 6–8 px gap between status groups, with slightly larger spacing around the clock.
- Do not show labels like “CPU” or “RAM” in idle state.
- If a data source is unavailable, hide that indicator or show a neutral unavailable state; never invent a value.
- Clicking/tapping the pill opens the system panel.
- Hover may reveal a subtle surface change, not a bright glow.

### B. Idle with activity

**Purpose:** hint that an activity is happening without opening a large card.

Examples:
- Music: small album thumbnail plus a minimal equalizer or waveform.
- Download: download glyph with a small progress ring/bar.
- Notification: bell glyph and a small unread dot.
- Recording: recording dot and elapsed time, only if a recording feature exists.

Rules:
- Keep the same pill height as idle where possible.
- Limit to one primary activity indicator; prioritize urgent notifications, then active media/download based on product settings.
- Never use a large bouncing animation. A subtle equalizer animation is allowed while media is playing.

### C. Music expanded

**Purpose:** provide glanceable playback controls.

Layout:
- Album art thumbnail on the left (72–96 px square).
- Track title and artist in the middle.
- Previous, play/pause, and next buttons on the right if the active media session supports them.
- Thin progress track at the bottom with elapsed and total time when available.
- Main card width around 480–560 px; height around 140–160 px.

Rules:
- Truncate long titles with ellipsis; expose the full title in an accessible label/tooltip.
- Controls must reflect actual session capability.
- Use purple as a restrained progress/accent color.
- If artwork or metadata is missing, use a neutral placeholder rather than broken image UI.
- Don't invent playback position or duration if Windows doesn't expose it.

### D. Notification expanded

**Purpose:** preview a single incoming notification.

Layout:
- App icon on the left (40–52 px).
- App/source name, optional timestamp, title, and a short message preview.
- Dismiss action only when supported and safe.
- Main card width around 400–480 px; height around 120–150 px.

Rules:
- Respect privacy settings: allow message previews to be hidden.
- Don't expose sensitive notification content in a persistent display by default.
- If no notification integration is available, show notifications generated by Desk Island itself or a clearly labeled demo state.
- Avoid copying app logos without need; use the app's available icon where permitted.

### E. Download expanded

**Purpose:** display a known download's status and progress.

Layout:
- File-type icon or file icon on the left.
- File name and state text in the middle.
- Progress bar and downloaded/total size below.
- Pause/resume/cancel controls only if the integration actually controls the download source.
- Main card width around 440–540 px; height around 120–150 px.

Rules:
- File-system monitoring can identify file changes but does not automatically reveal a browser's true progress, pause, or cancel capability.
- If progress is inferred from file size changes, label it as an estimate and do not invent a total size.
- For unsupported download sources, show “Activity detected” rather than false precision.

### F. System panel expanded

**Purpose:** show system health and quick status.

Header:
- Desk Island mark.
- Large digital clock.
- Date in the user's locale.
- Right-side status chips: network, Bluetooth if available, battery/charging, settings button.

Metric cards in a row or responsive grid:
1. CPU usage percentage with a tiny sparkline.
2. RAM usage percentage with a tiny sparkline.
3. Network download/upload rates.
4. Disk usage percentage, only when disk capacity data is implemented.

Dimensions:
- Typical width 760–920 px.
- Height 220–280 px.
- Inner metric cards have dark raised surfaces and subtle borders.
- Accent: red/pink for CPU, purple for RAM, cyan/blue for network, amber for storage.
- Sparkline must reflect real sampled values; don't show decorative fake data in production.
- If screen width is limited, wrap into a two-column grid rather than overflowing.

### G. Volume panel

**Purpose:** adjust system volume and optionally per-app volume.

Layout:
- Master volume slider at the top with speaker icon and numeric percentage.
- Optional list of app sessions with icon/name and individual sliders.
- Muted state must be visible and reversible.
- Use native Windows Core Audio integration; don't render sliders that do nothing.

### H. Processes panel

**Purpose:** inspect active processes.

Layout:
- Header with “Proses Aktif” and a “Lihat semua” action.
- Table columns: process/app name, CPU, memory.
- Show a short list (5–8 rows) with app icons where available.
- Use stable row heights and tabular numerals.
- Don't add “End task” unless explicitly implemented with permission/safety checks.

### I. Quick settings panel

**Purpose:** provide shortcuts to relevant Windows controls.

Use a compact grid of large square/rounded tiles:
- Wi-Fi settings
- Bluetooth settings
- Focus/Do Not Disturb settings
- Screenshot tool
- Display/projection settings
- Battery settings

Important:
- A shortcut that opens Windows Settings is acceptable when direct toggling isn't supported.
- Do not show a toggle as active unless the actual system state is known.
- Clearly differentiate “open settings” actions from real toggles.
- Avoid privileged or undocumented system modifications.

## 6. Interaction and motion

### Main interactions
- Click idle pill → open system panel.
- Click activity indicator → open the matching activity panel.
- Click outside → close expanded panel, unless keyboard focus or a modal interaction requires it to remain open.
- Escape → close expanded panel.
- Clicking the close/dismiss button → close only the relevant card/notification.
- Buttons and sliders must have clear hover, active, disabled, and focus-visible states.

### Motion guidance
- Expand/collapse duration: 200–280 ms.
- Use a smooth ease-out curve; avoid overshoot or elastic bounce.
- Animate opacity and size/transform where feasible.
- Respect `prefers-reduced-motion`.
- Avoid animating CPU/network values themselves; update the text smoothly without visual jitter.
- Debounce rapid activity changes to prevent the pill from repeatedly opening and closing.

### Auto-collapse
- Activity previews may collapse after a configurable short timeout (for example, 4–7 seconds).
- Never auto-collapse while the user is interacting with controls.
- System/volume/process panels should remain open until the user closes them or clicks outside.

### Window behavior
- Native Tauri window should be transparent, undecorated, and positioned at the top-center of the selected monitor.
- Window dimensions should adapt to the current UI state. CSS expansion alone cannot extend a native window beyond its existing bounds.
- Implement safe window resizing/repositioning through Tauri APIs or a suitable maintained plugin.
- Ensure the transparent window does not intercept clicks across a large invisible rectangle in idle state. Prefer resizing the native window to tightly fit the pill or using a tested click-through strategy.
- Keep focus behavior deliberate: don't steal keyboard focus for routine notifications.
- Test multi-monitor setups and Windows display scaling.
- Persist user settings such as monitor, top offset, theme, privacy, and activity priority.

## 7. Responsive behavior

- On narrow work areas, use 90% of available width with sensible side margins.
- System panel changes from four metric columns to two columns when necessary.
- Notification and download cards may stack icon/title/details vertically on small widths.
- Prevent any panel from extending below the visible work area.
- Support Windows scaling at 100%, 125%, 150%, and 200%.

## 8. Accessibility and privacy

- Keyboard-accessible interactions and visible focus states.
- Use semantic buttons for actions; avoid clickable `div` elements when possible.
- Add accessible labels to icon-only buttons.
- Maintain readable contrast and don't rely on color alone to communicate status.
- Respect Windows notification privacy settings and provide a “hide previews” option.
- Avoid logging notification message bodies or personal content.
- Request only permissions required by the enabled features.
- Provide a setting to disable activity popups and animations.

## 9. Implementation guidance for the AI coding agent

When implementing the UI:

1. Inspect the existing repository and current package versions before editing.
2. Preserve existing project conventions; don't recreate or replace a working project without need.
3. Implement the visual shell and all states with mock data first, but mark mock data clearly in development.
4. Build reusable Svelte components and shared design tokens; avoid a single giant component.
5. Keep Windows integration in Rust modules and expose narrow typed Tauri commands/events.
6. Use a discriminated union for the active island view.
7. Keep real data separate from display components.
8. Don't add dependencies until verifying compatibility with the installed Tauri/Svelte/Tailwind versions.
9. Don't claim features work until their Windows integration has been implemented and tested.
10. Verify `npm run check` (or the template's equivalent), `npm run build`, and `npm run tauri dev`.
11. Test idle, activity, expanded, keyboard, reduced-motion, and small-screen states.
12. Keep UI code formatted and free of avoidable TypeScript errors.

### Suggested implementation sequence
1. Design tokens and app shell.
2. Idle pill and expanded/collapsed transition.
3. System panel with clearly marked mock data.
4. Rust CPU/RAM command and polling lifecycle.
5. Media session integration.
6. Network and battery metrics.
7. Volume integration.
8. Process list.
9. Download tracking.
10. Notifications and quick settings.
11. Tray, preferences, startup behavior, and installer.

## 10. Acceptance checklist

The UI is ready for review when:
- [ ] Idle pill resembles the reference: centered, black, compact, subtly bordered, with balanced status icons.
- [ ] Expanded music card contains art, title, artist, controls, and progress in a clear hierarchy.
- [ ] Notification card is readable and respects privacy.
- [ ] Download card distinguishes actual progress from inferred activity.
- [ ] System panel shows clock/date/status chips and four metric cards.
- [ ] Volume, process, and quick-settings panels use the same visual language.
- [ ] Expansion/collapse is smooth and does not bounce.
- [ ] Transparent window has no visible rectangular background.
- [ ] Native window resizing keeps every state visible and click targets accurate.
- [ ] Buttons, sliders, and settings perform real actions or are clearly marked unavailable.
- [ ] No fabricated system values are shown in production.
- [ ] Keyboard navigation, Escape, reduced motion, and display scaling are tested.

## 11. Final instruction

Treat this file as the authoritative UI design brief. First reproduce the visual hierarchy and states using Svelte components and clearly labeled mock data. Then integrate Windows functionality one module at a time. Match the reference's black pill, restrained neon accents, rounded dark cards, subtle borders, and spacious layout, while ensuring that every visible control accurately reflects implemented capability.
