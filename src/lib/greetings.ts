export type Greeting = { title: string; body: string };

export function daypart(date = new Date()): string {
  const hour = date.getHours();
  return hour < 4 || hour >= 18 ? 'malam' : hour < 11 ? 'pagi' : hour < 15 ? 'siang' : 'sore';
}

export function builtinGreetings(): Greeting[] {
  const part = daypart();
  return [
    { title: `Selamat ${part}`, body: 'Selamat datang kembali' },
    {
      title: 'Hai, selamat datang kembali',
      body: part === 'malam' ? 'Selamat beristirahat' : 'Semoga harimu menyenangkan',
    },
    { title: 'Assalamualaikum', body: 'Selamat datang kembali' },
  ];
}

export const GREETING_PRESETS: string[] = [
  'Selamat datang',
  'Assalamualaikum',
  'Hai, selamat bekerja',
  'Bismillah, semoga produktif',
];

/// Reappearance greetings, one per line: `judul` or `judul | pesan`. Lines are trimmed,
/// capped to the same lengths the pill can render, and limited to twelve.
export function parseGreetingPool(raw: string): Greeting[] {
  return raw
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean)
    .slice(0, 12)
    .map((line) => {
      const split = line.indexOf('|');
      const title = (split < 0 ? line : line.slice(0, split)).trim().slice(0, 60);
      const body = split < 0 ? '' : line.slice(split + 1).trim().slice(0, 40);
      return { title, body };
    })
    .filter((entry) => entry.title.length > 0);
}
