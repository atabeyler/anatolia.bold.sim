import { describe, it, expect, beforeEach } from 'vitest';
import { useSimStore } from './simStore';

const baseEvent = (overrides: Record<string, any> = {}) => ({
  sim_day: 1,
  sim_year: 1,
  event_type: 'birth',
  description: 'test event',
  importance: 1,
  ...overrides,
});

beforeEach(() => {
  useSimStore.setState({
    events: [],
    moments: [],
    milestones: [],
    activePanel: null,
    lang: 'en',
  });
});

// ── addEvent() dedup ─────────────────────────────────────────────────────────

describe('addEvent()', () => {
  it('yeni bir olayı listenin başına ekler', () => {
    useSimStore.getState().addEvent(baseEvent({ id: 'e1' }));
    expect(useSimStore.getState().events).toHaveLength(1);
    expect(useSimStore.getState().events[0].id).toBe('e1');
  });

  it('aynı id ile eklenen olayı yinelemez', () => {
    useSimStore.getState().addEvent(baseEvent({ id: 'e1' }));
    useSimStore.getState().addEvent(baseEvent({ id: 'e1' }));
    expect(useSimStore.getState().events).toHaveLength(1);
  });

  it('id yokken gün+yıl+tip+açıklama aynıysa yine de yinelemez', () => {
    useSimStore.getState().addEvent(baseEvent());
    useSimStore.getState().addEvent(baseEvent());
    expect(useSimStore.getState().events).toHaveLength(1);
  });

  it('olay listesini en fazla 200 kayıtla sınırlar', () => {
    for (let i = 0; i < 250; i++) {
      useSimStore.getState().addEvent(baseEvent({ id: `e${i}` }));
    }
    expect(useSimStore.getState().events).toHaveLength(200);
    // en son eklenen olay listenin başında olmalı
    expect(useSimStore.getState().events[0].id).toBe('e249');
  });

  // Regression: a large tick batch at high sim speed can push more than 200
  // unrelated events (births, thoughts, discoveries, ...) alongside a few
  // death events in the same WS delivery -- a flat 200-cap shared across all
  // event types used to silently evict an earlier death from that very
  // batch before the Events panel's "Ölüm"/Death filter ever saw it.
  it('yoğun olay akışında ölüm olaylarını atmadan korur', () => {
    useSimStore.getState().addEvent(baseEvent({ id: 'death-1', event_type: 'death' }));
    for (let i = 0; i < 250; i++) {
      useSimStore.getState().addEvent(baseEvent({ id: `birth-${i}`, event_type: 'birth' }));
    }
    const events = useSimStore.getState().events;
    expect(events.some(e => e.id === 'death-1')).toBe(true);
    expect(events.filter(e => e.event_type === 'birth')).toHaveLength(200);
  });

  it('ölüm olayları kendi ayrı, daha büyük bir üst sınıra sahiptir', () => {
    for (let i = 0; i < 350; i++) {
      useSimStore.getState().addEvent(baseEvent({ id: `death-${i}`, event_type: 'death' }));
    }
    const deaths = useSimStore.getState().events.filter(e => e.event_type === 'death');
    expect(deaths).toHaveLength(300);
    expect(deaths[0].id).toBe('death-349');
  });
});

// ── setEvents() dedup ────────────────────────────────────────────────────────

describe('setEvents()', () => {
  it('gelen listedeki yinelenen id\'leri eler', () => {
    useSimStore.getState().setEvents([
      baseEvent({ id: 'a' }),
      baseEvent({ id: 'a' }),
      baseEvent({ id: 'b' }),
    ]);
    expect(useSimStore.getState().events.map(e => e.id)).toEqual(['a', 'b']);
  });
});

// ── activePanel toggle ───────────────────────────────────────────────────────

describe('setActivePanel()', () => {
  it('kapalıyken bir paneli açar', () => {
    useSimStore.getState().setActivePanel('biology');
    expect(useSimStore.getState().activePanel).toBe('biology');
  });

  it('aynı panel tekrar seçilirse kapatır (toggle)', () => {
    useSimStore.getState().setActivePanel('biology');
    useSimStore.getState().setActivePanel('biology');
    expect(useSimStore.getState().activePanel).toBeNull();
  });

  it('farklı bir panel seçilirse öncekinin yerine geçer', () => {
    useSimStore.getState().setActivePanel('biology');
    useSimStore.getState().setActivePanel('language');
    expect(useSimStore.getState().activePanel).toBe('language');
  });
});

// ── moments / milestones caps ────────────────────────────────────────────────

describe('addMoment()', () => {
  it('moments listesini en fazla 100 kayıtla sınırlar', () => {
    for (let i = 0; i < 120; i++) {
      useSimStore.getState().addMoment({ day: i, year: 1, icon: '✨', title: `m${i}`, color: '#fff' });
    }
    expect(useSimStore.getState().moments).toHaveLength(100);
    expect(useSimStore.getState().moments[0].title).toBe('m119');
  });
});

describe('addMilestone()', () => {
  it('milestones listesini en fazla 50 kayıtla sınırlar', () => {
    for (let i = 0; i < 60; i++) {
      useSimStore.getState().addMilestone({ key: `k${i}`, description: 'd', icon: '🏆', day: i });
    }
    expect(useSimStore.getState().milestones).toHaveLength(50);
    expect(useSimStore.getState().milestones[0].key).toBe('k59');
  });
});

// ── toggleLang() ─────────────────────────────────────────────────────────────

describe('toggleLang()', () => {
  it('dil listesinde bir sonraki dile geçer', () => {
    useSimStore.setState({ lang: 'en' });
    useSimStore.getState().toggleLang();
    expect(useSimStore.getState().lang).not.toBe('en');
  });

  it('son dilden sonra listenin başına döner', () => {
    useSimStore.setState({ lang: 'ar' }); // LANG_CODES son eleman
    useSimStore.getState().toggleLang();
    expect(useSimStore.getState().lang).toBe('tr');
  });
});

// ── setUser() / persistAuth (H-14 regression) ───────────────────────────────
// "Remember me" previously controlled nothing about where the actual access
// token landed -- persistAuth wrote it to localStorage unconditionally, so a
// user who explicitly did not check the box still got logged back in on a
// fresh browser launch. persistAuth (called from setUser) now checks
// 'anatolia_session_active' -- localStorage means remembered, sessionStorage
// means this-tab-only -- which LoginPage.tsx sets *before* calling setUser.

describe('setUser() / persistAuth', () => {
  const AUTH_TOKEN_KEY = 'anatolia_auth_token';
  const AUTH_USER_KEY = 'anatolia_auth_user';
  const testUser = { id: 'u1', role: 'user' } as any;

  beforeEach(() => {
    localStorage.clear();
    sessionStorage.clear();
  });

  it('persists the token to localStorage when the session was marked remembered', () => {
    localStorage.setItem('anatolia_session_active', '1');
    useSimStore.getState().setUser(testUser, 'token-123');
    expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBe('token-123');
    expect(JSON.parse(localStorage.getItem(AUTH_USER_KEY)!)).toEqual(testUser);
  });

  it('does not persist the token to localStorage when the session was not remembered', () => {
    sessionStorage.setItem('anatolia_session_active', '1'); // not localStorage
    useSimStore.getState().setUser(testUser, 'token-456');
    expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(AUTH_USER_KEY)).toBeNull();
  });

  it('always persists to sessionStorage regardless of remember choice, so the current tab stays logged in', () => {
    useSimStore.getState().setUser(testUser, 'token-789');
    expect(sessionStorage.getItem(AUTH_TOKEN_KEY)).toBe('token-789');
    expect(JSON.parse(sessionStorage.getItem(AUTH_USER_KEY)!)).toEqual(testUser);
  });

  it('clears a stale remembered localStorage token once logging in again without remember checked', () => {
    // Simulates: user previously logged in with "remember me", then logs in
    // again on the same device without checking it.
    localStorage.setItem(AUTH_TOKEN_KEY, 'stale-remembered-token');
    localStorage.setItem(AUTH_USER_KEY, JSON.stringify(testUser));
    sessionStorage.setItem('anatolia_session_active', '1'); // this login: not remembered
    useSimStore.getState().setUser(testUser, 'fresh-token');
    expect(localStorage.getItem(AUTH_TOKEN_KEY)).toBeNull();
    expect(localStorage.getItem(AUTH_USER_KEY)).toBeNull();
  });
});

// ── resetLiveState() ─────────────────────────────────────────────────────────

describe('resetLiveState()', () => {
  it('canlı simülasyon durumunu temizler ama auth/lang gibi kalıcı alanlara dokunmaz', () => {
    useSimStore.setState({
      stats: { population: 5 } as any,
      events: [baseEvent({ id: 'e1' })],
      isWarping: true,
      lang: 'de',
    });
    useSimStore.getState().resetLiveState();
    const state = useSimStore.getState();
    expect(state.stats).toBeNull();
    expect(state.events).toEqual([]);
    expect(state.isWarping).toBe(false);
    expect(state.lang).toBe('de');
  });
});
