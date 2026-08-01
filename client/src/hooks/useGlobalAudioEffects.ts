import { useEffect, useRef } from 'react';
import { useSimStore } from '../store/simStore';
import { resumeAudio, setMusicVolume, setSfxVolume, startMusic, stopMusic, playClick } from '../utils/audioEngine';

/** Mounted once at the app root. Wires the sound settings (Ayarlar > Ses) to
 * the procedural audio engine: starts/stops generative ambient music, keeps
 * volumes in sync, and plays a click tone on any button/link press. */
export function useGlobalAudioEffects() {
  const soundSettings = useSimStore(s => s.soundSettings);
  const interacted = useRef(false);

  useEffect(() => {
    function onFirstInteract() {
      if (interacted.current) return;
      interacted.current = true;
      resumeAudio();
      if (useSimStore.getState().soundSettings.musicEnabled) startMusic();
    }
    document.addEventListener('pointerdown', onFirstInteract, { once: true });
    document.addEventListener('keydown', onFirstInteract, { once: true });
    return () => {
      document.removeEventListener('pointerdown', onFirstInteract);
      document.removeEventListener('keydown', onFirstInteract);
    };
  }, []);

  useEffect(() => { setMusicVolume(soundSettings.musicVolume); }, [soundSettings.musicVolume]);
  useEffect(() => { setSfxVolume(soundSettings.sfxVolume); }, [soundSettings.sfxVolume]);

  useEffect(() => {
    if (soundSettings.musicEnabled && interacted.current) startMusic();
    else stopMusic();
  }, [soundSettings.musicEnabled]);

  useEffect(() => {
    if (!soundSettings.clickEnabled) return;
    function onClick(e: MouseEvent) {
      const target = e.target as HTMLElement | null;
      if (target?.closest('button, [role="button"], a')) playClick();
    }
    document.addEventListener('click', onClick, true);
    return () => document.removeEventListener('click', onClick, true);
  }, [soundSettings.clickEnabled]);
}
