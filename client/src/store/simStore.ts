import { create } from 'zustand';
import axios from 'axios';
import { LANG_CODES, isValidLangCode } from '../utils/i18n';
import { authUrl } from '../utils/cloud';

const LANG_ORDER = LANG_CODES;
const AUTH_USER_KEY = 'anatolia_auth_user';
const AUTH_TOKEN_KEY = 'anatolia_auth_token';

function safeStorageSet(storage: Storage, key: string, value: string) {
  try { storage.setItem(key, value); } catch {}
}

function safeStorageRemove(storage: Storage, key: string) {
  try { storage.removeItem(key); } catch {}
}

// 'anatolia_session_active' living in localStorage (vs sessionStorage) is
// already the single source of truth LoginPage.tsx sets for "remember me"
// at the moment of login -- reused here rather than threading a separate
// flag through setUser's every call site (fresh login, bridge handoff,
// stored-session rehydration, and three different token-refresh flows).
function isRememberedSession(): boolean {
  try { return localStorage.getItem('anatolia_session_active') === '1'; } catch { return false; }
}

function persistAuth(user: SimStore['user'], token: string) {
  const payload = JSON.stringify(user);
  // Session-only storage always gets the fresh token/user, so the current
  // tab stays logged in for its own lifetime regardless of remember choice.
  safeStorageSet(sessionStorage, AUTH_USER_KEY, payload);
  safeStorageSet(sessionStorage, AUTH_TOKEN_KEY, token);
  if (isRememberedSession()) {
    safeStorageSet(localStorage, AUTH_USER_KEY, payload);
    safeStorageSet(localStorage, AUTH_TOKEN_KEY, token);
  } else {
    // A user who explicitly did NOT check "remember me" must not have their
    // access token persist across browser restarts -- previously this ran
    // unconditionally, so the checkbox controlled nothing about where the
    // actual token landed (only a separate, unrelated prefill-the-user-code
    // convenience flag), and a stale localStorage token from an earlier
    // remembered login could otherwise linger and silently auto-log this
    // session back in on next launch.
    safeStorageRemove(localStorage, AUTH_USER_KEY);
    safeStorageRemove(localStorage, AUTH_TOKEN_KEY);
  }
}

function clearPersistedAuth() {
  safeStorageRemove(localStorage, AUTH_USER_KEY);
  safeStorageRemove(sessionStorage, AUTH_USER_KEY);
  safeStorageRemove(localStorage, AUTH_TOKEN_KEY);
  safeStorageRemove(sessionStorage, AUTH_TOKEN_KEY);
}

function getSavedLang() {
  try {
    const saved = localStorage.getItem('anatolia_lang');
    return isValidLangCode(saved) ? saved : 'en';
  } catch {
    return 'en';
  }
}

const SOUND_SETTINGS_KEY = 'anatolia_sound_settings';

interface SoundSettings {
  musicEnabled: boolean;
  musicVolume: number;
  clickEnabled: boolean;
  notificationEnabled: boolean;
  tickEnabled: boolean;
  sfxVolume: number;
}

// Music defaults to off (autoplay policies require a gesture anyway, and a
// first-time user shouldn't be surprised by sudden background audio); the
// short UI/notification/tick effects default to on since they're subtle.
const DEFAULT_SOUND_SETTINGS: SoundSettings = {
  musicEnabled: false,
  musicVolume: 0.35,
  clickEnabled: true,
  notificationEnabled: true,
  tickEnabled: true,
  sfxVolume: 0.5,
};

function getSavedSoundSettings(): SoundSettings {
  try {
    const raw = localStorage.getItem(SOUND_SETTINGS_KEY);
    if (!raw) return DEFAULT_SOUND_SETTINGS;
    return { ...DEFAULT_SOUND_SETTINGS, ...JSON.parse(raw) };
  } catch {
    return DEFAULT_SOUND_SETTINGS;
  }
}

const GLOBE_AUTO_ROTATE_KEY = 'anatolia_globe_auto_rotate';

// Defaults to on, matching the globe's existing always-spinning behavior --
// this is an opt-out for users who want the view to hold still (e.g. to
// track individuals by dragging to a fixed angle) rather than a new default.
function getSavedGlobeAutoRotate(): boolean {
  try {
    const raw = localStorage.getItem(GLOBE_AUTO_ROTATE_KEY);
    return raw === null ? true : raw === '1';
  } catch {
    return true;
  }
}

interface WorldState {
  latitude: number;
  longitude: number;
  biome: string;
  temperature: number;
  food_abundance: number;
  water_abundance: number;
  season: 'spring' | 'summer' | 'autumn' | 'winter';
  human_impact?: number;
  predator_risk?: number;
  current_weather?: string;
  weather_intensity?: number;
  phonology_seed?: number;
  phoneme_palette?: { consonants: string[]; vowels: string[] };
  fauna?: { prey_density?: number; predator_density?: number };
  flora?: { density?: number };
  alive_count?: number;
  recent_disaster?: boolean;
  [key: string]: unknown;
}

interface SimStats {
  day: number;
  year: number;
  hour?: number;
  population: number;
  avg_age: number;
  sex_ratio: number;
  avg_intelligence: number;
  technologies: number;
  season: string;
  temperature: number;
  food_abundance: number;
  beliefs: number;
  art_forms: number;
  groups: number;
  gini: number;
  happiness_index: number;
  sick_rate: number;
  mean_wealth: number;
  total_ever: number;
  water_abundance?: number;
  biome?: string;
  has_disaster?: boolean;
  births?: number;
  deaths?: number;
  word_count?: number;
  max_language_stage?: number;
  avg_consciousness?: number;
  avg_cultural_prestige?: number;
  max_tom_stage?: number;
  tech_progress?: Record<string, number>;
  qol_index?: number;
  social_order?: number;
  astronomy_knowledge?: number;
  weather?: string;
  total_techs?: number;
  allele_frequencies?: Record<string, number>;
  centroid_x?: number | null;
  centroid_y?: number | null;
  mean_stress?: number;
  mental_state_distribution?: Record<string, number>;
  total_population?: number;
  age_pyramid?: { group: string; male: number; female: number }[];
  epigenetics?: Record<string, number>;
  civilization_name?: string | null;
  events_count?: number;
  language_stage_distribution?: Record<string, number>;
  dominant_drive?: string | null;
  pathogen_diversity?: number;
  speed_multiplier?: number;
  genetic_diversity?: {
    avg_heterozygosity: number;
    allelic_variance: number;
    effective_population_size: number;
    avg_inbreeding_coefficient: number;
  };
  genetic_diversity_by_group?: Record<string, {
    avg_heterozygosity: number;
    allelic_variance: number;
    effective_population_size: number;
    avg_inbreeding_coefficient: number;
  }>;
  vocabulary_by_group?: Record<string, Record<string, string>>;
}

// Feature 1: centroid trail point
export interface CentroidPoint { x: number; y: number; day: number; }

// Feature 11: milestone event
export interface MilestoneEvent {
  key: string;
  description: string;
  icon: string;
  day: number;
}

// Feature 14: runtime performance metrics
export interface RuntimeMetrics {
  tick_avg_ms: number;
  tick_max_ms: number;
  tick_min_ms: number;
  tick_last_ms: number | null;
  ticks_per_second: number;
  tick_load_ms?: number | null;
  tick_compute_ms?: number | null;
  tick_save_ms?: number | null;
  tick_upsert_ms?: number | null;
  tick_phase_setup_ms?: number | null;
  tick_phase_economy_ms?: number | null;
  tick_phase_consciousness_psychology_ms?: number | null;
  tick_phase_language_naming_ms?: number | null;
  tick_phase_microbiome_agent_ms?: number | null;
  tick_phase_movement_ms?: number | null;
  tick_phase_observation_learning_ms?: number | null;
  tick_phase_tech_emergence_ms?: number | null;
  tick_phase_reproduction_ms?: number | null;
  tick_phase_mortality_roll_ms?: number | null;
  tick_phase_microbiome_outbreak_ms?: number | null;
  tick_phase_group_pruning_ms?: number | null;
  tick_phase_belief_ms?: number | null;
  tick_phase_culture_art_ms?: number | null;
  tick_phase_social_ms?: number | null;
  tick_phase_law_ms?: number | null;
  tick_phase_architecture_conflict_ms?: number | null;
  tick_phase_astronomy_ms?: number | null;
  tick_phase_trade_disease_ms?: number | null;
  disabled_engines?: string[];
  speed_multiplier: number;
  population: number;
  total_ever: number;
  current_day: number;
  milestones_reached: string[];
  centroid_trail: CentroidPoint[];
  fast_forward_target: number | null;
  is_warping: boolean;
  upload_paused?: boolean;
  status?: 'running' | 'paused' | 'completed';
  heavy_mode: boolean;
  cpu_cores_available?: number;
  cpu_cores_used?: number;
  cross_origin_isolated?: boolean;
  thread_pool_error?: string | null;
}

interface SimEvent {
  id?: string;
  data?: Record<string, any>;
  sim_day: number;
  sim_year: number;
  event_type: string;
  description: string;
  importance: number;
}

interface Simulation {
  id: string;
  name: string;
  status: 'running' | 'paused' | 'completed';
  current_day: number;
  current_year: number;
  total_ever?: number;
  population?: number;
  start_latitude: number;
  start_longitude: number;
  speed_multiplier?: number;
  world_state?: WorldState;
}

export interface Moment {
  id: string;
  day: number;
  year: number;
  icon: string;
  title: string;
  description?: string;
  color: string;
}

interface SimStore {
  // Auth
  user: { id: string; username: string; email: string; role: string; first_name?: string; last_name?: string; tc_no?: string | null; nickname?: string | null } | null;
  accessToken: string | null;
  setUser: (user: SimStore['user'], token: string) => void;
  logout: () => void;

  // Current simulation
  currentSim: Simulation | null;
  setCurrentSim: (sim: Simulation | null) => void;

  // Live stats from WebSocket
  stats: SimStats | null;
  events: SimEvent[];
  setStats: (stats: SimStats) => void;
  setStatsDay: (day: number) => void;
  addEvent: (event: SimEvent) => void;
  setEvents: (events: SimEvent[]) => void;
  resetLiveState: () => void;

  // Natural simulation end
  simulationEnded: string | null;
  setSimulationEnded: (reason: string) => void;
  clearSimulationEnded: () => void;

  // Moments gallery
  moments: Moment[];
  addMoment: (m: Omit<Moment, 'id'>) => void;
  clearMoments: () => void;

  // Witness mode
  watchedIndividualId: string | null;
  setWatchedIndividual: (id: string | null) => void;

  // UI state
  activePanel: string | null;
  setActivePanel: (panel: string | null) => void;
  lang: 'en' | 'tr' | 'de' | 'fr' | 'ar';
  setLang: (l: 'en' | 'tr' | 'de' | 'fr' | 'ar') => void;
  toggleLang: () => void;
  theme: 'dark' | 'light';
  toggleTheme: () => void;
  soundSettings: SoundSettings;
  setSoundSettings: (patch: Partial<SoundSettings>) => void;
  globeAutoRotate: boolean;
  setGlobeAutoRotate: (enabled: boolean) => void;
  settingsOpen: boolean;
  setSettingsOpen: (open: boolean) => void;
  speedMultiplier: number;
  setSpeed: (speed: number) => void;
  sidebarExpanded: boolean;
  toggleSidebar: () => void;

  // Fast-forward / warp mode (Feature 5)
  isWarping: boolean;
  fastForwardTarget: number | null;
  setIsWarping: (w: boolean) => void;
  setFastForwardTarget: (t: number | null) => void;

  // Centroid migration trail (Feature 1)
  centroidTrail: CentroidPoint[];
  setCentroidTrail: (trail: CentroidPoint[]) => void;

  // Live WS connection status -- surfaced in the Performance panel so a
  // "day counter looks frozen" report can be self-diagnosed (is the socket
  // even open? when did it last hear from the server?) without needing a
  // USB-attached remote debugger, which isn't an option for every user.
  wsStatus: 'connecting' | 'open' | 'closed' | 'error';
  wsLastMessageAt: number | null;
  wsCloseInfo: { code: number; reason: string } | null;
  wsReconnectCount: number;
  setWsStatus: (s: 'connecting' | 'open' | 'closed' | 'error') => void;
  setWsLastMessageAt: (t: number | null) => void;
  setWsCloseInfo: (info: { code: number; reason: string } | null) => void;
  incrementWsReconnectCount: () => void;

  // Milestone events (Feature 11)
  milestones: MilestoneEvent[];
  addMilestone: (m: MilestoneEvent) => void;
  clearMilestones: () => void;

  // Engine performance metrics (Feature 14)
  runtimeMetrics: RuntimeMetrics | null;
  setRuntimeMetrics: (m: RuntimeMetrics | null) => void;

  // Desktop shell update state
  updatePercent: number | null;
  updateReady: { version?: string } | null;
  updateInstall: (() => Promise<void>) | null;
  setUpdatePercent: (p: number | null) => void;
  setUpdateReady: (info: { version?: string } | null) => void;
  setUpdateInstall: (install: (() => Promise<void>) | null) => void;
}

function normalizeEventKey(event: SimEvent) {
  return event.id ?? [
    event.sim_day,
    event.sim_year,
    event.event_type ?? '',
    event.description ?? '',
    JSON.stringify(event.data ?? {}),
  ].join('|');
}

const EVENT_CAP = 200;
// A large tick batch at high sim speed (runtime.rs paces ~1s per batch,
// batch_size = speed multiplier -- e.g. speed=100 means up to 100 simulated
// days' worth of events land in a single WS delivery) can easily push more
// than EVENT_CAP *unrelated* events (thoughts, discoveries, births, ...)
// alongside a handful of death events within that same batch. A flat
// slice(0, EVENT_CAP) on the merged, newest-first array would silently
// evict an older death from that very batch before the user ever sees the
// Events panel's "Ölüm"/Death filter -- so death events get their own,
// separate cap instead of sharing the general one.
const PROTECTED_EVENT_TYPES = new Set(['death']);
const PROTECTED_EVENT_CAP = 300;

function capEvents(events: SimEvent[]): SimEvent[] {
  if (events.length <= EVENT_CAP) return events;
  let protectedCount = 0;
  let restCount = 0;
  const kept: SimEvent[] = [];
  for (const event of events) {
    if (PROTECTED_EVENT_TYPES.has(event.event_type)) {
      if (protectedCount >= PROTECTED_EVENT_CAP) continue;
      protectedCount++;
    } else {
      if (restCount >= EVENT_CAP) continue;
      restCount++;
    }
    kept.push(event);
  }
  return kept;
}

export const useSimStore = create<SimStore>((set) => ({
  user: null,
  accessToken: null,
  setUser: (user, token) => {
    persistAuth(user, token);
    set({ user, accessToken: token });
  },
  logout: () => {
    // The refresh_token cookie is httpOnly, so it can only be cleared by the
    // server -- without this call it survives past local state being wiped,
    // and App.tsx's own refresh-on-load call would silently sign the user
    // back in on their next visit despite them having "logged out".
    axios.post(authUrl('/api/auth/logout'), undefined, { withCredentials: true }).catch(() => {});
    try { sessionStorage.removeItem('anatolia_session_active'); } catch {}
    try { localStorage.removeItem('anatolia_session_active'); } catch {}
    clearPersistedAuth();
    set({ user: null, accessToken: null, currentSim: null });
  },

  currentSim: null,
  setCurrentSim: (sim) => set({ currentSim: sim }),

  stats: null,
  events: [],
  setStats: (stats) => set({ stats }),
  setStatsDay: (day) => set(s => (s.stats ? { stats: { ...s.stats, day } } : s)),
  addEvent: (event) => set(s => {
    const key = normalizeEventKey(event);
    if (s.events.some(existing => normalizeEventKey(existing) === key)) return s;
    return { events: capEvents([event, ...s.events]) };
  }),
  setEvents: (events) => {
    const seen = new Set<string>();
    const deduped = events.filter(event => {
      const key = normalizeEventKey(event);
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
    set({ events: capEvents(deduped) });
  },
  resetLiveState: () => set({ stats: null, events: [], simulationEnded: null, milestones: [], centroidTrail: [], isWarping: false, fastForwardTarget: null, runtimeMetrics: null }),

  simulationEnded: null,
  setSimulationEnded: (reason) => set({ simulationEnded: reason }),
  clearSimulationEnded: () => set({ simulationEnded: null }),

  moments: [],
  addMoment: (m) => set(s => {
    const id = Math.random().toString(36).slice(2);
    return { moments: [{ ...m, id }, ...s.moments].slice(0, 100) };
  }),
  clearMoments: () => set({ moments: [] }),

  watchedIndividualId: null,
  setWatchedIndividual: (id) => set({ watchedIndividualId: id }),

  activePanel: null,
  setActivePanel: (panel) => set(s => ({ activePanel: s.activePanel === panel ? null : panel })),
  lang: getSavedLang(),
  setLang: (l) => { localStorage.setItem('anatolia_lang', l); set({ lang: l }); },
  toggleLang: () => set(s => {
    const currentIndex = LANG_ORDER.indexOf((s.lang ?? 'en') as typeof LANG_ORDER[number]);
    const nextLang = LANG_ORDER[(currentIndex + 1) % LANG_ORDER.length] ?? 'en';
    localStorage.setItem('anatolia_lang', nextLang);
    return { lang: nextLang };
  }),
  theme: 'dark',
  toggleTheme: () => set(s => ({ theme: s.theme === 'dark' ? 'light' : 'dark' })),
  soundSettings: getSavedSoundSettings(),
  setSoundSettings: (patch) => set(s => {
    const next = { ...s.soundSettings, ...patch };
    try { localStorage.setItem(SOUND_SETTINGS_KEY, JSON.stringify(next)); } catch {}
    return { soundSettings: next };
  }),
  globeAutoRotate: getSavedGlobeAutoRotate(),
  setGlobeAutoRotate: (enabled) => {
    try { localStorage.setItem(GLOBE_AUTO_ROTATE_KEY, enabled ? '1' : '0'); } catch {}
    set({ globeAutoRotate: enabled });
  },
  settingsOpen: false,
  setSettingsOpen: (open) => set({ settingsOpen: open }),
  speedMultiplier: 1,
  setSpeed: (speed) => set({ speedMultiplier: speed }),
  sidebarExpanded: typeof window !== 'undefined' ? window.innerWidth >= 768 : true,
  toggleSidebar: () => set(s => ({ sidebarExpanded: !s.sidebarExpanded })),

  isWarping: false,
  fastForwardTarget: null,
  setIsWarping: (w) => set({ isWarping: w }),
  setFastForwardTarget: (t) => set({ fastForwardTarget: t }),

  centroidTrail: [],
  setCentroidTrail: (trail) => set({ centroidTrail: trail }),

  wsStatus: 'connecting',
  wsLastMessageAt: null,
  wsCloseInfo: null,
  wsReconnectCount: 0,
  setWsStatus: (s) => set({ wsStatus: s }),
  setWsLastMessageAt: (t) => set({ wsLastMessageAt: t }),
  setWsCloseInfo: (info) => set({ wsCloseInfo: info }),
  incrementWsReconnectCount: () => set(s => ({ wsReconnectCount: s.wsReconnectCount + 1 })),

  milestones: [],
  addMilestone: (m) => set(s => ({ milestones: [m, ...s.milestones].slice(0, 50) })),
  clearMilestones: () => set({ milestones: [] }),

  runtimeMetrics: null,
  setRuntimeMetrics: (m) => set({ runtimeMetrics: m }),

  updatePercent: null,
  updateReady: null,
  updateInstall: null,
  setUpdatePercent: (p) => set({ updatePercent: p }),
  setUpdateReady: (info) => set({ updateReady: info }),
  setUpdateInstall: (install) => set({ updateInstall: install }),
}));
