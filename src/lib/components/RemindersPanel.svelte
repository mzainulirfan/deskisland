<script lang="ts">
  import Icon from './Icon.svelte';
  import { island } from '$lib/stores/island.svelte';
  import { previewReminder, saveReminders } from '$lib/services/desktop';
  import type { Reminder } from '$lib/types/system';

  const DAY_SHORT = ['Sen', 'Sel', 'Rab', 'Kam', 'Jum', 'Sab', 'Min'];
  const EVERY_DAY = 0b111_1111;
  /** Senin sampai Jumat: what a workday alarm is worth. */
  const WORKDAYS = 0b001_1111;
  const MAX = 12;
  const TIME_RE = /^([01]\d|2[0-3]):[0-5]\d$/;

  const reminders = $derived(island.reminders);
  const full = $derived(reminders.length >= MAX);

  /** `id: 0` marks a not-yet-saved row; the backend assigns the real id on save. */
  let draft = $state<Reminder | null>(null);
  let busy = $state(false);

  const timeOk = $derived(TIME_RE.test(draft?.time ?? ''));
  const textOk = $derived((draft?.text.trim().length ?? 0) > 0);
  const canSave = $derived(!!draft && timeOk && textOk && !busy);

  function nextMinute() {
    const at = new Date(Date.now() + 60_000);
    const pad = (value: number) => String(value).padStart(2, '0');
    return `${pad(at.getHours())}:${pad(at.getMinutes())}`;
  }

  function daysLabel(days: number) {
    if (days === EVERY_DAY) return 'Setiap hari';
    if (days === WORKDAYS) return 'Hari kerja';
    const named = DAY_SHORT.filter((_, index) => days & (1 << index));
    return named.length > 3 ? `${named.length} hari seminggu` : named.join(', ');
  }

  /// Optimistic like the favorites list: the store moves first, then the file. The reply is
  /// the list the backend kept, so a newly created row picks up the id it was given.
  async function commit(next: Reminder[]) {
    if (busy) return;
    busy = true;
    island.reminders = next;
    try {
      island.reminders = await saveReminders(next);
    } finally {
      busy = false;
    }
  }

  function create() {
    draft = { id: 0, text: '', time: nextMinute(), days: EVERY_DAY, enabled: true };
  }

  function edit(reminder: Reminder) {
    draft = { ...reminder };
  }

  function save() {
    if (!draft || !canSave) return;
    const entry: Reminder = { ...draft, text: draft.text.trim() };
    void commit(
      entry.id ? reminders.map((r) => (r.id === entry.id ? entry : r)) : [...reminders, entry],
    );
    draft = null;
  }

  function remove(reminder: Reminder) {
    void commit(reminders.filter((r) => r.id !== reminder.id));
    if (draft?.id === reminder.id) draft = null;
  }

  function toggle(reminder: Reminder) {
    void commit(
      reminders.map((r) => (r.id === reminder.id ? { ...r, enabled: !r.enabled } : r)),
    );
  }

  function setDay(bit: number) {
    if (!draft) return;
    const days = draft.days ^ bit;
    // An empty week is not a schedule; keep the day that was just switched off.
    draft.days = days === 0 ? EVERY_DAY : days;
  }

  function setDays(days: number) {
    if (draft) draft.days = days;
  }
</script>

<div class="remind-shell flex h-full min-h-0 gap-2.5 rounded-[15px] p-2.5 ring-1 ring-[color:var(--line)]">
  <div class="flex min-w-0 flex-1 flex-col">
    <div class="flex h-6 shrink-0 items-center justify-between">
      <h2
        class="flex items-center gap-1.5 text-[9px] font-medium uppercase tracking-[0.08em] text-[var(--muted)]"
      >
        <span class="size-1.5 rounded-full bg-[var(--di-amber)]" aria-hidden="true"></span>
        Pengingat harian
        <span class="font-normal normal-case text-[var(--faint)]">{reminders.length} / {MAX}</span>
      </h2>
      <button
        type="button"
        class="head-btn"
        aria-label="Tambah pengingat"
        title={full ? 'Maksimal 12 pengingat' : 'Tambah pengingat'}
        disabled={full || !!draft}
        onclick={create}
      >
        <Icon name="plus" class="size-3" />
      </button>
    </div>

    {#if reminders.length === 0 && !draft}
      <div class="remind-empty">
        <span class="remind-empty-icon"><Icon name="bell" class="size-5" /></span>
        <p class="text-[10.5px] leading-snug text-[var(--muted)]">
          Belum ada pengingat.<br />Jam berbunyi walau island sedang tersembunyi.
        </p>
        <button type="button" class="remind-empty-cta" onclick={create}>
          <Icon name="plus" class="size-3" /> Buat pengingat
        </button>
      </div>
    {:else}
      <ul class="remind-list mt-1.5 min-h-0 flex-1">
        {#each reminders as reminder (reminder.id)}
          {@const active = draft?.id === reminder.id}
          <li class="remind-row" class:remind-off={!reminder.enabled} class:remind-on={active}>
            <button type="button" class="remind-main" onclick={() => edit(reminder)}>
              <span class="remind-time">{reminder.time}</span>
              <span class="flex min-w-0 flex-col leading-tight">
                <span class="truncate text-[11px] font-semibold">{reminder.text}</span>
                <span class="truncate text-[9px] text-[var(--faint)]"
                  >{daysLabel(reminder.days)}</span
                >
              </span>
            </button>
            <button
              type="button"
              class="row-btn"
              role="switch"
              aria-checked={reminder.enabled}
              aria-label={reminder.enabled ? `Matikan ${reminder.text}` : `Nyalakan ${reminder.text}`}
              title={reminder.enabled ? 'Nyala' : 'Mati'}
              onclick={() => toggle(reminder)}
            >
              <Icon name={reminder.enabled ? 'bell' : 'focus'} class="size-3.5" />
            </button>
            <button
              type="button"
              class="row-btn"
              aria-label={`Uji ${reminder.text}`}
              title="Bunyikan sekarang"
              onclick={() => void previewReminder(reminder.id)}
            >
              <Icon name="play" class="size-3" />
            </button>
            <button
              type="button"
              class="row-btn row-btn-x"
              aria-label={`Hapus ${reminder.text}`}
              title="Hapus"
              onclick={() => remove(reminder)}
            >
              <Icon name="trash" class="size-3.5" />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  {#if draft}
    <div class="remind-editor flex w-[212px] shrink-0 flex-col gap-1.5">
      <div class="flex items-center justify-between">
        <h3 class="text-[10px] font-semibold text-[var(--ink)]">
          {draft.id ? 'Ubah pengingat' : 'Pengingat baru'}
        </h3>
        <button
          type="button"
          class="row-btn row-btn-x"
          aria-label="Batal"
          onclick={() => (draft = null)}
        >
          <Icon name="close" class="size-3" />
        </button>
      </div>

      <input
        class="remind-input"
        type="text"
        maxlength="40"
        placeholder="Minum air"
        aria-label="Isi pengingat"
        bind:value={draft.text}
        onkeydown={(event) => {
          if (event.key === 'Enter') save();
        }}
      />
      <input
        class="remind-input remind-time-input"
        type="time"
        aria-label="Jam"
        bind:value={draft.time}
        onkeydown={(event) => {
          if (event.key === 'Enter') save();
        }}
      />

      <div class="flex flex-wrap gap-1">
        {#each DAY_SHORT as label, index}
          {@const on = draft.days & (1 << index)}
          <button
            type="button"
            class="day-chip"
            class:day-chip-on={!!on}
            aria-pressed={!!on}
            onclick={() => setDay(1 << index)}
          >
            {label}
          </button>
        {/each}
      </div>
      <div class="flex gap-1">
        <button type="button" class="preset-btn" onclick={() => setDays(EVERY_DAY)}>
          Setiap hari
        </button>
        <button type="button" class="preset-btn" onclick={() => setDays(WORKDAYS)}>
          Hari kerja
        </button>
      </div>

      <button type="button" class="save-btn" disabled={!canSave} onclick={save}>
        <Icon name="check" class="size-3.5" /> Simpan
      </button>
      <p class="text-[8.5px] leading-tight text-[var(--faint)]">
        {timeOk && textOk
          ? `Berbunyi pada ${draft.time} lewat jam sistem.`
          : 'Isi teks dan jam (HH:MM) untuk menyimpan.'}
      </p>
    </div>
  {/if}
</div>

<style>
  .remind-shell {
    --tint: var(--di-amber);
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--tint) 7%, transparent), transparent 55%),
      var(--card);
  }

  .head-btn {
    display: flex;
    width: 22px;
    height: 20px;
    align-items: center;
    justify-content: center;
    border-radius: 7px;
    color: var(--di-amber);
    background: color-mix(in srgb, var(--di-amber) 14%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 24%, transparent);
    transition: background 120ms var(--ease-out);
  }

  .head-btn:hover:not(:disabled) {
    background: color-mix(in srgb, var(--di-amber) 24%, transparent);
  }

  .head-btn:disabled {
    color: var(--faint);
    background: transparent;
    box-shadow: none;
  }

  .remind-list {
    display: flex;
    flex-direction: column;
    gap: 3px;
    overflow-y: auto;
    scrollbar-width: thin;
    scrollbar-color: color-mix(in srgb, var(--ink) 18%, transparent) transparent;
  }

  .remind-row {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    gap: 2px;
    height: 34px;
    padding-right: 2px;
    border-radius: 9px;
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .remind-on {
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 46%, transparent),
      0 1px 6px color-mix(in srgb, var(--ink) 10%, transparent);
  }

  .remind-off .remind-time,
  .remind-off .remind-main {
    opacity: 0.45;
  }

  .remind-main {
    display: flex;
    min-width: 0;
    flex: 1;
    align-items: center;
    gap: 9px;
    height: 100%;
    padding: 0 4px 0 8px;
    text-align: left;
  }

  .remind-time {
    font-size: 12.5px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    color: var(--di-amber);
  }

  .row-btn {
    display: flex;
    width: 22px;
    height: 22px;
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    border-radius: 6px;
    color: var(--muted);
    transition:
      background 120ms var(--ease-out),
      color 120ms var(--ease-out);
  }

  .row-btn:hover {
    background: color-mix(in srgb, var(--ink) 8%, transparent);
    color: var(--ink);
  }

  .row-btn-x:hover {
    color: var(--di-red);
    background: color-mix(in srgb, var(--di-red) 12%, transparent);
  }

  .remind-empty {
    display: flex;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 7px;
    text-align: center;
  }

  .remind-empty-icon {
    display: flex;
    width: 40px;
    height: 40px;
    align-items: center;
    justify-content: center;
    border-radius: 12px;
    color: var(--di-amber);
    background: color-mix(in srgb, var(--di-amber) 12%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 22%, transparent);
  }

  .remind-empty-cta {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 11px;
    border-radius: 999px;
    background: var(--di-amber);
    color: #0b0d12;
    font-size: 10.5px;
    font-weight: 700;
  }

  .remind-editor {
    padding-left: 10px;
    border-left: 1px solid var(--line);
  }

  .remind-input {
    width: 100%;
    height: 28px;
    padding: 0 9px;
    border: 0;
    border-radius: 8px;
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--ink);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }

  .remind-input:focus {
    box-shadow:
      inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 55%, transparent),
      0 0 0 3px color-mix(in srgb, var(--di-amber) 12%, transparent);
    outline: none;
  }

  .remind-time-input {
    height: 30px;
    font-size: 13px;
    font-weight: 700;
  }

  .day-chip {
    width: 32px;
    height: 20px;
    border-radius: 6px;
    background: var(--chip);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--faint);
    font-size: 9px;
    font-weight: 600;
    transition:
      background 120ms var(--ease-out),
      color 120ms var(--ease-out);
  }

  .day-chip:hover {
    color: var(--ink);
  }

  .day-chip-on,
  .day-chip-on:hover {
    background: color-mix(in srgb, var(--di-amber) 22%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-amber) 40%, transparent);
    color: var(--ink);
  }

  .preset-btn {
    height: 20px;
    padding: 0 8px;
    border-radius: 999px;
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
    font-size: 9px;
    font-weight: 600;
  }

  .preset-btn:hover {
    color: var(--ink);
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }

  .save-btn {
    display: flex;
    height: 28px;
    align-items: center;
    justify-content: center;
    gap: 6px;
    margin-top: auto;
    border-radius: 8px;
    background: color-mix(in srgb, var(--di-amber) 82%, transparent);
    color: #0b0d12;
    font-size: 11px;
    font-weight: 700;
    transition: opacity 120ms var(--ease-out);
  }

  .save-btn:disabled {
    opacity: 0.4;
  }
</style>
