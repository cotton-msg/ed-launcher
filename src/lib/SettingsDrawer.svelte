<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { fade, fly } from 'svelte/transition';

  export let open: boolean;
  export let nickname: string;

  const dispatch = createEventDispatcher<{ close: void; logout: void }>();

  // Settings state
  let ramGB = 4;
  const maxRam = 16;
  const minRam = 1;
  const ramStep = 0.5;

  let javaVersion = 'Java 17';
  let javaVersions = ['Java 8', 'Java 11', 'Java 17', 'Java 21'];

  let windowWidth = '1280';
  let windowHeight = '720';
  const resolutions = [
    { label: '854×480', w: '854', h: '480' },
    { label: '1280×720', w: '1280', h: '720' },
    { label: '1600×900', w: '1600', h: '900' },
    { label: '1920×1080', w: '1920', h: '1080' },
  ];

  let fullscreen = false;
  let vsync = true;
  let keepOpen = false;
  let debugInfo = false;

  function closeDrawer() {
    dispatch('close');
  }

  function logout() {
    dispatch('logout');
  }

  function setResolution(r: { w: string; h: string }) {
    windowWidth = r.w;
    windowHeight = r.h;
  }

  function ramPercent() {
    return ((ramGB - minRam) / (maxRam - minRam)) * 100;
  }

  function ramDisplay() {
    return ramGB % 1 === 0 ? `${ramGB}` : `${ramGB.toFixed(1)}`;
  }
</script>

{#if open}
  <div class="backdrop" on:click={closeDrawer} on:keydown={(e) => e.key === 'Escape' && closeDrawer()} role="button" tabindex="-1" transition:fade={{ duration: 200 }} />

  <aside class="drawer" transition:fly={{ x: -360, duration: 300, easing: (t) => 1 - Math.pow(1 - t, 3) }}>
    <header class="drawer-header">
      <div class="drawer-title">
        <h2>Настройки</h2>
        <span class="drawer-sub">Конфигурация запуска</span>
      </div>
      <button class="close-btn" on:click={closeDrawer} aria-label="Закрыть">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M18 6L6 18M6 6l12 12" stroke-linecap="round" />
        </svg>
      </button>
    </header>

    <div class="drawer-body">
      <!-- RAM allocation -->
      <section class="setting-group">
        <div class="group-header">
          <span class="group-title">Оперативная память</span>
          <span class="group-value">{ramDisplay()} ГБ</span>
        </div>
        <div class="slider-wrap">
          <div class="slider-track">
            <div class="slider-fill" style="width: {ramPercent()}%" />
          </div>
          <input
            type="range"
            min={minRam}
            max={maxRam}
            step={ramStep}
            bind:value={ramGB}
            class="slider-input"
          />
        </div>
        <div class="slider-labels">
          <span>{minRam.toFixed(1)} ГБ</span>
          <span>{maxRam.toFixed(1)} ГБ</span>
        </div>
      </section>

      <!-- Java version -->
      <section class="setting-group">
        <div class="group-header">
          <span class="group-title">Версия Java</span>
        </div>
        <div class="option-grid">
          {#each javaVersions as v}
            <button class="option-chip" class:active={javaVersion === v} on:click={() => (javaVersion = v)}>
              {v}
            </button>
          {/each}
        </div>
      </section>

      <!-- Resolution -->
      <section class="setting-group">
        <div class="group-header">
          <span class="group-title">Разрешение окна</span>
        </div>
        <div class="option-grid">
          {#each resolutions as r}
            <button
              class="option-chip"
              class:active={windowWidth === r.w && windowHeight === r.h}
              on:click={() => setResolution(r)}
            >
              {r.label}
            </button>
          {/each}
        </div>
      </section>

      <!-- Toggles -->
      <section class="setting-group">
        <div class="group-header">
          <span class="group-title">Параметры окна</span>
        </div>
        <div class="toggle-list">
          <label class="toggle-row">
            <span class="toggle-label">Полноэкранный режим</span>
            <button class="toggle" class:on={fullscreen} on:click={() => (fullscreen = !fullscreen)}>
              <span class="toggle-knob" />
            </button>
          </label>
          <label class="toggle-row">
            <span class="toggle-label">Вертикальная синхронизация</span>
            <button class="toggle" class:on={vsync} on:click={() => (vsync = !vsync)}>
              <span class="toggle-knob" />
            </button>
          </label>
          <label class="toggle-row">
            <span class="toggle-label">Не закрывать лаунчер</span>
            <button class="toggle" class:on={keepOpen} on:click={() => (keepOpen = !keepOpen)}>
              <span class="toggle-knob" />
            </button>
          </label>
          <label class="toggle-row">
            <span class="toggle-label">Отладочная информация</span>
            <button class="toggle" class:on={debugInfo} on:click={() => (debugInfo = !debugInfo)}>
              <span class="toggle-knob" />
            </button>
          </label>
        </div>
      </section>
    </div>

    <footer class="drawer-footer">
      <div class="account-block">
        <div class="avatar">{nickname.charAt(0).toUpperCase()}</div>
        <div class="account-info">
          <span class="account-name">{nickname}</span>
          <span class="account-role">Игрок</span>
        </div>
      </div>
      <button class="logout-btn" on:click={logout}>
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        <span>Сменить ник</span>
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

  .drawer-title {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .drawer-title h2 {
    font-family: var(--font-display);
    font-size: 1.25rem;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .drawer-sub {
    font-size: 0.75rem;
    color: var(--text-tertiary);
  }

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

  .close-btn:hover {
    color: var(--text);
    background: var(--bg-surface-hover);
  }

  .close-btn svg {
    width: 18px;
    height: 18px;
  }

  .drawer-body {
    flex: 1;
    overflow-y: auto;
    padding: 8px 24px 24px;
    display: flex;
    flex-direction: column;
    gap: 28px;
  }

  .setting-group {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .group-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

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

  /* Slider */
  .slider-wrap {
    position: relative;
    height: 24px;
    display: flex;
    align-items: center;
  }

  .slider-track {
    position: absolute;
    left: 0;
    right: 0;
    height: 4px;
    background: var(--bg-elevated);
    border-radius: 2px;
    overflow: hidden;
  }

  .slider-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
  }

  .slider-input {
    position: relative;
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
    background: transparent;
    border: none;
  }

  .slider-input::-moz-range-track {
    height: 4px;
    background: transparent;
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
  }

  .slider-input::-webkit-slider-thumb:hover {
    transform: scale(1.15);
  }

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

  /* Option chips */
  .option-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }

  .option-chip {
    padding: 8px 14px;
    border-radius: 9px;
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-secondary);
    transition: all 0.15s ease;
  }

  .option-chip:hover {
    border-color: var(--border-strong);
    color: var(--text);
  }

  .option-chip.active {
    background: var(--accent);
    color: var(--accent-contrast);
    border-color: var(--accent);
  }

  /* Toggles */
  .toggle-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .toggle-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 0;
  }

  .toggle-label {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .toggle {
    position: relative;
    width: 38px;
    height: 22px;
    border-radius: 11px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    transition: background 0.2s ease, border-color 0.2s ease;
    flex-shrink: 0;
  }

  .toggle.on {
    background: var(--accent);
    border-color: var(--accent);
  }

  .toggle-knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-tertiary);
    transition: transform 0.2s cubic-bezier(0.16, 1, 0.3, 1), background 0.2s ease;
  }

  .toggle.on .toggle-knob {
    transform: translateX(16px);
    background: var(--accent-contrast);
  }

  /* Footer */
  .drawer-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 24px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    gap: 12px;
  }

  .account-block {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

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

  .account-info {
    display: flex;
    flex-direction: column;
    gap: 1px;
    min-width: 0;
  }

  .account-name {
    font-size: 0.8125rem;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .account-role {
    font-size: 0.68rem;
    color: var(--text-tertiary);
  }

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

  .logout-btn:hover {
    color: var(--text);
    border-color: var(--border-strong);
    background: var(--bg-surface-hover);
  }

  .logout-btn svg {
    width: 14px;
    height: 14px;
  }
</style>
