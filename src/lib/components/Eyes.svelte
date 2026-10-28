<script lang="ts">
  import { onMount } from 'svelte';
  import { island } from '$lib/stores/island.svelte';

  let {
    class: className = '',
    greeting = false,
    travel = 1,
  }: { class?: string; greeting?: boolean; travel?: number } = $props();

  // A 12×16 shell with a 6×8 pupil leaves 3px horizontally and 4px vertically, so these
  // caps keep the pupil inside the eye even when `travel` raises them.
  const pupilX = $derived(Math.max(-2.2, Math.min(2.2, island.cursor.x / 170)) * travel);
  const pupilY = $derived(Math.max(-2.4, Math.min(2.4, island.cursor.y / 140)) * travel);

  // An infinite CSS blink keeps WebView2's compositor awake around the clock, which is
  // expensive for an always-on-top transparent window. Trigger one short blink at a
  // time instead and let the compositor idle between them.
  let blinking = $state(false);

  onMount(() => {
    let blinkTimer: ReturnType<typeof setTimeout> | undefined;
    let closeTimer: ReturnType<typeof setTimeout> | undefined;

    const schedule = () => {
      blinkTimer = setTimeout(
        () => {
          blinking = true;
          closeTimer = setTimeout(() => {
            blinking = false;
            schedule();
          }, 260);
        },
        5000 + Math.random() * 4000,
      );
    };

    schedule();
    return () => {
      clearTimeout(blinkTimer);
      clearTimeout(closeTimer);
    };
  });
</script>

<span
  class="eyes {className}"
  class:is-blinking={blinking}
  class:is-greeting={greeting}
  aria-hidden="true"
>
  {#each [0, 1] as eye}
    <span class="eye-shell">
      <span class="eye">
        <span class="pupil" style:transform="translate({pupilX}px, {pupilY}px)"></span>
      </span>
    </span>
  {/each}
</span>

<style>
  .eyes {
    display: inline-flex;
    align-items: center;
    gap: calc(3px * var(--m, 1));
  }

  .eye-shell {
    display: flex;
    transform-origin: center;
  }

  .eyes.is-blinking .eye-shell {
    animation: eyes-wink 240ms ease-in-out;
  }

  .eyes.is-greeting {
    animation: eyes-scan 4s ease-in-out 1600ms infinite;
  }

  .eye {
    position: relative;
    display: flex;
    width: calc(12px * var(--m, 1));
    height: calc(16px * var(--m, 1));
    align-items: center;
    justify-content: center;
    overflow: hidden;
    border-radius: calc(6px * var(--m, 1));
    background: #fff;
    box-shadow: inset 0 -1px 2px rgba(15, 23, 42, 0.25);
    transform-origin: center;
    animation: eyes-open 620ms cubic-bezier(0.22, 1.3, 0.36, 1) both;
  }

  .eye-shell:nth-child(2) .eye {
    animation-delay: 70ms;
  }

  .eyes:hover .eye-shell:last-child .eye,
  :global(.welcome-root:hover) .eye-shell:last-child .eye,
  :global(.eyes-badge:hover) .eye-shell:last-child .eye {
    animation: eyes-wink 720ms 100ms ease-in-out;
  }

  .pupil {
    display: block;
    width: calc(6px * var(--m, 1));
    height: calc(8px * var(--m, 1));
    border-radius: 999px;
    background: #0b0d12;
    transition: transform 120ms ease-out;
    animation: eyes-look 1.4s 480ms ease-in-out;
  }

  @keyframes eyes-open {
    0% {
      transform: scaleY(0.06);
    }
    65% {
      transform: scaleY(1.12);
    }
    100% {
      transform: scaleY(1);
    }
  }

  @keyframes eyes-look {
    0% {
      transform: translate(0, calc(0.5px * var(--m, 1)));
    }
    30% {
      transform: translate(calc(-2px * var(--m, 1)), calc(1px * var(--m, 1)));
    }
    65% {
      transform: translate(calc(2px * var(--m, 1)), calc(-0.5px * var(--m, 1)));
    }
    100% {
      transform: translate(0, 0);
    }
  }

  @keyframes eyes-wink {
    0%,
    100% {
      transform: scaleY(1);
    }
    45%,
    62% {
      transform: scaleY(0.08);
    }
  }

  @keyframes eyes-scan {
    0%,
    54% {
      transform: translateX(0);
    }
    60%,
    70% {
      transform: translateX(calc(-3px * var(--m, 1)));
    }
    78%,
    88% {
      transform: translateX(calc(3px * var(--m, 1)));
    }
    95%,
    100% {
      transform: translateX(0);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .eyes.is-blinking .eye-shell,
    .eyes.is-greeting,
    .eyes.is-greeting .pupil,
    .eye-shell,
    .eye,
    .pupil {
      animation: none;
      transition: none;
    }
  }
</style>
