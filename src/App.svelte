<script lang="ts">
  import { onMount } from 'svelte';
  import ParticleField from './lib/ParticleField.svelte';
  import IntroOverlay from './lib/IntroOverlay.svelte';
  import NicknameScreen from './lib/NicknameScreen.svelte';
  import LauncherScreen from './lib/LauncherScreen.svelte';
  import TitleBar from './lib/TitleBar.svelte';
  import { loadSettings, saveSettings } from './lib/tauri-api';
  import type { LauncherSettings } from './lib/tauri-api';

  type Stage = 'intro' | 'nickname' | 'launcher';
  let stage: Stage = 'intro';
  let nickname = '';
  let theme: 'dark' | 'light' = 'dark';
  let particleField: ParticleField;
  let settings: LauncherSettings | null = null;

  function setTheme(t: 'dark' | 'light') {
    theme = t;
    document.documentElement.setAttribute('data-theme', t);
    window.dispatchEvent(new Event('themechange'));
  }

  function toggleTheme() {
    setTheme(theme === 'dark' ? 'light' : 'dark');
  }

  function onIntroDone() {
    stage = 'nickname';
  }

  function onNicknameEnter(e: CustomEvent<{ nickname: string }>) {
    nickname = e.detail.nickname;
    stage = 'launcher';
    if (settings) {
      settings.last_nickname = nickname;
      saveSettings(settings);
    }
  }

  function onLogout() {
    nickname = '';
    if (settings) {
      settings.last_nickname = '';
      saveSettings(settings);
    }
    stage = 'nickname';
  }

  onMount(async () => {
    setTheme('dark');
    try {
      settings = await loadSettings();
      if (settings.last_nickname) {
        nickname = settings.last_nickname;
        stage = 'launcher';
      }
    } catch {
      settings = {
        ram_gb: 4,
        java_version: 'Java 17',
        window_width: '1280',
        window_height: '720',
        fullscreen: false,
        vsync: true,
        keep_open: false,
        debug_info: false,
        last_nickname: '',
        game_path: '',
      };
    }
  });
</script>

<div class="app-container">
  <TitleBar />
  <div class="app-content">
    <ParticleField bind:this={particleField} intensity={1} burst={stage === 'intro'} />

    {#if stage === 'intro'}
      <IntroOverlay on:done={onIntroDone} />
    {/if}

    {#if stage === 'nickname'}
      <NicknameScreen on:enter={onNicknameEnter} />
    {/if}

    {#if stage === 'launcher' && settings}
      <LauncherScreen {nickname} bind:settings on:logout={onLogout} />
    {/if}


  </div>
</div>

<style>
  .app-container {
    display: flex;
    flex-direction: column;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
  }

  .app-content {
    flex: 1;
    position: relative;
    overflow: hidden;
  }


</style>
