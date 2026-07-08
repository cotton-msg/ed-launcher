<script lang="ts">
  import { onMount } from 'svelte';
  import ParticleField from './lib/ParticleField.svelte';
  import IntroOverlay from './lib/IntroOverlay.svelte';
  import NicknameScreen from './lib/NicknameScreen.svelte';
  import LauncherScreen from './lib/LauncherScreen.svelte';

  type Stage = 'intro' | 'nickname' | 'launcher';
  let stage: Stage = 'intro';
  let nickname = '';
  let theme: 'dark' | 'light' = 'dark';
  let particleField: ParticleField;

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
  }

  function onLogout() {
    nickname = '';
    stage = 'nickname';
  }

  onMount(() => {
    setTheme('dark');
  });
</script>

<ParticleField bind:this={particleField} intensity={1} burst={stage === 'intro'} />

{#if stage === 'intro'}
  <IntroOverlay on:done={onIntroDone} />
{/if}

{#if stage === 'nickname'}
  <NicknameScreen on:enter={onNicknameEnter} />
{/if}

{#if stage === 'launcher'}
  <LauncherScreen {nickname} on:logout={onLogout} />
{/if}

<!-- Theme toggle -->
{#if stage !== 'intro'}
  <button class="theme-toggle" on:click={toggleTheme} aria-label="Переключить тему">
    {#if theme === 'dark'}
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="4" />
        <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" stroke-linecap="round" />
      </svg>
    {:else}
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    {/if}
  </button>
{/if}

<style>
  .theme-toggle {
    position: fixed;
    top: 24px;
    right: 32px;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: 12px;
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--text-secondary);
    backdrop-filter: blur(12px);
    transition: color 0.2s ease, border-color 0.2s ease, transform 0.15s ease;
  }

  .theme-toggle:hover {
    color: var(--text);
    border-color: var(--border-strong);
    transform: translateY(-1px);
  }

  .theme-toggle svg {
    width: 18px;
    height: 18px;
  }

  @media (max-width: 640px) {
    .theme-toggle {
      top: 18px;
      right: 18px;
    }
  }
</style>
