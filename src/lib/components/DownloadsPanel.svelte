<script lang="ts">
  import Icon from './Icon.svelte';
  import { island } from '$lib/stores/island.svelte';
  import { openDownload, openDownloadsFolder, revealDownload } from '$lib/services/desktop';

  const files = $derived(island.downloadList);

  function size(bytes: number) {
    const kb = bytes / 1024;
    if (kb >= 1024) {
      const mb = kb / 1024;
      return mb >= 1024 ? `${(mb / 1024).toFixed(1).replace('.', ',')} GB` : `${mb.toFixed(1).replace('.', ',')} MB`;
    }
    return `${Math.max(1, Math.round(kb))} KB`;
  }

  function ext(name: string): string {
    const dot = name.lastIndexOf('.');
    return dot > 0 ? name.slice(dot + 1).toLowerCase() : '';
  }

  // Colour the file-type badge by family so the eye can group mixed downloads fast.
  const TONE_BY_EXT: Record<string, string> = {
    png: 'var(--di-purple)',
    jpg: 'var(--di-purple)',
    jpeg: 'var(--di-purple)',
    gif: 'var(--di-purple)',
    webp: 'var(--di-purple)',
    svg: 'var(--di-purple)',
    bmp: 'var(--di-purple)',
    zip: 'var(--di-amber)',
    rar: 'var(--di-amber)',
    '7z': 'var(--di-amber)',
    tar: 'var(--di-amber)',
    gz: 'var(--di-amber)',
    exe: 'var(--di-red)',
    msi: 'var(--di-red)',
    pdf: 'var(--di-blue)',
    doc: 'var(--di-blue)',
    docx: 'var(--di-blue)',
    xls: 'var(--di-blue)',
    xlsx: 'var(--di-blue)',
    ppt: 'var(--di-blue)',
    pptx: 'var(--di-blue)',
    txt: 'var(--di-blue)',
    csv: 'var(--di-blue)',
    mp3: 'var(--di-green)',
    wav: 'var(--di-green)',
    flac: 'var(--di-green)',
    mp4: 'var(--di-green)',
    mkv: 'var(--di-green)',
    mov: 'var(--di-green)',
  };

  const toneOf = (name: string) => TONE_BY_EXT[ext(name)] ?? 'var(--muted)';
</script>

<div
  class="flex h-full flex-col rounded-[16px] bg-[var(--card)] p-3 ring-1 ring-[color:var(--line)]"
>
  <header class="flex h-5 shrink-0 items-center justify-between gap-3">
    <h2
      class="flex items-center gap-1.5 text-[9px] font-medium tracking-[0.08em] text-[var(--muted)] uppercase"
    >
      <span class="size-1.5 rounded-full bg-[var(--di-green)]"></span>
      Unduhan
      <span class="text-[var(--faint)] normal-case">· {files.length} terbaru</span>
    </h2>

    <button
      type="button"
      class="head-btn"
      aria-label="Buka folder Downloads"
      title="Buka folder Downloads"
      onclick={() => void openDownloadsFolder()}
    >
      <Icon name="folder" class="size-2.5" />
      Buka Folder
    </button>
  </header>

  {#if files.length}
    <ol class="dl-list mt-1.5 min-h-0 flex-1 overflow-y-auto pr-0.5" aria-label="File terbaru">
      {#each files as file (file.path)}
        {@const tone = toneOf(file.name)}
        <li class="dl-row" style:--file-tone={tone}>
          <button
            type="button"
            class="dl-main"
            title="{file.name} · {size(file.size)} — klik untuk membuka"
            onclick={() => void openDownload(file.path)}
          >
            <span class="dl-badge">{ext(file.name).slice(0, 3).toUpperCase() || '···'}</span>
            <span class="flex min-w-0 flex-1 flex-col leading-tight text-left">
              <span class="truncate text-[11px] font-medium">{file.name}</span>
              <span class="text-[9px] tabular-nums text-[var(--faint)]">{size(file.size)}</span>
            </span>
          </button>
          <span class="dl-actions">
            <button
              type="button"
              class="row-action"
              aria-label="Buka file"
              title="Buka file"
              onclick={() => void openDownload(file.path)}
            >
              <Icon name="external" class="size-3" />
            </button>
            <button
              type="button"
              class="row-action"
              aria-label="Buka di Explorer"
              title="Buka di Explorer"
              onclick={() => void revealDownload(file.path)}
            >
              <Icon name="folder" class="size-3" />
            </button>
          </span>
        </li>
      {/each}
    </ol>
  {:else}
    <div class="dl-empty">
      <span class="dl-empty-icon"><Icon name="download" class="size-4" /></span>
      <p class="text-[10.5px] leading-snug text-[var(--muted)]">
        Belum ada file.<br />Hasil unduh terbaru akan muncul di sini.
      </p>
    </div>
  {/if}
</div>

<style>
  .head-btn {
    display: flex;
    height: 18px;
    align-items: center;
    gap: 4px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--di-green) 12%, transparent);
    padding: 0 8px;
    color: var(--di-green);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--di-green) 22%, transparent);
    font-size: 9px;
    font-weight: 500;
    transition: background 120ms ease;
  }

  .head-btn:hover {
    background: color-mix(in srgb, var(--di-green) 20%, transparent);
  }

  .dl-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
    scrollbar-width: thin;
    scrollbar-color: color-mix(in srgb, var(--ink) 18%, transparent) transparent;
  }

  .dl-row {
    display: flex;
    width: 100%;
    align-items: center;
    gap: 4px;
    border-radius: 9px;
    transition: background 120ms ease;
  }

  .dl-row:hover {
    background: color-mix(in srgb, var(--ink) 6%, transparent);
  }

  .dl-main {
    display: flex;
    min-width: 0;
    flex: 1;
    align-items: center;
    gap: 8px;
    border-radius: 9px;
    padding: 4px 6px;
    text-align: left;
  }

  .dl-badge {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    flex-shrink: 0;
    border-radius: 7px;
    background: color-mix(in srgb, var(--file-tone) 14%, transparent);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--file-tone) 26%, transparent);
    color: var(--file-tone);
    font-size: 8px;
    font-weight: 700;
    letter-spacing: 0.02em;
  }

  .dl-actions {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    gap: 2px;
    padding-right: 4px;
    opacity: 0;
    transition: opacity 120ms ease;
  }

  .dl-row:hover .dl-actions,
  .dl-row:focus-within .dl-actions {
    opacity: 1;
  }

  .row-action {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 6px;
    color: var(--faint);
    transition:
      color 120ms ease,
      background 120ms ease;
  }

  .row-action:hover {
    color: var(--ink);
    background: color-mix(in srgb, var(--ink) 10%, transparent);
  }

  .dl-empty {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    text-align: center;
  }

  .dl-empty-icon {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 11px;
    background: color-mix(in srgb, var(--ink) 4%, transparent);
    box-shadow: inset 0 0 0 1px var(--line);
    color: var(--muted);
  }

  .dl-list :global(*) {
    min-width: 0;
  }
</style>
