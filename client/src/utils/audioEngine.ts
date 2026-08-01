// Procedural audio engine (Web Audio API) — no external audio files, so there
// are no licensing concerns and nothing to download. Covers two needs:
//   1. UI sound effects (click / notification / tick) — short synthesized tones.
//   2. Generative ambient background music — a slow sub-drone plus randomly
//      scheduled soft pad notes drawn from a calm scale, in the spirit of
//      Eno-style generative ambient music. Never repeats identically.

let ctx: AudioContext | null = null;
let musicGain: GainNode | null = null;
let sfxGain: GainNode | null = null;

function getCtx(): AudioContext | null {
  if (typeof window === 'undefined') return null;
  const Ctor = window.AudioContext || (window as any).webkitAudioContext;
  if (!Ctor) return null;
  if (!ctx) {
    ctx = new Ctor();
    musicGain = ctx.createGain();
    musicGain.gain.value = 0.35;
    musicGain.connect(ctx.destination);
    sfxGain = ctx.createGain();
    sfxGain.gain.value = 0.5;
    sfxGain.connect(ctx.destination);
  }
  return ctx;
}

/** Must be called from within a user-gesture handler (click/keydown) — browsers
 * suspend AudioContext until one occurs. */
export function resumeAudio() {
  const c = getCtx();
  if (c && c.state === 'suspended') c.resume().catch(() => {});
}

export function setMusicVolume(v: number) {
  getCtx();
  if (musicGain) musicGain.gain.value = Math.max(0, Math.min(1, v));
}

export function setSfxVolume(v: number) {
  getCtx();
  if (sfxGain) sfxGain.gain.value = Math.max(0, Math.min(1, v));
}

function playTone(freq: number, duration: number, type: OscillatorType, peak: number, delay = 0) {
  const c = getCtx();
  if (!c || !sfxGain) return;
  const now = c.currentTime + delay;
  const osc = c.createOscillator();
  osc.type = type;
  osc.frequency.setValueAtTime(freq, now);
  const gain = c.createGain();
  gain.gain.setValueAtTime(0, now);
  gain.gain.linearRampToValueAtTime(peak, now + 0.008);
  gain.gain.exponentialRampToValueAtTime(0.0001, now + duration);
  osc.connect(gain);
  gain.connect(sfxGain);
  osc.start(now);
  osc.stop(now + duration + 0.02);
}

export function playClick() {
  playTone(720, 0.05, 'square', 0.18);
}

export function playNotification() {
  playTone(660, 0.14, 'sine', 0.22);
  playTone(880, 0.18, 'sine', 0.18, 0.09);
}

export function playTick() {
  playTone(300, 0.035, 'sine', 0.12);
}

/** ~3s alarm-style alert (alternating urgent tones) for a founder's death --
 * the one death that ends the experiment's entire premise, so it gets a
 * distinct, harder-to-miss sound instead of the ordinary playNotification()
 * chime used for milestones. */
export function playFounderDeathAlarm() {
  const beeps = 6;
  const beepDuration = 0.22;
  const gap = 0.28;
  for (let i = 0; i < beeps; i++) {
    const freq = i % 2 === 0 ? 880 : 660;
    playTone(freq, beepDuration, 'square', 0.3, i * gap);
  }
}

// Calm, open interval scale (low register) for the generative pad notes —
// avoids anything that reads as a "melody", stays texture-like.
const SCALE = [174.61, 196.0, 220.0, 261.63, 293.66, 329.63, 392.0];

let musicPlaying = false;
let musicTimer: ReturnType<typeof setTimeout> | null = null;
let stopDrone: (() => void) | null = null;

function scheduleNextPadNote() {
  if (!musicPlaying) return;
  const c = getCtx();
  if (!c || !musicGain) return;
  const freq = SCALE[Math.floor(Math.random() * SCALE.length)];
  const now = c.currentTime;
  const osc = c.createOscillator();
  osc.type = 'sine';
  osc.frequency.value = freq;
  const gain = c.createGain();
  gain.gain.setValueAtTime(0, now);
  gain.gain.linearRampToValueAtTime(0.16, now + 3);
  gain.gain.linearRampToValueAtTime(0, now + 9);
  osc.connect(gain);
  gain.connect(musicGain);
  osc.start(now);
  osc.stop(now + 9.5);
  musicTimer = setTimeout(scheduleNextPadNote, 6000 + Math.random() * 8000);
}

export function startMusic() {
  const c = getCtx();
  if (!c || !musicGain || musicPlaying) return;
  musicPlaying = true;
  resumeAudio();

  const drone = c.createOscillator();
  drone.type = 'sine';
  drone.frequency.value = SCALE[0] / 2;
  const droneGain = c.createGain();
  droneGain.gain.value = 0;
  droneGain.gain.linearRampToValueAtTime(0.24, c.currentTime + 3);
  const filter = c.createBiquadFilter();
  filter.type = 'lowpass';
  filter.frequency.value = 800;
  drone.connect(filter);
  filter.connect(droneGain);
  droneGain.connect(musicGain);
  drone.start();

  // Slow LFO on the filter cutoff so the drone breathes instead of sitting static.
  const lfo = c.createOscillator();
  lfo.frequency.value = 0.03;
  const lfoGain = c.createGain();
  lfoGain.gain.value = 250;
  lfo.connect(lfoGain);
  lfoGain.connect(filter.frequency);
  lfo.start();

  stopDrone = () => {
    const now = c.currentTime;
    droneGain.gain.linearRampToValueAtTime(0, now + 1.2);
    lfo.stop(now + 1.3);
    drone.stop(now + 1.3);
  };

  scheduleNextPadNote();
}

export function stopMusic() {
  musicPlaying = false;
  if (musicTimer) {
    clearTimeout(musicTimer);
    musicTimer = null;
  }
  stopDrone?.();
  stopDrone = null;
}

export function isMusicPlaying() {
  return musicPlaying;
}
