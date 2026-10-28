export type Battery = { percent: number; charging: boolean };

export type SystemStats = {
  cpu: number;
  ram: number;
  ramUsed: number;
  ramTotal: number;
  rx: number;
  tx: number;
  disk: number;
  diskUsed: number;
  diskTotal: number;
  battery: Battery | null;
};

export type Volume = { percent: number; muted: boolean; device: string };

export type IdleVariant = 'eyes' | 'min' | 'music' | 'mon' | 'timer' | 'battery' | 'notification';
export type IdleStyle = IdleVariant | 'auto';

export type CursorProbe = { over: boolean; dx: number; dy: number };

export type NowPlaying = {
  app: string;
  title: string;
  artist: string;
  playing: boolean;
  position: number;
  duration: number;
};

export type ProcessInfo = { name: string; cpu: number; memory: number };

export type DownloadEvent = { name: string; size: number; path: string };

export type Downloads = { count: number; event: DownloadEvent | null };

export type AppShortcut = { name: string; path: string };

export type Favorite = { name: string; path: string };

export type NotificationEvent = { app: string; title: string; body: string };

export type TimerStatus = 'idle' | 'running' | 'paused' | 'done';

export type TimerInfo = {
  status: TimerStatus;
  label: string;
  /** Full sweep in ms. */
  total: number;
  /** Deadline as epoch ms, only while running. */
  endsAt: number;
  /** ms left: live while running, frozen while paused. */
  remaining: number;
};

export type Reminder = {
  /** Assigned by the backend when the item is created. */
  id: number;
  text: string;
  /** Local 24-hour wall clock, `HH:MM`. */
  time: string;
  /** Bitmask, bit 0 = Senin .. bit 6 = Minggu. */
  days: number;
  enabled: boolean;
};

/** What the Rust watcher sends when a reminder's minute arrives. */
export type ReminderEvent = { id: number; text: string; time: string };
