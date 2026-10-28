import { mount } from 'svelte';
import App from './App.svelte';
import Settings from './Settings.svelte';
import { island } from '$lib/stores/island.svelte';
import '@fontsource-variable/geist';
import './app.css';

const view =
  (window as Window & { __DESK_VIEW__?: string }).__DESK_VIEW__ ??
  new URLSearchParams(location.search).get('view') ??
  (location.hash === '#settings' ? 'settings' : null);

// Page-level styles for the settings document (background, scrolling) are gated
// on this attribute so they can never leak into the transparent island window.
document.documentElement.dataset.view = view === 'settings' ? 'settings' : 'island';

if (view === 'settings') {
  mount(Settings, { target: document.getElementById('app')! });
} else {
  // Settings must land before the island mounts: the window is placed with the
  // stored anchor during Island.svelte's onMount.
  void island.load().then(() =>
    mount(App, { target: document.getElementById('app')! }),
  );
}
