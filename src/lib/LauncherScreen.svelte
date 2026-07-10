<script lang="ts">
  import { createEventDispatcher, onMount, onDestroy } from 'svelte';
  import SettingsDrawer from './SettingsDrawer.svelte';
  import { checkServerStatus, downloadGame, launchGame, onDownloadProgress } from './tauri-api';
  import { open } from '@tauri-apps/plugin-shell';
  import type { LauncherSettings, ServerStatus, DownloadProgress } from './tauri-api';

  export let nickname: string;
  export let settings: LauncherSettings;

  const dispatch = createEventDispatcher<{ logout: void }>();

  let settingsOpen = false;
  let serverStatus: ServerStatus = {
    online: false,
    players: 0,
    max_players: 500,
    version: '1.20.4',
    ping_ms: 0,
    tps: 0,
  };

  type Status = 'idle' | 'downloading' | 'playing';
  let status: Status = 'idle';
  let progress = 0;
  let currentFile = '';
  let speed = 0;
  let downloadedMB = 0;
  let totalMB = 0;
  let error = '';

  let statusInterval: ReturnType<typeof setInterval> | null = null;
  let unlistenProgress: (() => void) | null = null;

  async function refreshStatus() {
    try {
      serverStatus = await checkServerStatus();
    } catch {}
  }

  onMount(async () => {
    try {
      const fs = await import('@tauri-apps/plugin-fs');
      const logPath = 'C:\\Users\\supminer\\.minecraft\\onmount_debug.log';
      await fs.writeTextFile(logPath, `onMount started\nstatus: ${status}\n`);
    } catch (e) {
      console.error('Failed to log onMount:', e);
    }
    
    await refreshStatus();
    statusInterval = setInterval(refreshStatus, 30000);

    unlistenProgress = await onDownloadProgress((p: DownloadProgress) => {
      progress = p.progress;
      currentFile = p.file;
      downloadedMB = p.downloaded_mb;
      totalMB = p.total_mb;
      speed = p.speed_mb_s;
    });
  });

  onDestroy(() => {
    if (statusInterval) clearInterval(statusInterval);
    if (unlistenProgress) unlistenProgress();
  });

  async function play() {
    error = '';
    if (status === 'idle') {
      await startDownload();
    } else if (status === 'playing') {
      status = 'idle';
      progress = 0;
      currentFile = '';
      speed = 0;
      downloadedMB = 0;
      totalMB = 0;
    }
  }

  async function startDownload() {
    const log = async (msg: string) => {
      try {
        const fs = await import('@tauri-apps/plugin-fs');
        const logPath = 'C:\\Users\\supminer\\.minecraft\\frontend_debug.log';
        const time = new Date().toISOString();
        await fs.appendTextFile(logPath, `[${time}] ${msg}\n`);
      } catch (e) {
        console.error('Failed to log:', e);
      }
    };

    await log('startDownload called');
    status = 'downloading';
    progress = 0;
    downloadedMB = 0;
    totalMB = 0;
    currentFile = '';

    try {
      await log('Calling downloadGame...');
      const success = await downloadGame(settings.game_path);
      await log(`downloadGame result: ${success}`);
      if (success) {
        status = 'playing';
        try {
          await log('Calling launchGame...');
          await launchGame(nickname, settings);
          await log('launchGame completed');
        } catch (e) {
          await log(`launchGame error: ${e}`);
          error = `Failed to launch: ${e}`;
          status = 'idle';
        }
      }
    } catch (e) {
      await log(`downloadGame error: ${e}`);
      error = `Download failed: ${e}`;
      status = 'idle';
    }
  }

  function formatMB(mb: number) {
    if (mb < 1 && mb > 0) return mb.toFixed(2);
    if (mb === 0) return '0';
    return mb.toFixed(1);
  }

  function logout() {
    dispatch('logout');
  }

  function openSettings() {
    settingsOpen = true;
  }

  async function openLink() {
    try {
      await open('https://t.me/supminerr');
    } catch {}
  }

  function closeSettings() {
    settingsOpen = false;
  }
</script>

<div class="launcher" in:fade>
  <main class="main">
    <div class="hero">
      <div class="hero-text">
        <img src="/logo.png" alt="Grand Eden" class="hero-logo" />
      </div>
      <div class="hero-meta">
        <div class="meta-item">
          <span class="meta-label">Version</span>
          <span class="meta-value">1.20.1</span>
        </div>
        <div class="meta-item">
          <span class="meta-label">Ping</span>
          <span class="meta-value">{serverStatus.ping_ms > 0 ? `${serverStatus.ping_ms} ms` : '—'}</span>
        </div>
        <div class="meta-item">
          <span class="meta-label">TPS</span>
          <span class="meta-value">{serverStatus.tps > 0 ? serverStatus.tps.toFixed(1) : '—'}</span>
        </div>
      </div>
    </div>

    <div class="play-section">
      {#if error}
        <div class="error-msg">{error}</div>
      {/if}

      {#if status === 'idle'}
        <button class="play-btn" on:click={play}>
          <svg viewBox="0 0 24 24" fill="currentColor">
            <path d="M8 5v14l11-7z" />
          </svg>
          <span>Play</span>
        </button>
        <div class="play-sub">Forge 1.20.1 · Ready to launch</div>
      {:else if status === 'downloading'}
        <button class="play-btn downloading" disabled>
          <div class="spinner" />
          <span>Downloading...</span>
        </button>
        <div class="download-area">
          <div class="progress-bar">
            <div class="progress-fill" style="width: {progress}%" />
          </div>
          <div class="progress-info">
            <span class="progress-file" title={currentFile}>{currentFile}</span>
            <span class="progress-percent">{progress.toFixed(1)}%</span>
          </div>
          <div class="progress-stats">
            <span>{formatMB(downloadedMB)} / {formatMB(totalMB)} MB</span>
            <span>{speed.toFixed(1)} MB/s</span>
          </div>
        </div>
      {:else if status === 'playing'}
        <button class="play-btn playing" on:click={play}>
          <svg viewBox="0 0 24 24" fill="currentColor">
            <rect x="6" y="5" width="4" height="14" rx="1" />
            <rect x="14" y="5" width="4" height="14" rx="1" />
          </svg>
          <span>Launched</span>
        </button>
        <div class="play-sub">Game is running — have fun!</div>
      {/if}
    </div>
  </main>

  <footer class="bottombar">
    <div class="user" on:click={openSettings} role="button" tabindex="0" on:keydown={(e) => e.key === 'Enter' && openSettings()}>
      <div class="avatar">{nickname.charAt(0).toUpperCase()}</div>
      <div class="user-info">
        <span class="user-name">{nickname}</span>
        <span class="user-role">Player</span>
      </div>
      <svg class="settings-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </div>
    <a class="credit" href="https://t.me/supminerr" on:click={openLink}>by supminer</a>
  </footer>
</div>

<SettingsDrawer
  bind:open={settingsOpen}
  bind:settings={settings}
  {nickname}
  on:close={closeSettings}
  on:logout={logout}
/>

<script context="module" lang="ts">
  import { fade } from 'svelte/transition';
</script>

<style>
  .launcher {
    position: relative;
    z-index: 10;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    padding: 0 32px 24px;
  }

  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 48px;
    padding: 16px 0;
  }

  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 32px;
    text-align: center;
    max-width: 600px;
  }

  .hero-logo {
    height: clamp(80px, 12vw, 140px);
    width: auto;
  }

  .hero-meta { display: flex; gap: 32px; }

  .meta-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .meta-label {
    font-size: 0.7rem;
    font-weight: 500;
    letter-spacing: 0.1em;
    text-transform: uppercase;
    color: var(--text-tertiary);
  }

  .meta-value {
    font-family: var(--font-display);
    font-size: 1.125rem;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .play-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    width: 100%;
    max-width: 480px;
  }

  .error-msg {
    font-size: 0.8125rem;
    color: #e54848;
    text-align: center;
    padding: 8px 16px;
    background: rgba(229, 72, 72, 0.1);
    border-radius: 8px;
    width: 100%;
  }

  .play-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    width: 100%;
    padding: 18px 32px;
    background: var(--accent);
    color: var(--accent-contrast);
    border-radius: 14px;
    font-family: var(--font-display);
    font-size: 1.125rem;
    font-weight: 600;
    letter-spacing: 0.02em;
    transition: transform 0.15s ease, box-shadow 0.2s ease, opacity 0.2s ease;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
  }

  .play-btn:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.3);
  }

  .play-btn:active:not(:disabled) { transform: translateY(0); }
  .play-btn svg { width: 22px; height: 22px; }
  .play-btn.downloading { opacity: 0.7; cursor: default; }

  .play-btn.playing {
    background: transparent;
    border: 1px solid var(--border-strong);
    color: var(--text);
    box-shadow: none;
  }

  .play-btn.playing:hover { background: var(--bg-surface-hover); }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid var(--accent-contrast);
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    opacity: 0.5;
  }

  @keyframes spin { to { transform: rotate(360deg); } }

  .play-sub { font-size: 0.8125rem; color: var(--text-tertiary); }

  .download-area { width: 100%; display: flex; flex-direction: column; gap: 8px; }

  .progress-bar {
    width: 100%;
    height: 4px;
    background: var(--bg-elevated);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.12s linear;
  }

  .progress-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
  }

  .progress-file {
    color: var(--text-tertiary);
    font-family: monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 70%;
  }

  .progress-percent { color: var(--text); font-weight: 600; font-variant-numeric: tabular-nums; }

  .progress-stats {
    display: flex;
    justify-content: space-between;
    font-size: 0.7rem;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
  }

  .bottombar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
    padding: 8px 0;
  }

  .user {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 10px 6px 6px;
    border-radius: 12px;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .user:hover { background: var(--bg-surface-hover); }

  .settings-icon {
    width: 16px;
    height: 16px;
    color: var(--text-tertiary);
    transition: color 0.15s ease, transform 0.3s ease;
  }

  .user:hover .settings-icon { color: var(--text); transform: rotate(45deg); }

  .avatar {
    width: 38px;
    height: 38px;
    border-radius: 10px;
    background: var(--accent);
    color: var(--accent-contrast);
    display: flex;
    align-items: center;
    justify-content: center;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 1.125rem;
  }

  .user-info { display: flex; flex-direction: column; gap: 1px; }
  .user-name { font-size: 0.875rem; font-weight: 600; }
  .user-role { font-size: 0.7rem; color: var(--text-tertiary); }

  .credit {
    position: fixed;
    bottom: 16px;
    right: 24px;
    font-size: 0.75rem;
    color: var(--text-tertiary);
    font-weight: 500;
    text-decoration: none;
    cursor: pointer;
    transition: color 0.2s ease;
  }

  .credit:hover {
    color: var(--text);
  }

  @media (max-width: 640px) {
    .launcher { padding: 0 20px 20px; }
    .hero-meta { gap: 20px; }
  }
</style>
