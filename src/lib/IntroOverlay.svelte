<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';

  const dispatch = createEventDispatcher<{ done: void }>();

  let container: HTMLElement;
  let logoText: HTMLElement;
  let line: HTMLElement;
  let subtitle: HTMLElement;
  let started = false;

  onMount(() => {
    // trigger entrance
    requestAnimationFrame(() => {
      started = true;
    });

    // after animation completes, fade out and signal done
    const total = 2600;
    const t = setTimeout(() => {
      dispatch('done');
    }, total);

    return () => clearTimeout(t);
  });
</script>

<div class="intro" class:started bind:this={container}>
  <div class="intro-content">
    <div class="logo-wrap" bind:this={logoText}>
      <img src="/logo.png" alt="Grand Eden" class="logo-img" />
    </div>
    <div class="line-wrap">
      <div class="line" bind:this={line} />
    </div>
    <div class="subtitle" bind:this={subtitle}>LAUNCHER</div>
  </div>
  <div class="intro-fade" />
</div>

<style>
  .intro {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg);
    transition: opacity 0.6s ease, visibility 0.6s ease;
  }

  .intro-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
  }

  .logo-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .logo-img {
    height: clamp(120px, 20vw, 200px);
    width: auto;
    opacity: 0;
    transform: translateY(40px) scale(0.9);
    transition: opacity 0.8s cubic-bezier(0.16, 1, 0.3, 1),
      transform 0.8s cubic-bezier(0.16, 1, 0.3, 1);
  }

  .started .logo-img {
    opacity: 1;
    transform: translateY(0) scale(1);
  }

  .line-wrap {
    height: 1px;
    width: 0;
    overflow: hidden;
    margin-top: 4px;
  }

  .line {
    height: 1px;
    width: 240px;
    background: var(--text);
    transform: scaleX(0);
    transform-origin: left;
    transition: transform 0.9s cubic-bezier(0.7, 0, 0.2, 1) 0.4s;
  }

  .started .line {
    transform: scaleX(1);
  }

  .subtitle {
    font-family: var(--font-sans);
    font-size: 0.75rem;
    font-weight: 500;
    letter-spacing: 0.6em;
    text-indent: 0.6em;
    color: var(--text-secondary);
    opacity: 0;
    transition: opacity 0.6s ease 0.9s;
  }

  .started .subtitle {
    opacity: 1;
  }

  .intro-fade {
    position: absolute;
    inset: 0;
    pointer-events: none;
    background: var(--bg);
    opacity: 0;
    transition: opacity 0.5s ease 2.1s;
  }

  .started .intro-fade {
    opacity: 1;
  }
</style>
