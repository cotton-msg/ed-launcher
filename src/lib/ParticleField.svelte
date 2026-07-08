<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  export let intensity: number = 1;
  export let burst: boolean = false;

  let canvas: HTMLCanvasElement;
  let ctx: CanvasRenderingContext2D;
  let raf = 0;
  let particles: Particle[] = [];
  let bursts: Particle[] = [];
  let w = 0;
  let h = 0;
  let dpr = 1;
  let mouseX = 0;
  let mouseY = 0;
  let running = true;
  let time = 0;

  type Particle = {
    x: number;
    y: number;
    baseX: number;
    baseY: number;
    vx: number;
    vy: number;
    r: number;
    baseR: number;
    life: number;
    maxLife: number;
    alpha: number;
    phase: number;
    drift: number;
  };

  function getParticleColor() {
    const style = getComputedStyle(document.documentElement);
    return style.getPropertyValue('--particle').trim() || 'rgba(255,255,255,0.5)';
  }

  function parseColor(c: string) {
    const m = c.match(/rgba?\(([^)]+)\)/);
    if (!m) return [255, 255, 255, 0.5];
    const parts = m[1].split(',').map((p) => parseFloat(p.trim()));
    return [parts[0], parts[1], parts[2], parts[3] ?? 1];
  }

  let color: number[] = [255, 255, 255, 0.5];

  function refreshColor() {
    color = parseColor(getParticleColor());
  }

  function rand(min: number, max: number) {
    return Math.random() * (max - min) + min;
  }

  function createParticle(x?: number, y?: number, isBurst = false): Particle {
    const angle = rand(0, Math.PI * 2);
    const speed = isBurst ? rand(0.3, 2.5) : rand(0.02, 0.12);
    const px = x ?? rand(0, w);
    const py = y ?? rand(0, h);
    return {
      x: px,
      y: py,
      baseX: px,
      baseY: py,
      vx: Math.cos(angle) * speed,
      vy: Math.sin(angle) * speed,
      r: isBurst ? rand(0.8, 2) : rand(0.5, 1.8),
      baseR: rand(0.5, 1.8),
      life: 0,
      maxLife: isBurst ? rand(80, 180) : rand(400, 900),
      alpha: 0,
      phase: rand(0, Math.PI * 2),
      drift: rand(0.3, 0.8),
    };
  }

  function initParticles() {
    const count = Math.floor((w * h) / 28000 * intensity);
    particles = [];
    for (let i = 0; i < count; i++) {
      particles.push(createParticle());
    }
  }

  function resize() {
    dpr = Math.min(window.devicePixelRatio || 1, 2);
    w = window.innerWidth;
    h = window.innerHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    canvas.style.width = w + 'px';
    canvas.style.height = h + 'px';
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    initParticles();
  }

  function doBurst(x: number, y: number, count = 40) {
    for (let i = 0; i < count; i++) {
      bursts.push(createParticle(x, y, true));
    }
  }

  let lastBurst = 0;
  function maybeBurst() {
    if (!burst) return;
    const now = performance.now();
    if (now - lastBurst > 600) {
      lastBurst = now;
      doBurst(rand(w * 0.25, w * 0.75), rand(h * 0.25, h * 0.75), 30);
    }
  }

  function step(p: Particle, isBurst = false) {
    p.x += p.vx;
    p.y += p.vy;
    p.life++;
    const t = p.life / p.maxLife;

    if (isBurst) {
      p.alpha = Math.sin(t * Math.PI) * 0.6;
      p.vx *= 0.97;
      p.vy *= 0.97;
    } else {
      // slow gentle floating with sine wave drift
      p.alpha = (Math.sin(time * 0.001 * p.drift + p.phase) * 0.5 + 0.5) * 0.35;
      // subtle drift toward mouse, very gentle
      const dx = mouseX - p.x;
      const dy = mouseY - p.y;
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (dist < 160 && dist > 0) {
        const force = (1 - dist / 160) * 0.015;
        p.vx += (dx / dist) * force;
        p.vy += (dy / dist) * force;
      }
      p.vx *= 0.992;
      p.vy *= 0.992;
      // breathing radius
      p.r = p.baseR * (0.8 + Math.sin(time * 0.002 + p.phase) * 0.2);
    }
  }

  function drawParticle(p: Particle) {
    const a = Math.max(0, p.alpha) * color[3];
    if (a < 0.01) return;
    // soft glow
    const glow = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, p.r * 4);
    glow.addColorStop(0, `rgba(${color[0]},${color[1]},${color[2]},${a * 0.8})`);
    glow.addColorStop(0.4, `rgba(${color[0]},${color[1]},${color[2]},${a * 0.2})`);
    glow.addColorStop(1, `rgba(${color[0]},${color[1]},${color[2]},0)`);
    ctx.fillStyle = glow;
    ctx.beginPath();
    ctx.arc(p.x, p.y, p.r * 4, 0, Math.PI * 2);
    ctx.fill();
    // core
    ctx.beginPath();
    ctx.arc(p.x, p.y, p.r, 0, Math.PI * 2);
    ctx.fillStyle = `rgba(${color[0]},${color[1]},${color[2]},${a})`;
    ctx.fill();
  }

  function loop() {
    if (!running) return;
    time = performance.now();
    ctx.clearRect(0, 0, w, h);
    maybeBurst();

    for (const p of particles) {
      step(p);
      drawParticle(p);
      if (p.life >= p.maxLife) {
        Object.assign(p, createParticle());
      }
    }

    bursts = bursts.filter((p) => p.life < p.maxLife);
    for (const p of bursts) {
      step(p, true);
      drawParticle(p);
    }

    raf = requestAnimationFrame(loop);
  }

  function onMouseMove(e: MouseEvent) {
    mouseX = e.clientX;
    mouseY = e.clientY;
  }

  function onThemeChange() {
    refreshColor();
  }

  onMount(() => {
    ctx = canvas.getContext('2d')!;
    refreshColor();
    resize();
    mouseX = w / 2;
    mouseY = h / 2;
    loop();

    window.addEventListener('resize', resize);
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('themechange', onThemeChange);
  });

  onDestroy(() => {
    running = false;
    cancelAnimationFrame(raf);
    window.removeEventListener('resize', resize);
    window.removeEventListener('mousemove', onMouseMove);
    window.removeEventListener('themechange', onThemeChange);
  });

  export function triggerBurst(x: number, y: number, count = 50) {
    doBurst(x, y, count);
  }
</script>

<canvas bind:this={canvas} class="particle-canvas" />

<style>
  .particle-canvas {
    position: fixed;
    inset: 0;
    z-index: 0;
    pointer-events: none;
  }
</style>
