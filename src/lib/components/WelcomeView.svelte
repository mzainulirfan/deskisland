<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import Mascot from './Mascot.svelte';
  import { island } from '$lib/stores/island.svelte';
  import { builtinGreetings, type Greeting } from '$lib/greetings';

  let {
    duration = 6000,
    index = 0,
    entries = [],
  }: {
    duration?: number;
    /** Which appearance this is, used to rotate `entries` so no two in a row repeat. */
    index?: number;
    /** Pool greetings for a reappearance; empty means the startup greeting. */
    entries?: Greeting[];
  } = $props();

  // The random built-in pick is chosen once per welcome; only the choice between the
  // built-in and the user's custom text reacts to settings changes.
  const picked = builtinGreetings()[Math.floor(Math.random() * builtinGreetings().length)];
  // Snapshot on purpose: Island.svelte remounts this view with `{#key welcomeSeq}` for
  // every new greeting, so the line must not shift while it is being typed.
  const rotated = untrack(() =>
    entries.length > 0 ? entries[index % entries.length] : null,
  );

  const usingCustom = $derived(
    rotated === null &&
      island.greetingMode === 'custom' &&
      island.greetingText.trim().length > 0,
  );
  const fullTitle = $derived(
    rotated ? rotated.title : usingCustom ? island.greetingText.trim() : picked.title,
  );
  const fullBody = $derived(
    rotated
      ? rotated.body
      : usingCustom
        ? island.greetingText2.trim() || 'Selamat datang kembali'
        : picked.body,
  );
  const showLine2 = $derived(
    rotated ? rotated.body.length > 0 : !usingCustom || island.greetingShowLine2,
  );

  const TYPE_START_MS = 2050;
  const CHAIN_GAP_MS = 300;
  const TITLE_SPEED: [number, number] = [45, 50];
  const BODY_SPEED: [number, number] = [28, 32];
  // The ring starts after the mascot has dropped, slid left and settled into its hop
  // loop, and drains for exactly the remaining welcome window.
  const RING_DELAY_MS = 2500;
  const ringDurationMs = $derived(Math.max(500, duration - RING_DELAY_MS));

  let titleText = $state('');
  let bodyText = $state('');
  let titleTyping = $state(false);
  let bodyTyping = $state(false);

  let timers: ReturnType<typeof setTimeout>[] = [];
  function schedule(fn: () => void, ms: number) {
    timers.push(setTimeout(fn, ms));
  }
  function clearTimers() {
    for (const t of timers) clearTimeout(t);
    timers = [];
  }
  function typeLine(
    source: string,
    write: (value: string) => void,
    flag: (typing: boolean) => void,
    speed: [number, number],
    done: () => void,
  ) {
    if (!source.length) {
      write('');
      done();
      return;
    }
    flag(true);
    let i = 0;
    const step = () => {
      i += 1;
      write(source.slice(0, i));
      if (i < source.length) schedule(step, speed[0] + Math.random() * speed[1]);
      else {
        flag(false);
        done();
      }
    };
    step();
  }

  const reducedMotion =
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  // The mascot scales with the "Ukuran maskot" setting, so every geometry that touches
  // it is derived: the drop box, the centred start position, and where the text begins
  // (it rests at left:10px plus an 8px gap).
  const mascotPx = $derived(Math.round(34 * island.mascotScale));
  const mascotTop = $derived(Math.max(3, Math.round((50 - mascotPx) / 2)));
  const mascotLeft = $derived(
    reducedMotion ? '10px' : `calc(50% - ${Math.round(mascotPx / 2)}px)`,
  );
  const contentLeft = $derived(mascotPx + 18);

  // Both the greeting pill and its progress ring follow the "Radius pill" setting; the
  // ring is inset by 0.75px on each side, so its corner radius shrinks by the same amount.
  const ringRadius = $derived(Math.min(Math.max(island.idleRadius - 0.75, 1), 24.25));

  function runTimeline() {
    clearTimers();
    titleText = '';
    bodyText = '';
    titleTyping = false;
    bodyTyping = false;
    if (reducedMotion) {
      titleText = fullTitle;
      bodyText = showLine2 ? fullBody : '';
      return;
    }
    schedule(() => {
      typeLine(fullTitle, (v) => (titleText = v), (v) => (titleTyping = v), TITLE_SPEED, () => {
        if (!showLine2) return;
        schedule(
          () =>
            typeLine(fullBody, (v) => (bodyText = v), (v) => (bodyTyping = v), BODY_SPEED, () => {}),
          CHAIN_GAP_MS,
        );
      });
    }, TYPE_START_MS);
  }

  runTimeline();
  onDestroy(clearTimers);
</script>

<div class="welcome-root" aria-hidden="true">
  <div class="greet-bg" style:border-radius="{island.idleRadius}px"></div>

  <svg class="greet-ring" viewBox="0 0 288 50" preserveAspectRatio="none">
    <rect
      x="0.75"
      y="0.75"
      width="286.5"
      height="48.5"
      rx={ringRadius}
      pathLength="1"
      style:animation-duration="{ringDurationMs}ms"
      style:animation-delay="{RING_DELAY_MS}ms"
    />
  </svg>

  <div
    class="greet-mascot"
    style:top="{mascotTop}px"
    style:left={mascotLeft}
    style:width="{mascotPx}px"
    style:height="{mascotPx}px"
  >
    <div class="greet-hop">
      <Mascot mode="welcome" />
    </div>
  </div>

  <div class="greet-content" style:left="{contentLeft}px" class:greet-content-single={!showLine2}>
    <div class="greet-title" class:greet-title-lg={!showLine2} class:typing={titleTyping}>
      {titleText}
    </div>
    {#if showLine2}
      <div class="greet-sub" class:typing={bodyTyping}>{bodyText}</div>
    {/if}
  </div>
</div>

<style>
  .welcome-root {
    position: relative;
    width: 100%;
    height: 100%;
  }

  /* Reference sequence: the shared outer glow is muted by `.island-welcome`, so the
     pill background is drawn here. It starts as a 50-px circle behind the mascot,
     bursts out wider than the pill, then snaps back to 288 px — the dynamic-island
     expand-then-collapse is what makes the greeting the centre of attention. The
     burst stays inside the window's 32 px transparent padding so no anchor clips it. */
  .greet-bg {
    position: absolute;
    left: 0;
    top: 0;
    width: 50px;
    height: 50px;
    border-radius: 25px;
    opacity: 0;
    background:
      radial-gradient(130% 110px at 50% 0, rgba(var(--glow-rgb), 0.2), transparent 70%),
      var(--surface);
    box-shadow:
      0 0 18px rgba(var(--glow-rgb), 0.42),
      0 4px 26px rgba(var(--glow-rgb), 0.2),
      0 8px 22px var(--drop),
      inset 0 1px 0 var(--sheen);
    border: 1px solid rgba(var(--glow-rgb), 0.35);
    z-index: 0;
    animation: greet-bg-in 1000ms linear 1450ms forwards;
  }

  @keyframes greet-bg-in {
    0% {
      opacity: 0;
      left: 0;
      width: 50px;
      transform: scale(0.6);
      animation-timing-function: cubic-bezier(0.22, 1.3, 0.36, 1);
    }
    22% {
      opacity: 1;
      left: 0;
      width: 50px;
      transform: scale(1);
      animation-timing-function: cubic-bezier(0.2, 0.9, 0.25, 1);
    }
    58% {
      opacity: 1;
      left: -28px;
      width: 344px;
      transform: scale(1);
      animation-timing-function: cubic-bezier(0.65, 0, 0.35, 1);
    }
    100% {
      opacity: 1;
      left: 0;
      width: 288px;
      transform: scale(1);
    }
  }

  /* Mascot: falls from above the pill (centered), then slides to its resting corner.
     The parent hop keyframe carries squash-and-stretch + a small rotation, the badge
     itself drops, and Eyes scans left/right on the same 4s cycle. */
  .greet-mascot {
    position: absolute;
    z-index: 3;
    animation: greet-slide 520ms cubic-bezier(0.65, 0, 0.25, 1) 950ms forwards;
  }

  .greet-hop {
    transform-origin: 50% 100%;
    animation: greet-hop 4s ease-in-out 1600ms infinite;
  }

  .greet-content {
    position: absolute;
    right: 16px;
    top: 0;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    gap: 3px;
    z-index: 2;
    pointer-events: none;
    animation: greet-content-in 540ms 80ms ease-out both;
  }

  .greet-content-single {
    justify-content: center;
  }

  .greet-title {
    color: var(--ink);
    font-weight: 700;
    font-size: 13px;
    line-height: 16px;
    min-height: 16px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .greet-title-lg {
    font-size: 15px;
    line-height: 20px;
    min-height: 20px;
  }

  .greet-sub {
    color: color-mix(in srgb, var(--ink) 68%, transparent);
    font-weight: 500;
    font-size: 10.5px;
    line-height: 13px;
    min-height: 13px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .greet-title.typing::after,
  .greet-sub.typing::after {
    content: '';
    display: inline-block;
    width: 2px;
    height: 1em;
    margin-left: 2px;
    vertical-align: -2px;
    background: var(--di-blue);
    animation: greet-caret 700ms steps(1) infinite;
  }

  @keyframes greet-caret {
    50% {
      opacity: 0;
    }
  }

  /* Progress ring: the SVG rect has pathLength=1, so animating dashoffset from 1→0
     draws the entire outline of the pill exactly once, over `ringDurationMs`. */
  .greet-ring {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    pointer-events: none;
    overflow: visible;
    z-index: 1;
  }

  .greet-ring rect {
    fill: none;
    stroke: var(--di-blue);
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-dasharray: 1;
    stroke-dashoffset: 1;
    opacity: 0;
    animation: greet-ring-fill linear forwards;
  }

  @keyframes greet-ring-fill {
    0% {
      opacity: 0;
      stroke-dashoffset: 1;
    }
    1% {
      opacity: 0.9;
      stroke-dashoffset: 1;
    }
    100% {
      opacity: 0.9;
      stroke-dashoffset: 0;
    }
  }

  @keyframes greet-content-in {
    from {
      opacity: 0;
      transform: translateY(3px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes greet-slide {
    to {
      left: 10px;
    }
  }

  @keyframes greet-hop {
    0% {
      transform: translateY(0) scale(1, 1) rotate(0deg);
    }
    6% {
      transform: translateY(2px) scale(1.12, 0.84) rotate(0deg);
    }
    16% {
      transform: translateY(-9px) scale(0.92, 1.12) rotate(-5deg);
    }
    26% {
      transform: translateY(0) scale(1.12, 0.86) rotate(0deg);
    }
    31% {
      transform: translateY(0) scale(1, 1) rotate(0deg);
    }
    38% {
      transform: translateY(-5px) scale(0.95, 1.07) rotate(5deg);
    }
    46% {
      transform: translateY(0) scale(1.08, 0.9) rotate(0deg);
    }
    52% {
      transform: translateY(0) scale(1, 1) rotate(0deg);
    }
    60%,
    70% {
      transform: translateY(0) scale(1, 1) rotate(-6deg);
    }
    78%,
    88% {
      transform: translateY(0) scale(1, 1) rotate(6deg);
    }
    95%,
    100% {
      transform: translateY(0) scale(1, 1) rotate(0deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .greet-bg {
      animation: none;
      opacity: 1;
      width: 100%;
    }
    .greet-mascot {
      animation: none;
      left: 10px;
    }
    .greet-hop {
      animation: none;
    }
    .greet-content {
      animation: none;
    }
    .greet-ring rect {
      animation: none;
      opacity: 0.9;
      stroke-dashoffset: 0;
    }
  }
</style>
