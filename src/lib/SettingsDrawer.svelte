<script lang="ts">
  import { createEventDispatcher, onDestroy } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { saveSettings, selectGameFolder } from './tauri-api';
  import type { LauncherSettings } from './tauri-api';

  export let open: boolean;
  export let settings: LauncherSettings;
  export let nickname: string;

  const dispatch = createEventDispatcher<{ close: void; logout: void }>();

  const maxRam = 16;
  const minRam = 1;

  let ram = settings.ram_gb;

  function onRamInput() {
    settings.ram_gb = ram;
    scheduleSave();
  }

  let dirty = false;
  let saveTimeout: ReturnType<typeof setTimeout> | null = null;

  function scheduleSave() {
    dirty = true;
    if (saveTimeout) clearTimeout(saveTimeout);
    saveTimeout = setTimeout(() => {
      saveSettings(settings);
      dirty = false;
    }, 500);
  }

  onDestroy(() => {
    if (saveTimeout) clearTimeout(saveTimeout);
  });

  function closeDrawer() {
    if (dirty) {
      saveSettings(settings);
    }
    dispatch('close');
  }

  function logout() {
    dispatch('logout');
  }

  $: ramText = ram % 1 === 0 ? `${ram}` : `${ram.toFixed(1)}`;
  $: ramPercent = ((ram - minRam) / (maxRam - minRam)) * 100;

  async function pickGameFolder() {
    try {
      const path = await selectGameFolder();
      if (path) {
        settings.game_path = path;
        scheduleSave();
      }
    } catch {}
  }
</script>

{#if open}
  <div class="backdrop" on:click={closeDrawer} on:keydown={(e) => e.key === 'Escape' && closeDrawer()} role="button" tabindex="-1" transition:fade={{ duration: 200 }} />

  <aside class="drawer" transition:fly={{ x: -360, duration: 300, easing: (t) => 1 - Math.pow(1 - t, 3) }}>
    <header class="drawer-header">
      <div class="drawer-title">
        <h2>Settings</h2>
        <span class="drawer-sub">Launch configuration</span>
      </div>
      <button class="close-btn" on:click={closeDrawer} aria-label="Close">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M18 6L6 18M6 6l12 12" stroke-linecap="round" />
        </svg>
      </button>
    </header>

    <div class="drawer-body">
      <section class="setting-group">
        <div class="group-header">
          <span class="group-title">RAM</span>
          <span class="group-value">{ramText} GB</span>
        </div>
        <div class="slider-wrap">
          <input
            type="range"
            min={minRam}
            max={maxRam}
            step={0.5}
            bind:value={ram}
            on:input={onRamInput}
            class="slider-input"
          />
        </div>
        <div class="slider-labels">
          <span>{minRam.toFixed(1)} GB</span>
          <span>{maxRam.toFixed(1)} GB</span>
        </div>
      </section>

      <section class="setting-group">
        <div class="group-header">
          <span class="group-title">Game folder</span>
        </div>
        <div class="path-row">
          <input
            type="text"
            class="path-input"
            value={settings.game_path || 'Default (AppData)'}
            readonly
          />
          <button class="browse-btn" on:click={pickGameFolder}>Browse</button>
        </div>
      </section>
    </div>

    <footer class="drawer-footer">
      <div class="account-block">
        <div class="avatar">{nickname.charAt(0).toUpperCase()}</div>
        <div class="account-info">
          <span class="account-name">{nickname}</span>
          <span class="account-role">Player</span>
        </div>
      </div>
      <button class="logout-btn" on:click={logout}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        <span>Change name</span>
      </button>
    </footer>
  </aside>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(2px);
  }

  .drawer {
    position: fixed;
    top: 0;
    left: 0;
    bottom: 0;
    z-index: 60;
    width: 360px;
    max-width: 90vw;
    background: var(--bg-surface);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    box-shadow: 8px 0 40px rgba(0, 0, 0, 0.3);
  }

  .drawer-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 28px 24px 20px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .drawer-title { display: flex; flex-direction: column; gap: 4px; }

  .drawer-title h2 {
    font-family: var(--font-display);
    font-size: 1.25rem;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .drawer-sub { font-size: 0.75rem; color: var(--text-tertiary); }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 8px;
    color: var(--text-tertiary);
    transition: color 0.2s ease, background 0.2s ease;
  }

  .close-btn:hover { color: var(--text); background: var(--bg-surface-hover); }
  .close-btn svg { width: 18px; height: 18px; }

  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 24px 24px;
    display: flex;
    flex-direction: column;
    gap: 28px;
  }

  .setting-group { display: flex; flex-direction: column; gap: 12px; }

  .group-header { display: flex; align-items: center; justify-content: space-between; }

  .group-title {
    font-size: 0.7rem;
    font-weight: 600;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }

  .group-value {
    font-family: var(--font-display);
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text);
    font-variant-numeric: tabular-nums;
  }

  .slider-wrap { position: relative; height: 24px; display: flex; align-items: center; }

  .slider-input {
    width: 100%;
    height: 24px;
    margin: 0;
    -webkit-appearance: none;
    appearance: none;
    background: transparent;
    cursor: pointer;
  }

  .slider-input::-webkit-slider-runnable-track {
    -webkit-appearance: none;
    appearance: none;
    height: 4px;
    background: var(--bg-elevated);
    border-radius: 2px;
  }

  .slider-input::-moz-range-track {
    height: 4px;
    background: var(--bg-elevated);
    border-radius: 2px;
    border: none;
  }

  .slider-input::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--accent);
    border: 3px solid var(--bg-surface);
    box-shadow: 0 0 0 1px var(--border-strong);
    cursor: pointer;
    transition: transform 0.15s ease;
    margin-top: -6px;
  }

  .slider-input::-webkit-slider-thumb:hover { transform: scale(1.15); }

  .slider-input::-moz-range-thumb {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--accent);
    border: 3px solid var(--bg-surface);
    box-shadow: 0 0 0 1px var(--border-strong);
    cursor: pointer;
  }

  .slider-labels {
    display: flex;
    justify-content: space-between;
    font-size: 0.7rem;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
  }

  .path-row { display: flex; gap: 8px; }

  .path-input {
    flex: 1;
    padding: 10px 12px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 9px;
    font-size: 0.8125rem;
    color: var(--text-secondary);
    cursor: default;
  }

  .browse-btn {
    padding: 10px 14px;
    border-radius: 9px;
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .browse-btn:hover { border-color: var(--border-strong); color: var(--text); }

  .drawer-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 24px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    gap: 12px;
  }

  .account-block { display: flex; align-items: center; gap: 10px; min-width: 0; }

  .avatar {
    width: 34px;
    height: 34px;
    border-radius: 9px;
    background: var(--accent);
    color: var(--accent-contrast);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1rem;
    flex-shrink: 0;
  }

  .account-info { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
  .account-name { font-size: 0.8125rem; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .account-role { font-size: 0.68rem; color: var(--text-tertiary); }

  .logout-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-radius: 8px;
    border: 1px solid var(--border);
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: all 0.15s ease;
    flex-shrink: 0;
  }

  .logout-btn:hover { color: var(--text); border-color: var(--border-strong); background: var(--bg-surface-hover); }
  .logout-btn svg { width: 14px; height: 14px; }
</style>
