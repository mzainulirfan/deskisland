import type { PanelTab } from '$lib/stores/island.svelte';

/**
 * Box height per tab. The window shrinks to the content so the trimmed strip below the
 * panel never traps desktop clicks, and the rail layout uses the same numbers to size its
 * pane instead of stretching every page to the height of the menu.
 * Raise a value here if a tab feels cramped — nothing needs to change elsewhere.
 */
export const PANEL_HEIGHTS: Record<PanelTab, number> = {
  system: 208,
  // Playback and volume sit side by side in one card, so the shell only needs the
  // height of the album-art tile plus its padding.
  media: 176,
  // Hero circular dial + controls + interactive Pomodoro pipeline & preset hub.
  timer: 268,
  // One row per alarm plus the editor; the editor column (text, time, two chip rows,
  // presets, save, hint) is the tall part, so the pane is sized for it.
  reminders: 296,
  processes: 220,
  downloads: 220,
  apps: 260,
  quick: 220,
};

export const PANEL_MIN_HEIGHT = 150;

/** What a box height spends on chrome around the pane: the 48 px header row, the gap under
 * it (12 px horizontal, 6 px in the rail) and the section's `pb-3`. Both layouts reserve
 * the same chrome, so a tab's pane is the same height whichever menu it is reached from. */
export const PANE_CHROME = 72;

/** Height of the pane itself for one tab. */
export function paneHeight(tab: PanelTab): number {
  return (PANEL_HEIGHTS[tab] ?? 0) - PANE_CHROME;
}
