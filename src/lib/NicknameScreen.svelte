<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { fade } from 'svelte/transition';

  export let initialNickname = '';

  const dispatch = createEventDispatcher<{ enter: { nickname: string } }>();

  let nickname = initialNickname;
  let focused = false;
  let error = '';
  let shaking = false;

  function submit() {
    const trimmed = nickname.trim();
    if (trimmed.length < 3) {
      error = 'Минимум 3 символа';
      shaking = true;
      setTimeout(() => (shaking = false), 500);
      return;
    }
    if (trimmed.length > 16) {
      error = 'Максимум 16 символов';
      shaking = true;
      setTimeout(() => (shaking = false), 500);
      return;
    }
    if (!/^[a-zA-Z0-9_]+$/.test(trimmed)) {
      error = 'Только латиница, цифры и _';
      shaking = true;
      setTimeout(() => (shaking = false), 500);
      return;
    }
    error = '';
    dispatch('enter', { nickname: trimmed });
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') submit();
  }

  function onInput(e: Event) {
    nickname = (e.target as HTMLInputElement).value;
  }


</script>

<div class="nick-screen" in:fade>
  <div class="card" class:shake={shaking}>
    <div class="brand">
      <img src="/logo.png" alt="Grand Eden" class="brand-logo" />
    </div>

    <div class="heading">
      <h1>Добро пожаловать</h1>
      <p>Введите никнейм, чтобы начать играть</p>
    </div>

    <div class="field" class:focused>
      <label for="nick">Никнейм</label>
      <div class="input-row">
        <div class="input-wrap">
          <input
            id="nick"
            type="text"
            value={nickname}
            on:input={onInput}
            on:focus={() => (focused = true)}
            on:blur={() => (focused = false)}
            on:keydown={onKeydown}
            maxlength={16}
            autocomplete="off"
            spellcheck="false"
            placeholder="Steve"
          />
          <div class="char-overlay" aria-hidden="true">
            {#each [...nickname] as ch, i (i)}
              <span class="char" style="--idx: {i}">{ch}</span>
            {/each}
          </div>
        </div>
        <span class="counter">{nickname.length}/16</span>
      </div>
    </div>

    {#if error}
      <div class="error">{error}</div>
    {/if}

    <button class="enter-btn" on:click={submit} disabled={nickname.trim().length === 0}>
      <span>Продолжить</span>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M5 12h14M13 6l6 6-6 6" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
    </button>

    <div class="hint">
      Нажмите <kbd>Enter</kbd> чтобы продолжить
    </div>
  </div>
</div>

<style>
  .nick-screen {
    position: relative;
    z-index: 10;
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }

  .card {
    width: 100%;
    max-width: 420px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 20px;
    padding: 40px 36px 32px;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    gap: 24px;
    backdrop-filter: blur(20px);
  }

  .shake {
    animation: shake 0.4s ease;
  }

  @keyframes shake {
    0%, 100% { transform: translateX(0); }
    20% { transform: translateX(-8px); }
    40% { transform: translateX(8px); }
    60% { transform: translateX(-5px); }
    80% { transform: translateX(5px); }
  }

  .brand {
    display: flex;
    flex-direction: column;
    align-items: center;
    margin-bottom: 4px;
  }

  .brand-logo {
    height: 120px;
    width: auto;
  }

  .heading {
    text-align: center;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .heading h1 {
    font-family: var(--font-display);
    font-size: 1.5rem;
    font-weight: 600;
    letter-spacing: -0.01em;
  }

  .heading p {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .field label {
    font-size: 0.75rem;
    font-weight: 500;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-tertiary);
    transition: color 0.2s ease;
  }

  .field.focused label {
    color: var(--text-secondary);
  }

  .input-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 14px 16px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 12px;
    transition: border-color 0.2s ease, box-shadow 0.2s ease;
  }

  .input-wrap {
    position: relative;
    flex: 1;
    display: flex;
    align-items: center;
  }

  .input-wrap input {
    flex: 1;
    font-size: 1rem;
    font-weight: 500;
    letter-spacing: 0.01em;
    color: transparent;
    caret-color: var(--text);
    position: relative;
    z-index: 2;
  }

  .input-wrap input::placeholder {
    color: var(--text-tertiary);
  }

  .char-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    pointer-events: none;
    z-index: 1;
    font-size: 1rem;
    font-weight: 500;
    letter-spacing: 0.01em;
    color: var(--text);
  }

  .char {
    display: inline-block;
    animation: charPop 0.35s cubic-bezier(0.16, 1, 0.3, 1) both;
  }

  @keyframes charPop {
    0% {
      opacity: 0;
      transform: translateY(8px) scale(0.6);
      filter: blur(4px);
    }
    60% {
      opacity: 1;
      transform: translateY(-2px) scale(1.08);
      filter: blur(0);
    }
    100% {
      opacity: 1;
      transform: translateY(0) scale(1);
      filter: blur(0);
    }
  }

  .field.focused .input-row {
    border-color: var(--border-strong);
    box-shadow: 0 0 0 3px var(--border);
  }

  .input-row input::placeholder {
    color: var(--text-tertiary);
  }

  .counter {
    font-size: 0.75rem;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .error {
    font-size: 0.8125rem;
    color: #e54848;
    margin-top: -12px;
  }

  .enter-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 100%;
    padding: 14px 20px;
    background: var(--accent);
    color: var(--accent-contrast);
    border-radius: 12px;
    font-size: 0.9375rem;
    font-weight: 600;
    letter-spacing: 0.01em;
    transition: transform 0.15s ease, opacity 0.2s ease;
  }

  .enter-btn:hover:not(:disabled) {
    transform: translateY(-1px);
  }

  .enter-btn:active:not(:disabled) {
    transform: translateY(0);
  }

  .enter-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .enter-btn svg {
    width: 18px;
    height: 18px;
    transition: transform 0.2s ease;
  }

  .enter-btn:hover:not(:disabled) svg {
    transform: translateX(3px);
  }

  .hint {
    text-align: center;
    font-size: 0.75rem;
    color: var(--text-tertiary);
  }

  kbd {
    display: inline-block;
    padding: 2px 7px;
    border: 1px solid var(--border-strong);
    border-radius: 5px;
    font-family: var(--font-sans);
    font-size: 0.7rem;
    font-weight: 500;
    background: var(--bg-elevated);
  }
</style>
