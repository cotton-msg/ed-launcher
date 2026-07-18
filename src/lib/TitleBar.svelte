<script lang="ts">
  import { windowMinimize, windowMaximize, windowClose, windowIsMaximized } from './tauri-api';

  let maximized = false;

  async function checkMaximized() {
    try {
      maximized = await windowIsMaximized();
    } catch (e) {
      console.error('windowIsMaximized failed:', e);
      maximized = false;
    }
  }

  async function handleMinimize() {
    try { await windowMinimize(); } catch (e) { console.error('windowMinimize failed:', e); }
  }

  async function handleMaximize() {
    try {
      await windowMaximize();
      await checkMaximized();
    } catch (e) { console.error('windowMaximize failed:', e); }
  }

  async function handleClose() {
    try { await windowClose(); } catch (e) { console.error('windowClose failed:', e); }
  }
</script>

<div class="titlebar" data-tauri-drag-region>
  <div class="titlebar-drag" data-tauri-drag-region>
    <span class="titlebar-title" data-tauri-drag-region>Grand Eden Launcher</span>
  </div>
  <div class="titlebar-buttons">
    <button class="titlebar-btn" on:click={handleMinimize} aria-label="Minimize">
      <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1">
        <line x1="1" y1="6" x2="11" y2="6" />
      </svg>
    </button>
    <button class="titlebar-btn" on:click={handleMaximize} aria-label="Maximize">
      {#if maximized}
        <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1">
          <rect x="3" y="3" width="7" height="7" rx="0.5" />
          <path d="M3 5V2.5A.5.5 0 0 1 3.5 2H6" />
        </svg>
      {:else}
        <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1">
          <rect x="1.5" y="1.5" width="9" height="9" rx="0.5" />
        </svg>
      {/if}
    </button>
    <button class="titlebar-btn close" on:click={handleClose} aria-label="Close">
      <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="1">
        <line x1="2" y1="2" x2="10" y2="10" />
        <line x1="10" y1="2" x2="2" y2="10" />
      </svg>
    </button>
  </div>
</div>

<style>
  .titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 32px;
    padding: 0 4px 0 12px;
    flex-shrink: 0;
    user-select: none;
    -webkit-user-select: none;
    background: transparent;
  }

  .titlebar-drag {
    flex: 1;
    display: flex;
    align-items: center;
    height: 100%;
  }

  .titlebar-title {
    font-size: 0.75rem;
    font-weight: 500;
    color: var(--text-tertiary);
    letter-spacing: 0.02em;
  }

  .titlebar-buttons {
    display: flex;
    align-items: center;
    gap: 2px;
    height: 100%;
  }

  .titlebar-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 28px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    border-radius: 6px;
    transition: background 0.15s ease, color 0.15s ease;
    cursor: pointer;
  }

  .titlebar-btn:hover {
    background: var(--bg-surface-hover);
    color: var(--text);
  }

  .titlebar-btn.close:hover {
    background: #e54848;
    color: #fff;
  }

  .titlebar-btn svg {
    width: 12px;
    height: 12px;
  }
</style>
