<script lang="ts">
  import Icon, { type IconName } from './Icon.svelte';
  import { openWindowsSettings, type QuickSettingsSection } from '$lib/services/desktop';

  const rows: {
    section: QuickSettingsSection;
    label: string;
    hint: string;
    icon: IconName;
    color: string;
  }[] = [
    { section: 'wifi', label: 'Wi-Fi', hint: 'Jaringan & internet', icon: 'network', color: 'var(--di-blue)' },
    { section: 'bluetooth', label: 'Bluetooth', hint: 'Perangkat terhubung', icon: 'bluetooth', color: 'var(--di-blue)' },
    { section: 'nightlight', label: 'Mode malam', hint: 'Cahaya hangat', icon: 'moon', color: 'var(--di-purple)' },
    { section: 'focus', label: 'Fokus', hint: 'Jangan ganggu', icon: 'focus', color: 'var(--di-purple)' },
    { section: 'microphone', label: 'Mikrofon', hint: 'Izin aplikasi', icon: 'microphone', color: 'var(--di-green)' },
    { section: 'brightness', label: 'Kecerahan', hint: 'Cahaya layar', icon: 'sun', color: 'var(--di-amber)' },
  ];
</script>

<div class="grid h-full grid-cols-2 grid-rows-3 gap-x-1.5 gap-y-1">
  {#each rows as row}
    <button
      type="button"
      class="quick-row group flex h-full min-w-0 items-center gap-2.5 rounded-[10px] px-2.5 text-left transition-colors hover:bg-[color:color-mix(in_srgb,var(--ink)_6%,transparent)]"
      title={`Buka setelan ${row.label}`}
      onclick={() => void openWindowsSettings(row.section)}
    >
      <Icon name={row.icon} class="size-4 shrink-0" style={`color:${row.color}`} />
      <span class="flex min-w-0 flex-1 flex-col leading-tight">
        <span class="truncate text-[11.5px] font-medium">{row.label}</span>
        <span class="truncate text-[9.5px] text-[var(--faint)]">{row.hint}</span>
      </span>
      <Icon
        name="external"
        class="size-3 shrink-0 text-[var(--faint)] opacity-0 transition-opacity group-hover:opacity-100"
      />
    </button>
  {/each}
</div>

<style>
  .quick-row:focus-visible {
    outline: 1px solid color-mix(in srgb, var(--accent) 45%, transparent);
    outline-offset: -1px;
  }
</style>
