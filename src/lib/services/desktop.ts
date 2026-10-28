import { invoke } from '@tauri-apps/api/core';
import type { IslandPosition } from '$lib/stores/island.svelte';
import type {
  AppShortcut,
  CursorProbe,
  DownloadEvent,
  Downloads,
  Favorite,
  IdleStyle,
  NotificationEvent,
  NowPlaying,
  ProcessInfo,
  Reminder,
  SystemStats,
  TimerInfo,
  Volume,
} from '$lib/types/system';

const inTauri = () => '__TAURI_INTERNALS__' in window;

/// True when running inside the Tauri shell, where hover is delivered by the native
/// cursor monitor. In a plain browser (dev harness) we fall back to polling; `?poll=1`
/// also forces polling so the browser harness can exercise the event handler.
export const usesNativeRuntime = () =>
  inTauri() && !new URLSearchParams(window.location.search).has('poll');

export async function getSystemStats(): Promise<SystemStats | null> {
  if (!inTauri()) return null;
  return invoke<SystemStats>('system_stats');
}

// Volume and GSMTC are Windows-only APIs; a rejected command reads as "no data".
export async function getVolume(): Promise<Volume | null> {
  if (!inTauri()) return null;
  return invoke<Volume>('master_volume').catch(() => null);
}

export async function setVolume(percent: number): Promise<Volume | null> {
  if (!inTauri()) return null;
  return invoke<Volume>('set_master_volume', { percent }).catch(() => null);
}

export async function setMasterMute(muted: boolean): Promise<Volume | null> {
  if (!inTauri()) return null;
  return invoke<Volume>('set_master_mute', { muted }).catch(() => null);
}

export async function getNowPlaying(): Promise<NowPlaying | null> {
  if (!inTauri()) return null;
  return invoke<NowPlaying | null>('now_playing').catch(() => null);
}

export type MediaAction = 'toggle' | 'play' | 'pause' | 'next' | 'previous';

export async function mediaCommand(action: MediaAction): Promise<boolean> {
  if (!inTauri()) return false;
  return invoke<boolean>('media_control', { action }).catch(() => false);
}

export async function getTopProcesses(limit: number): Promise<ProcessInfo[]> {
  if (!inTauri()) return [];
  return invoke<ProcessInfo[]>('top_processes', { limit }).catch(() => []);
}

export async function getDownloads(): Promise<Downloads | null> {
  if (!inTauri()) return null;
  return invoke<Downloads>('downloads_status');
}

export async function openDownload(path: string): Promise<void> {
  if (!inTauri()) return;
  await invoke('open_download', { path }).catch(() => undefined);
}

export async function revealDownload(path: string): Promise<void> {
  if (!inTauri()) return;
  await invoke('reveal_download', { path }).catch(() => undefined);
}

export async function getRecentDownloads(limit: number): Promise<DownloadEvent[]> {
  if (!inTauri()) return [];
  return invoke<DownloadEvent[]>('recent_downloads', { limit }).catch(() => []);
}

export async function openDownloadsFolder(): Promise<void> {
  if (!inTauri()) return;
  await invoke('open_downloads_folder').catch(() => undefined);
}

export async function listAppShortcuts(): Promise<AppShortcut[]> {
  if (!inTauri()) return [];
  return invoke<AppShortcut[]>('list_app_shortcuts').catch(() => []);
}

export async function launchApp(path: string): Promise<void> {
  if (!inTauri()) return;
  await invoke('launch_app', { path }).catch(() => undefined);
}

export async function getAppIcon(path: string): Promise<string | null> {
  if (!inTauri()) return null;
  return invoke<string>('app_icon', { path }).catch(() => null);
}

export async function setFavorites(favorites: Favorite[]): Promise<void> {
  if (!inTauri()) return;
  await invoke('set_favorites', { favorites }).catch(() => undefined);
}

export async function getRecentNotifications(): Promise<NotificationEvent[]> {
  if (!inTauri()) return [];
  return invoke<NotificationEvent[]>('recent_notifications').catch(() => []);
}

// The countdown lives in Rust, so it survives a hidden island; these calls only return
// the deadline and let the webview draw it without polling.
export async function getTimer(): Promise<TimerInfo | null> {
  if (!inTauri()) return null;
  return invoke<TimerInfo>('timer_status').catch(() => null);
}

export async function startTimer(minutes: number, label: string): Promise<TimerInfo | null> {
  if (!inTauri()) return null;
  return invoke<TimerInfo>('start_timer', { minutes, label }).catch(() => null);
}

export async function pauseTimer(): Promise<TimerInfo | null> {
  if (!inTauri()) return null;
  return invoke<TimerInfo>('pause_timer').catch(() => null);
}

export async function resumeTimer(): Promise<TimerInfo | null> {
  if (!inTauri()) return null;
  return invoke<TimerInfo>('resume_timer').catch(() => null);
}

export async function cancelTimer(): Promise<TimerInfo | null> {
  if (!inTauri()) return null;
  return invoke<TimerInfo>('cancel_timer').catch(() => null);
}

export async function addTimerTime(minutes: number): Promise<TimerInfo | null> {
  if (!inTauri()) return null;
  return invoke<TimerInfo>('add_timer_time', { minutes }).catch(() => null);
}

export async function playSystemBeep(): Promise<void> {
  if (!inTauri()) return;
  await invoke('play_system_beep').catch(() => undefined);
}

// The deadline is watched in Rust, so reminders still fire while the island is hidden by
// the tray or covered by a fullscreen app. `saveReminders` hands back the list the backend
// actually kept: it caps the count, repairs bad times, and assigns the id of a new item.
export async function getReminders(): Promise<Reminder[]> {
  if (!inTauri()) return [];
  return invoke<Reminder[]>('get_reminders').catch(() => []);
}

export async function saveReminders(reminders: Reminder[]): Promise<Reminder[]> {
  if (!inTauri()) return reminders;
  return invoke<Reminder[]>('save_reminders', { reminders }).catch(() => reminders);
}

/// Sounds a reminder right now, for the row's test button.
export async function previewReminder(id: number): Promise<boolean> {
  if (!inTauri()) return false;
  return invoke('preview_reminder', { id }).then(() => true).catch(() => false);
}

export async function getAutostart(): Promise<boolean> {
  if (!inTauri()) return false;
  return invoke<boolean>('autostart_status').catch(() => false);
}

export async function setAutostart(enabled: boolean): Promise<boolean | null> {
  if (!inTauri()) return null;
  return invoke<boolean>('set_autostart', { enabled }).catch(() => null);
}

export async function fitIslandWindow(
  width: number,
  height: number,
  anchor: IslandPosition,
): Promise<void> {
  if (!inTauri()) return;
  await invoke('fit_island_window', { width, height, anchor });
}

export async function setIslandPosition(anchor: IslandPosition, top: number): Promise<void> {
  if (!inTauri()) return;
  await invoke('set_island_position', { anchor, top });
}

export async function cursorProbe(width: number, height: number): Promise<CursorProbe> {
  if (!inTauri()) return { over: false, dx: 0, dy: 0 };
  return invoke<CursorProbe>('cursor_probe', { width, height });
}

export async function setIslandHoverRegion(width: number, height: number): Promise<void> {
  if (!inTauri()) return;
  await invoke('set_island_hover_region', { width, height }).catch(() => undefined);
}

export async function startCursorMonitor(): Promise<void> {
  if (!inTauri()) return;
  await invoke('start_cursor_monitor').catch(() => undefined);
}

export async function setIslandPassthrough(ignore: boolean): Promise<void> {
  if (!inTauri()) return;
  await invoke('set_island_passthrough', { ignore });
}

export async function openSettingsWindow(): Promise<void> {
  if (!inTauri()) return;
  await invoke('open_settings').catch(() => undefined);
}

export type QuickSettingsSection =
  | 'wifi'
  | 'bluetooth'
  | 'focus'
  | 'nightlight'
  | 'microphone'
  | 'brightness'
  | 'display'
  | 'battery'
  | 'screenshot';

export async function openWindowsSettings(section: QuickSettingsSection): Promise<void> {
  if (!inTauri()) return;
  await invoke('open_windows_settings', { section }).catch(() => undefined);
}

export async function openTaskManager(): Promise<void> {
  if (!inTauri()) return;
  await invoke('open_task_manager').catch(() => undefined);
}

export async function quitIsland(): Promise<void> {
  if (!inTauri()) return;
  await invoke('quit_app');
}

export interface Settings {
  theme: 'dark' | 'light';
  anchor: 'left' | 'center' | 'right';
  top_margin: number;
  idle: IdleStyle;
  nav: 'rail' | 'tabs';
  greeting_mode: 'auto' | 'custom';
  greeting_text: string;
  greeting_text2: string;
  greeting_show2: boolean;
  greeting_pool: string;
  mascot_scale: number;
  radius_idle: number;
  radius_panel: number;
  welcome_ms: number;
  morph_ms: number;
  notifications: boolean;
  hud_ms: number;
  notice_ms: number;
  poll_ms: number;
  favorites?: Favorite[];
}

export async function getSettings(): Promise<Settings | null> {
  if (!inTauri()) return null;
  return invoke<Settings>('get_settings').catch(() => null);
}

export async function setSettings(settings: Settings): Promise<void> {
  if (!inTauri()) return;
  await invoke('set_settings', { settings });
}
