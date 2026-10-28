<script lang="ts">
  import Eyes from './Eyes.svelte';
  import { island } from '$lib/stores/island.svelte';

  let {
    class: className = '',
    mode = 'pill',
  }: {
    class?: string;
    /** `welcome` plays the drop-in and turns the head toward the cursor. */
    mode?: 'pill' | 'welcome';
  } = $props();

  /** Size multiplier from Setelan → Sambutan → Ukuran maskot (34px base). */
  const s = $derived(island.mascotScale);

  const reducedMotion =
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  const looking = $derived(mode === 'welcome' && !reducedMotion);
  const lookX = $derived(Math.max(-1, Math.min(1, island.cursor.x / 170)));
  const lookY = $derived(Math.max(-1, Math.min(1, island.cursor.y / 120)));
  const headStyle = $derived(
    looking
      ? `translate(${(lookX * 2.5 * s).toFixed(2)}px, ${(lookY * 2 * s).toFixed(2)}px) rotate(${(lookX * 4).toFixed(2)}deg)`
      : '',
  );

  // The idle pill is 50px tall and clips its content, so the hop can only be as high as
  // the room the mascot size leaves above it — minus the stretch that lengthens the badge
  // at the peak. Below 1px it stops reading as a bounce, so that is the floor.
  const hopPx = $derived(Math.max(1, Math.min(5, (50 - 34 * s) / 2 - 2)));
</script>

<div
  class="mascot {className}"
  class:mascot-welcome={mode === 'welcome'}
  class:mascot-hop={mode === 'pill'}
  style="--m: {s}"
  style:--hop="{hopPx.toFixed(1)}px"
  aria-hidden="true"
>
  <div class="mascot-look" style={headStyle}>
    <Eyes greeting={mode === 'welcome'} travel={s * (mode === 'welcome' ? 1.25 : 1)} />
  </div>
</div>

<style>
  /* Solid navy base — mixing with `transparent` here turns the mascot into a ghost
     over the wallpaper while it falls through the window's transparent area. */
  .mascot {
    display: flex;
    width: calc(34px * var(--m));
    height: calc(34px * var(--m));
    flex-shrink: 0;
    align-items: center;
    justify-content: center;
    border-radius: calc(11px * var(--m));
    background: linear-gradient(
      145deg,
      color-mix(in srgb, var(--di-blue) 36%, #1c2740),
      color-mix(in srgb, var(--di-purple) 18%, #131b2e)
    );
    box-shadow:
      0 4px 12px rgba(0, 0, 0, 0.35),
      0 0 14px color-mix(in srgb, var(--di-blue) 26%, transparent);
    outline: 1px solid color-mix(in srgb, var(--di-blue) 34%, transparent);
  }

  /* Free-transform layer between the drop (this element) and the eyes: it carries the
     cursor-driven head turn, so no two transforms share one element. */
  .mascot-look {
    display: flex;
    transition: transform 150ms ease-out;
  }

  .mascot-welcome {
    opacity: 0;
    transform: translateY(-30px) scale(0.7);
    animation: mascot-drop 800ms cubic-bezier(0.3, 1.4, 0.5, 1) 100ms forwards;
    transition:
      transform 240ms var(--ease-island),
      box-shadow 240ms var(--ease-island);
  }

  .mascot-welcome:hover {
    box-shadow:
      0 4px 12px rgba(0, 0, 0, 0.35),
      0 0 20px color-mix(in srgb, var(--di-blue) 40%, transparent);
  }

  @keyframes mascot-drop {
    0% {
      opacity: 0;
      transform: translateY(-30px) scale(0.7);
    }
    15% {
      opacity: 1;
    }
    100% {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  /* Idle badge (variants Mata & Minimal): the greeting's squash-and-stretch hop, calmed
     down and looped with a long rest so it stays a glance, not a distraction. */
  .mascot-hop {
    transform-origin: 50% 100%;
    animation: mascot-hop 3.4s ease-in-out 900ms infinite;
  }

  @keyframes mascot-hop {
    0% {
      transform: translateY(0) scale(1, 1);
    }
    7% {
      transform: translateY(1px) scale(1.08, 0.9);
    }
    18% {
      transform: translateY(calc(-1 * var(--hop, 3px))) scale(0.95, 1.07);
    }
    28% {
      transform: translateY(0) scale(1.07, 0.92);
    }
    36% {
      transform: translateY(0) scale(0.98, 1.02);
    }
    44%,
    100% {
      transform: translateY(0) scale(1, 1);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .mascot-welcome {
      animation: none;
      opacity: 1;
      transform: none;
    }
    .mascot-hop {
      animation: none;
    }
  }
</style>
