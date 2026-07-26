import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { useEffect, useState } from 'react';
import axios from 'axios';
import { useSimStore } from './store/simStore';
import UpdateBanner from './components/layout/UpdateBanner';
import LoginPage from './pages/LoginPage';
import DashboardPage from './pages/DashboardPage';
import SimulationPage from './pages/SimulationPage';
import AdminPage from './pages/AdminPage';
import WatchPage from './pages/WatchPage';
import AriaButton from './components/layout/AriaButton';
import SettingsOverlay from './components/layout/SettingsOverlay';
import { useGlobalAudioEffects } from './hooks/useGlobalAudioEffects';
import { authUrl } from './utils/cloud';
import { text, type LangCode } from './utils/i18n';
import { isTauriDesktop, checkForDesktopUpdate } from './utils/desktopUpdate';

const AUTH_USER_KEY = 'anatolia_auth_user';
const AUTH_TOKEN_KEY = 'anatolia_auth_token';

function readStoredAuth() {
  const sources: Storage[] = [];
  try { sources.push(localStorage); } catch {}
  try { sources.push(sessionStorage); } catch {}

  for (const storage of sources) {
    try {
      const token = storage.getItem(AUTH_TOKEN_KEY);
      const rawUser = storage.getItem(AUTH_USER_KEY);
      if (!token || !rawUser) continue;
      const user = JSON.parse(rawUser);
      if (user && typeof user === 'object') {
        return { user, token };
      }
    } catch {}
  }
  return null;
}

function PrivateRoute({ children }: { children: React.ReactNode }) {
  const { user } = useSimStore();
  return user ? <>{children}</> : <Navigate to="/login" replace />;
}

function AdminRoute({ children }: { children: React.ReactNode }) {
  const { user } = useSimStore();
  if (!user) return <Navigate to="/login" replace />;
  if (user.role !== 'admin') return <Navigate to="/" replace />;
  return <>{children}</>;
}

export default function App() {
  const { user, lang, setUser, setUpdatePercent, setUpdateReady, setUpdateInstall, settingsOpen, setSettingsOpen } = useSimStore();
  const [serverDown, setServerDown] = useState(false);
  useGlobalAudioEffects();

  useEffect(() => {
    let cancelled = false;
    let unlisten: null | (() => void) = null;

    if (!isTauriDesktop()) return;

    (async () => {
      try {
        const [{ installUpdate, onUpdaterEvent }, { relaunch }] = await Promise.all([
          import('@tauri-apps/api/updater'),
          import('@tauri-apps/api/process'),
        ]);

        if (cancelled) return;

        setUpdateInstall(async () => {
          setUpdatePercent(0);
          try {
            await installUpdate();
            await relaunch();
          } finally {
            setUpdatePercent(null);
            setUpdateReady(null);
          }
        });

        unlisten = await onUpdaterEvent(({ error, status }) => {
          if (error) {
            console.error('[updater]', error);
            return;
          }
          console.debug('[updater]', status);
        });

        const info = await checkForDesktopUpdate();
        if (cancelled) return;
        if (info) {
          setUpdateReady(info);
        }
      } catch (err) {
        console.warn('[updater] unavailable:', err);
      }
    })();

    return () => {
      cancelled = true;
      try { unlisten?.(); } catch {}
      setUpdateInstall(null);
    };
  }, [setUpdateInstall, setUpdatePercent, setUpdateReady]);
  const [authChecked, setAuthChecked] = useState(false);

  useEffect(() => {
    // Cross-origin handoff from desktop's local (Yerel) mode: "Devam Et" on
    // a cloud simulation there does a full navigation here with the user's
    // cloud access token as a query param, since localStorage from the
    // local sidecar's origin (127.0.0.1) isn't visible on this origin. Runs
    // before the normal stored-session check below so it wins when present.
    const bridgeToken = new URLSearchParams(window.location.search).get('token');
    if (bridgeToken) {
      axios.get('/api/auth/me', { headers: { Authorization: `Bearer ${bridgeToken}` } })
        .then(({ data }) => {
          // Set before setUser (see LoginPage.tsx's own comment on this
          // ordering) -- this bridge handoff always uses sessionStorage, so
          // persistAuth's localStorage check stays false and the token
          // doesn't linger past this tab's lifetime.
          try { sessionStorage.setItem('anatolia_session_active', '1'); } catch {}
          setUser(data, bridgeToken);
          try {
            sessionStorage.setItem(AUTH_TOKEN_KEY, bridgeToken);
            sessionStorage.setItem(AUTH_USER_KEY, JSON.stringify(data));
          } catch {}
        })
        .finally(() => {
          const url = new URL(window.location.href);
          url.searchParams.delete('token');
          window.history.replaceState({}, '', url.pathname + url.search + url.hash);
          setAuthChecked(true);
        });
      return;
    }

    const storedAuth = readStoredAuth();
    if (storedAuth) {
      setUser(storedAuth.user, storedAuth.token);
    }

    const sessionActive =
      localStorage.getItem('anatolia_session_active') === '1' ||
      sessionStorage.getItem('anatolia_session_active') === '1';
    if (!sessionActive && !storedAuth) {
      setAuthChecked(true);
      return;
    }
    axios.post(authUrl('/api/auth/refresh'), undefined, { withCredentials: true })
      .then(({ data }) => {
        setUser(data.user, data.access_token);
        // Hangisi aktifse onu yenile
        if (localStorage.getItem('anatolia_session_active') === '1') {
          localStorage.setItem('anatolia_session_active', '1');
        } else {
          sessionStorage.setItem('anatolia_session_active', '1');
        }
      })
      .catch(() => {
        // If refresh cookies are unavailable but we have a stored session
        // snapshot, keep the user signed in and let the access token be
        // refreshed later if possible.
        if (!storedAuth) {
          try { sessionStorage.removeItem('anatolia_session_active'); } catch {}
          try { localStorage.removeItem('anatolia_session_active'); } catch {}
        }
      })
      .finally(() => setAuthChecked(true));
  }, [setUser]);

  // Proactive token refresh every 14 minutes (access token expires in 15 min)
  useEffect(() => {
    if (!user) return;
    const interval = setInterval(() => {
      axios.post(authUrl('/api/auth/refresh'), undefined, { withCredentials: true })
        .then(({ data }) => setUser(data.user, data.access_token))
        .catch(() => {});
    }, 14 * 60 * 1000);
    return () => clearInterval(interval);
  }, [user, setUser]);

  // Axios interceptor: retry on network errors (server down) with exponential backoff
  useEffect(() => {
    const retryInterceptor = axios.interceptors.response.use(
      res => { setServerDown(false); return res; },
      async err => {
        const cfg = err.config;
        if (!cfg || cfg._retryCount >= 3) { if (!err.response) setServerDown(true); return Promise.reject(err); }
        if (err.response) return Promise.reject(err); // HTTP error — don't retry
        cfg._retryCount = (cfg._retryCount ?? 0) + 1;
        await new Promise(r => setTimeout(r, cfg._retryCount * 1000));
        return axios(cfg);
      }
    );
    return () => axios.interceptors.response.eject(retryInterceptor);
  }, []);

  // Axios interceptor: on 401, try refresh once then retry original request
  useEffect(() => {
    let isRefreshing = false;
    const queue: Array<(token: string) => void> = [];

    const interceptor = axios.interceptors.response.use(
      res => res,
      async (err) => {
        const original = err.config;
        if (err.response?.status !== 401 || original._retry) return Promise.reject(err);
        if ((original.url as string)?.includes('/api/auth/')) return Promise.reject(err);
        if (isRefreshing) {
          return new Promise((resolve) => {
            queue.push((token) => {
              original.headers['Authorization'] = `Bearer ${token}`;
              resolve(axios(original));
            });
          });
        }
        original._retry = true;
        isRefreshing = true;
        try {
          const { data } = await axios.post(authUrl('/api/auth/refresh'), undefined, { withCredentials: true });
          setUser(data.user, data.access_token);
          queue.forEach(cb => cb(data.access_token));
          original.headers['Authorization'] = `Bearer ${data.access_token}`;
          return axios(original);
        } catch {
          return Promise.reject(err);
        } finally {
          isRefreshing = false;
          queue.length = 0;
        }
      }
    );
    return () => axios.interceptors.response.eject(interceptor);
  }, [setUser]);

  useEffect(() => {
    let lastTap = 0;
    function onTouchEnd(e: TouchEvent) {
      const now = Date.now();
      if (now - lastTap < 300 && e.touches.length === 0) {
        const el = document.documentElement as any;
        if (!document.fullscreenElement && !(document as any).webkitFullscreenElement) {
          (el.requestFullscreen?.() ?? el.webkitRequestFullscreen?.())?.catch?.(() => {});
        } else {
          (document.exitFullscreen?.() ?? (document as any).webkitExitFullscreen?.())?.catch?.(() => {});
        }
      }
      lastTap = now;
    }
    document.addEventListener('touchend', onTouchEnd, { passive: true });
    return () => document.removeEventListener('touchend', onTouchEnd);
  }, []);

  if (!authChecked) return <div className="w-screen h-screen bg-sim-bg" />;

  return (
    <BrowserRouter>
      {serverDown && (
        <div className="fixed top-0 left-0 right-0 z-[9999] flex items-center justify-center gap-2 py-2"
          style={{ background: 'rgba(200,34,34,0.92)', backdropFilter: 'blur(8px)' }}>
          <span className="inline-block w-2 h-2 rounded-full bg-white" style={{ animation: 'pulse 1s infinite' }} />
          <span className="font-share-tech tracking-widest text-white" style={{ fontSize: 13 }}>
            {text(lang as LangCode, { tr: 'SUNUCU BAĞLANTISI KESİLDİ — YENİDEN DENENİYOR...', en: 'SERVER CONNECTION LOST — RETRYING...', de: 'SERVERVERBINDUNG UNTERBROCHEN — WIRD WIEDERHOLT...', fr: 'CONNEXION AU SERVEUR PERDUE — NOUVELLE TENTATIVE...', ar: 'انقطع الاتصال بالخادم — جارٍ إعادة المحاولة...' })}
          </span>
        </div>
      )}
      {user && <AriaButton />}
      <SettingsOverlay isOpen={settingsOpen} onClose={() => setSettingsOpen(false)} />
      <UpdateBanner />
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route path="/admin" element={<AdminRoute><AdminPage /></AdminRoute>} />
        <Route path="/" element={<PrivateRoute><DashboardPage /></PrivateRoute>} />
        <Route path="/simulation/:simId" element={<PrivateRoute><SimulationPage /></PrivateRoute>} />
        <Route path="/watch/:simId" element={<PrivateRoute><WatchPage /></PrivateRoute>} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </BrowserRouter>
  );
}
