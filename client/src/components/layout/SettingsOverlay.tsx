import { useEffect, useState } from 'react';
import axios from 'axios';
import { X } from 'lucide-react';
import { useSimStore } from '../../store/simStore';
import { text, type LangCode } from '../../utils/i18n';
import { LANGUAGES } from '../../utils/menuI18n';
import { isMusicPlaying } from '../../utils/audioEngine';
import { isTauriDesktop, checkForDesktopUpdate } from '../../utils/desktopUpdate';
import { isNativeAndroidApp } from '../../utils/nativeMode';
import { checkForAndroidUpdateDetailed, installAndroidUpdate, type AndroidUpdateInfo } from '../../utils/androidUpdate';
import { authUrl } from '../../utils/cloud';

type Tab = 'profile' | 'language' | 'sound' | 'display' | 'about';

interface Props {
  isOpen: boolean;
  onClose: () => void;
}

function ToggleRow({ label, checked, onChange, rtl }: { label: string; checked: boolean; onChange: (v: boolean) => void; rtl: boolean }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 10, flexDirection: rtl ? 'row-reverse' : 'row' }}>
      <span style={{ fontSize: 14, color: '#a0c8b0', letterSpacing: '0.04em' }}>{label}</span>
      <button
        onClick={() => onChange(!checked)}
        aria-pressed={checked}
        style={{
          width: 38, height: 20, borderRadius: 10, flexShrink: 0, position: 'relative',
          background: checked ? 'rgba(0,232,135,0.35)' : 'rgba(160,200,176,0.15)',
          border: `1px solid ${checked ? '#00e887' : 'rgba(160,200,176,0.35)'}`,
          cursor: 'pointer', padding: 0,
        }}>
        <span style={{
          position: 'absolute', top: 1, left: checked ? 19 : 1, width: 16, height: 16, borderRadius: '50%',
          background: checked ? '#00e887' : '#6a9a78', transition: 'left 0.15s',
        }} />
      </button>
    </div>
  );
}

function SliderRow({ label, value, disabled, onChange }: { label: string; value: number; disabled?: boolean; onChange: (v: number) => void }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4, opacity: disabled ? 0.4 : 1 }}>
      <span style={{ fontSize: 14, color: '#a0c8b0', letterSpacing: '0.04em' }}>{label}</span>
      <input
        type="range" min={0} max={1} step={0.05} value={value} disabled={disabled}
        onChange={e => onChange(Number(e.target.value))}
        style={{ width: '100%', accentColor: '#00e887' }}
      />
    </div>
  );
}

type CheckState = 'idle' | 'checking' | 'up-to-date' | 'found' | 'error';

export default function SettingsOverlay({ isOpen, onClose }: Props) {
  const { lang, setLang, soundSettings, setSoundSettings, globeAutoRotate, setGlobeAutoRotate, updateReady, setUpdateReady, user, accessToken, setUser } = useSimStore();
  const activeLang = lang as LangCode;
  const rtl = activeLang === 'ar';
  const [tab, setTab] = useState<Tab>(user ? 'profile' : 'language');

  // Profile ("Hesap Bilgilerim") -- self-service edit of the logged-in
  // user's own account fields, writing to the same `users` row the admin
  // panel reads, so there's no separate sync step: an admin's next
  // `/api/admin/users` load already reflects whatever was changed here.
  const [pFirstName, setPFirstName] = useState('');
  const [pLastName, setPLastName] = useState('');
  const [pTcNo, setPTcNo] = useState('');
  const [pUserCode, setPUserCode] = useState('');
  const [pNickname, setPNickname] = useState('');
  const [pEmail, setPEmail] = useState('');
  const [pPassword, setPPassword] = useState('');
  const [pError, setPError] = useState<string | null>(null);
  const [pSuccess, setPSuccess] = useState(false);
  const [pSaving, setPSaving] = useState(false);

  useEffect(() => {
    if (!user) return;
    setPFirstName(user.first_name ?? '');
    setPLastName(user.last_name ?? '');
    setPTcNo(user.tc_no ?? '');
    setPUserCode(user.username ?? '');
    setPNickname(user.nickname ?? '');
    setPEmail(user.email?.endsWith('@no-email.internal') ? '' : (user.email ?? ''));
    setPPassword('');
    setPError(null);
    setPSuccess(false);
  }, [user, isOpen]);

  async function saveProfile() {
    setPError(null);
    setPSuccess(false);
    setPSaving(true);
    try {
      const { data } = await axios.put(authUrl('/api/auth/me'), {
        user_code: pUserCode,
        first_name: pFirstName,
        last_name: pLastName,
        tc_no: pTcNo,
        username: pNickname.trim() || null,
        email: pEmail.trim() || null,
        password: pPassword.trim() || null,
      }, { headers: { Authorization: `Bearer ${accessToken}` } });
      setUser(data.user, accessToken!);
      setPPassword('');
      setPSuccess(true);
    } catch (err: any) {
      setPError(err?.response?.data?.error ?? text(activeLang, { tr: 'Güncellenemedi.', en: 'Failed to update.', de: 'Aktualisierung fehlgeschlagen.', fr: 'Échec de la mise à jour.', ar: 'فشل التحديث.' }));
    } finally {
      setPSaving(false);
    }
  }

  const [checkState, setCheckState] = useState<CheckState>('idle');
  const [androidUpdate, setAndroidUpdate] = useState<AndroidUpdateInfo | null>(null);
  const [checkError, setCheckError] = useState('');
  const [installState, setInstallState] = useState<'idle' | 'downloading' | 'permission-required' | 'error'>('idle');
  const [downloadPercent, setDownloadPercent] = useState(0);
  const platform = isNativeAndroidApp() ? 'android' : isTauriDesktop() ? 'desktop' : 'web';

  const handleInstall = async () => {
    if (!androidUpdate) return;
    setInstallState('downloading');
    setDownloadPercent(0);
    const result = await installAndroidUpdate(androidUpdate, setDownloadPercent);
    if (result === 'ok') {
      setInstallState('idle');
    } else {
      setInstallState(result);
    }
  };

  const runUpdateCheck = async () => {
    setCheckState('checking');
    if (platform === 'desktop') {
      const info = await checkForDesktopUpdate();
      if (info) {
        setUpdateReady(info);
        setCheckState('found');
      } else {
        setCheckState('up-to-date');
      }
    } else if (platform === 'android') {
      const result = await checkForAndroidUpdateDetailed();
      if (result.status === 'found') {
        setAndroidUpdate(result.info);
        setCheckState('found');
      } else if (result.status === 'up-to-date') {
        setCheckState('up-to-date');
      } else {
        setCheckError(result.reason);
        setCheckState('error');
      }
    } else {
      setCheckState('up-to-date');
    }
  };

  // Browsers block audio autoplay until the user interacts with the page, so
  // "enabled" in settings and "actually playing" (isMusicPlaying) can
  // genuinely disagree -- surface that instead of silently doing nothing.
  const [musicActuallyPlaying, setMusicActuallyPlaying] = useState(false);
  useEffect(() => {
    if (!isOpen || tab !== 'sound') return;
    setMusicActuallyPlaying(isMusicPlaying());
    const id = setInterval(() => setMusicActuallyPlaying(isMusicPlaying()), 500);
    return () => clearInterval(id);
  }, [isOpen, tab]);

  if (!isOpen) return null;

  const TABS: Array<{ id: Tab; label: string }> = [
    ...(user ? [{ id: 'profile' as const, label: text(activeLang, { tr: 'Hesabım', en: 'Account', de: 'Konto', fr: 'Compte', ar: 'حسابي' }) }] : []),
    { id: 'language', label: text(activeLang, { tr: 'Dil', en: 'Language', de: 'Sprache', fr: 'Langue', ar: 'اللغة' }) },
    { id: 'sound', label: text(activeLang, { tr: 'Ses', en: 'Sound', de: 'Ton', fr: 'Son', ar: 'الصوت' }) },
    { id: 'display', label: text(activeLang, { tr: 'Görünüm', en: 'Display', de: 'Anzeige', fr: 'Affichage', ar: 'العرض' }) },
    { id: 'about', label: text(activeLang, { tr: 'Hakkında', en: 'About', de: 'Info', fr: 'À propos', ar: 'حول' }) },
  ];

  const inputStyle: React.CSSProperties = {
    width: '100%', padding: '8px 10px', fontSize: 14, fontFamily: 'Share Tech Mono, monospace',
    background: 'rgba(0,20,10,0.5)', border: '1px solid rgba(160,200,176,0.25)', color: '#e0f0e6',
    outline: 'none',
  };

  return (
    <div
      style={{ position: 'fixed', inset: 0, zIndex: 201, background: 'rgba(0,0,0,0.85)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
      onClick={onClose}>
      <div
        dir={rtl ? 'rtl' : 'ltr'}
        style={{ background: 'rgba(0,4,2,0.98)', border: '1px solid #cc2222', width: 'clamp(300px, 90vw, 480px)', fontFamily: 'Share Tech Mono, monospace', boxShadow: '0 8px 40px rgba(0,0,0,0.8)', overflow: 'hidden' }}
        onClick={e => e.stopPropagation()}>

        {/* Header */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '8px 14px', borderBottom: '1px solid #cc2222', background: 'rgba(0,20,10,0.9)' }}>
          <div style={{ width: 3, height: 14, background: '#00e887', boxShadow: '0 0 6px #00e887', flexShrink: 0 }} />
          <span style={{ fontSize: 14, color: '#00e887', letterSpacing: '0.2em', flex: 1 }}>
            {text(activeLang, { tr: 'AYARLAR', en: 'SETTINGS', de: 'EINSTELLUNGEN', fr: 'PARAMÈTRES', ar: 'الإعدادات' })}
          </span>
          <button onClick={onClose} style={{ background: 'transparent', border: 'none', color: '#6a9a78', cursor: 'pointer', display: 'flex', alignItems: 'center' }}>
            <X size={12} />
          </button>
        </div>

        {/* Tabs */}
        <div style={{ display: 'flex', borderBottom: '1px solid #0a1a10' }}>
          {TABS.map(t => (
            <button key={t.id} onClick={() => setTab(t.id)}
              style={{
                flex: 1, padding: '8px 10px',
                background: tab === t.id ? 'rgba(0,232,135,0.08)' : 'transparent',
                border: 'none', borderBottom: tab === t.id ? '2px solid #00e887' : '2px solid transparent',
                color: tab === t.id ? '#00e887' : '#6a9a78',
                fontSize: 14, letterSpacing: '0.08em', cursor: 'pointer', fontFamily: 'Share Tech Mono, monospace',
              }}>
              {t.label}
            </button>
          ))}
        </div>

        {tab === 'profile' && user && (
          <div style={{ padding: '14px', display: 'flex', flexDirection: 'column', gap: 10, maxHeight: 420, overflowY: 'auto' }}>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
              <input style={inputStyle} placeholder={text(activeLang, { tr: 'Ad', en: 'First name', de: 'Vorname', fr: 'Prénom', ar: 'الاسم' })}
                value={pFirstName} onChange={e => setPFirstName(e.target.value)} />
              <input style={inputStyle} placeholder={text(activeLang, { tr: 'Soyad', en: 'Last name', de: 'Nachname', fr: 'Nom', ar: 'اللقب' })}
                value={pLastName} onChange={e => setPLastName(e.target.value)} />
            </div>
            <input style={inputStyle} placeholder={text(activeLang, { tr: 'TC Kimlik No (11 hane)', en: 'National ID (11 digits)', de: 'Ausweisnummer (11 Ziffern)', fr: "N° d'identité (11 chiffres)", ar: 'رقم الهوية (11 رقمًا)' })}
              value={pTcNo} maxLength={11} onChange={e => setPTcNo(e.target.value.replace(/\D/g, ''))} />
            <input style={inputStyle} placeholder={text(activeLang, { tr: 'Kullanıcı kodu', en: 'User code', de: 'Benutzercode', fr: 'Code utilisateur', ar: 'رمز المستخدم' })}
              value={pUserCode} onChange={e => setPUserCode(e.target.value)} />
            <input style={inputStyle} placeholder={text(activeLang, { tr: 'Rumuz (opsiyonel)', en: 'Nickname (optional)', de: 'Spitzname (optional)', fr: 'Pseudo (facultatif)', ar: 'الاسم المستعار (اختياري)' })}
              value={pNickname} onChange={e => setPNickname(e.target.value)} />
            <input style={inputStyle} placeholder={text(activeLang, { tr: 'E-posta', en: 'Email', de: 'E-Mail', fr: 'E-mail', ar: 'البريد الإلكتروني' })}
              value={pEmail} onChange={e => setPEmail(e.target.value)} />
            <input type="password" style={inputStyle} placeholder={text(activeLang, { tr: 'Yeni şifre (opsiyonel, boş bırakılırsa değişmez)', en: 'New password (optional, leave blank to keep current)', de: 'Neues Passwort (optional)', fr: 'Nouveau mot de passe (facultatif)', ar: 'كلمة مرور جديدة (اختياري)' })}
              value={pPassword} onChange={e => setPPassword(e.target.value)} />
            {pError && <span style={{ fontSize: 12, color: '#e05a5a' }}>{pError}</span>}
            {pSuccess && !pError && (
              <span style={{ fontSize: 12, color: '#00e887' }}>
                {text(activeLang, { tr: '✓ Bilgileriniz güncellendi.', en: '✓ Your details were updated.', de: '✓ Ihre Angaben wurden aktualisiert.', fr: '✓ Vos informations ont été mises à jour.', ar: '✓ تم تحديث بياناتك.' })}
              </span>
            )}
            <button
              onClick={saveProfile}
              disabled={pSaving || !pUserCode.trim() || !pFirstName.trim() || !pLastName.trim() || pTcNo.length !== 11}
              style={{
                padding: '8px 14px', fontSize: 14, alignSelf: 'flex-start',
                border: '1px solid rgba(0,232,135,0.5)', color: '#00e887',
                background: 'rgba(0,232,135,0.08)', fontFamily: 'Share Tech Mono, monospace',
                cursor: pSaving ? 'default' : 'pointer',
                opacity: pSaving || !pUserCode.trim() || !pFirstName.trim() || !pLastName.trim() || pTcNo.length !== 11 ? 0.5 : 1,
                letterSpacing: '0.06em',
              }}>
              {pSaving
                ? text(activeLang, { tr: 'KAYDEDİLİYOR…', en: 'SAVING…', de: 'WIRD GESPEICHERT…', fr: 'ENREGISTREMENT…', ar: 'جارٍ الحفظ…' })
                : text(activeLang, { tr: 'ONAYLA', en: 'CONFIRM', de: 'BESTÄTIGEN', fr: 'CONFIRMER', ar: 'تأكيد' })}
            </button>
          </div>
        )}

        {tab === 'language' && (
          <div style={{ padding: '6px 0' }}>
            {LANGUAGES.map(l => (
              <button key={l.code}
                onClick={() => setLang(l.code)}
                style={{
                  display: 'flex', alignItems: 'center', gap: 8,
                  width: '100%', padding: '10px 14px',
                  background: activeLang === l.code ? 'rgba(0,232,135,0.08)' : 'transparent',
                  border: 'none', borderBottom: '1px solid #0a1a10',
                  color: activeLang === l.code ? '#00e887' : '#a0c8b0',
                  fontSize: 14, textAlign: rtl ? 'right' : 'left', cursor: 'pointer',
                  letterSpacing: '0.08em', fontFamily: 'Share Tech Mono, monospace',
                }}>
                <span style={{ flex: 1 }}>› {l.label}</span>
                {activeLang === l.code && <span style={{ fontSize: 14, color: '#00e887' }}>✓</span>}
              </button>
            ))}
          </div>
        )}

        {tab === 'sound' && (
          <div style={{ padding: '14px', display: 'flex', flexDirection: 'column', gap: 14 }}>
            <ToggleRow rtl={rtl}
              label={text(activeLang, { tr: 'Arka Plan Müziği', en: 'Background Music', de: 'Hintergrundmusik', fr: 'Musique de fond', ar: 'موسيقى الخلفية' })}
              checked={soundSettings.musicEnabled}
              onChange={v => setSoundSettings({ musicEnabled: v })}
            />
            {soundSettings.musicEnabled && !musicActuallyPlaying && (
              <div style={{ fontSize: 12, color: '#c8a840', letterSpacing: '0.02em' }}>
                {text(activeLang, {
                  tr: 'Tarayıcı otomatik oynatmayı engelliyor olabilir — sayfaya bir kez dokunun.',
                  en: "Your browser may be blocking autoplay — tap anywhere on the page once.",
                  de: 'Ihr Browser blockiert möglicherweise die automatische Wiedergabe — tippen Sie einmal auf die Seite.',
                  fr: "Votre navigateur bloque peut-être la lecture automatique — touchez la page une fois.",
                  ar: 'قد يمنع متصفحك التشغيل التلقائي — انقر في أي مكان بالصفحة مرة واحدة.',
                })}
              </div>
            )}
            <SliderRow
              label={text(activeLang, { tr: 'Müzik Seviyesi', en: 'Music Volume', de: 'Musiklautstärke', fr: 'Volume de la musique', ar: 'مستوى الموسيقى' })}
              value={soundSettings.musicVolume}
              disabled={!soundSettings.musicEnabled}
              onChange={v => setSoundSettings({ musicVolume: v })}
            />
            <div style={{ height: 1, background: '#0a1a10' }} />
            <ToggleRow rtl={rtl}
              label={text(activeLang, { tr: 'Buton Sesi', en: 'Button Clicks', de: 'Klickgeräusche', fr: 'Sons de clic', ar: 'صوت الأزرار' })}
              checked={soundSettings.clickEnabled}
              onChange={v => setSoundSettings({ clickEnabled: v })}
            />
            <ToggleRow rtl={rtl}
              label={text(activeLang, { tr: 'Bildirim Sesi', en: 'Notification Sound', de: 'Benachrichtigungston', fr: 'Son de notification', ar: 'صوت الإشعارات' })}
              checked={soundSettings.notificationEnabled}
              onChange={v => setSoundSettings({ notificationEnabled: v })}
            />
            <ToggleRow rtl={rtl}
              label={text(activeLang, { tr: 'Tik Sesi (Gün İlerlemesi)', en: 'Tick Sound (Day Advance)', de: 'Tick-Ton (Tagesfortschritt)', fr: 'Son de tic (avancée du jour)', ar: 'صوت التكة (تقدم اليوم)' })}
              checked={soundSettings.tickEnabled}
              onChange={v => setSoundSettings({ tickEnabled: v })}
            />
            <SliderRow
              label={text(activeLang, { tr: 'Efekt Seviyesi', en: 'Effects Volume', de: 'Effektlautstärke', fr: 'Volume des effets', ar: 'مستوى المؤثرات' })}
              value={soundSettings.sfxVolume}
              disabled={!(soundSettings.clickEnabled || soundSettings.notificationEnabled || soundSettings.tickEnabled)}
              onChange={v => setSoundSettings({ sfxVolume: v })}
            />
          </div>
        )}

        {tab === 'display' && (
          <div style={{ padding: '14px', display: 'flex', flexDirection: 'column', gap: 14 }}>
            <ToggleRow rtl={rtl}
              label={text(activeLang, {
                tr: 'Dünyayı Otomatik Döndür',
                en: 'Auto-Rotate Globe',
                de: 'Globus automatisch drehen',
                fr: 'Rotation automatique du globe',
                ar: 'تدوير الكرة الأرضية تلقائيًا',
              })}
              checked={globeAutoRotate}
              onChange={setGlobeAutoRotate}
            />
            <span style={{ fontSize: 12, color: '#6a9a78', lineHeight: 1.5 }}>
              {text(activeLang, {
                tr: 'Kapatırsanız dünya kendiliğinden dönmez; bireyleri izlemek için sürükleyerek istediğiniz açıda sabit tutabilirsiniz.',
                en: 'When off, the globe stays still on its own; drag to hold it at whatever angle keeps the individuals you’re watching in view.',
                de: 'Bei Deaktivierung dreht sich der Globus nicht von selbst; ziehen Sie ihn, um ihn in dem Winkel zu halten, der die beobachteten Personen im Blick behält.',
                fr: 'Une fois désactivé, le globe reste immobile ; faites-le glisser pour le maintenir à l’angle qui garde les individus observés en vue.',
                ar: 'عند الإيقاف، تبقى الكرة الأرضية ثابتة من تلقاء نفسها؛ اسحب لتثبيتها بالزاوية التي تُبقي الأفراد الذين تراقبهم ضمن الرؤية.',
              })}
            </span>
          </div>
        )}

        {tab === 'about' && (
          <div style={{ padding: '14px', display: 'flex', flexDirection: 'column', gap: 14 }}>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              <span style={{ fontSize: 16, color: '#00e887', letterSpacing: '0.1em' }}>ANATOLIA SIM</span>
              <span style={{ fontSize: 14, color: '#a0c8b0' }}>
                {text(activeLang, { tr: 'Sürüm', en: 'Version', de: 'Version', fr: 'Version', ar: 'الإصدار' })} v{__APP_VERSION__}
                {' · '}
                {platform === 'android'
                  ? 'Android'
                  : platform === 'desktop'
                  ? text(activeLang, { tr: 'Masaüstü', en: 'Desktop', de: 'Desktop', fr: 'Bureau', ar: 'سطح المكتب' })
                  : text(activeLang, { tr: 'Web', en: 'Web', de: 'Web', fr: 'Web', ar: 'ويب' })}
              </span>
            </div>

            <div style={{ height: 1, background: '#0a1a10' }} />

            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              <span style={{ fontSize: 14, color: '#a0c8b0', letterSpacing: '0.04em' }}>
                {text(activeLang, { tr: 'Yazılım Güncelleme', en: 'Software Update', de: 'Software-Update', fr: 'Mise à jour logicielle', ar: 'تحديث البرنامج' })}
              </span>

              <button
                onClick={runUpdateCheck}
                disabled={checkState === 'checking'}
                style={{
                  alignSelf: 'flex-start',
                  padding: '6px 14px', fontSize: 14,
                  border: '1px solid rgba(0,232,135,0.5)', color: '#00e887',
                  background: 'rgba(0,232,135,0.08)', fontFamily: 'Share Tech Mono, monospace',
                  cursor: checkState === 'checking' ? 'default' : 'pointer',
                  opacity: checkState === 'checking' ? 0.6 : 1, letterSpacing: '0.06em',
                }}>
                {checkState === 'checking'
                  ? text(activeLang, { tr: 'Kontrol ediliyor…', en: 'Checking…', de: 'Prüfe…', fr: 'Vérification…', ar: 'جارٍ التحقق…' })
                  : text(activeLang, { tr: 'Güncellemeleri Kontrol Et', en: 'Check for Updates', de: 'Nach Updates suchen', fr: 'Vérifier les mises à jour', ar: 'التحقق من التحديثات' })}
              </button>

              {checkState === 'up-to-date' && (
                <span style={{ fontSize: 12, color: '#8abda0' }}>
                  {text(activeLang, { tr: '✓ En güncel sürümü kullanıyorsunuz.', en: '✓ You are on the latest version.', de: '✓ Sie verwenden die neueste Version.', fr: '✓ Vous utilisez la dernière version.', ar: '✓ أنت تستخدم أحدث إصدار.' })}
                </span>
              )}

              {checkState === 'error' && (
                <span style={{ fontSize: 12, color: '#e05a5a' }}>
                  {text(activeLang, {
                    tr: `Kontrol başarısız oldu (${checkError}). İnternet bağlantınızı kontrol edip tekrar deneyin.`,
                    en: `Check failed (${checkError}). Check your internet connection and try again.`,
                    de: `Prüfung fehlgeschlagen (${checkError}). Überprüfen Sie Ihre Internetverbindung und versuchen Sie es erneut.`,
                    fr: `Échec de la vérification (${checkError}). Vérifiez votre connexion internet et réessayez.`,
                    ar: `فشل التحقق (${checkError}). تحقق من اتصالك بالإنترنت وحاول مرة أخرى.`,
                  })}
                </span>
              )}

              {checkState === 'found' && platform === 'desktop' && updateReady && (
                <span style={{ fontSize: 12, color: '#00e887' }}>
                  {text(activeLang, {
                    tr: `v${updateReady.version ?? '?'} bulundu — ekranın altındaki bildirimden yükleyebilirsiniz.`,
                    en: `v${updateReady.version ?? '?'} found -- install it from the banner at the bottom of the screen.`,
                    de: `v${updateReady.version ?? '?'} gefunden -- installieren Sie es über die Leiste am unteren Bildschirmrand.`,
                    fr: `v${updateReady.version ?? '?'} trouvée -- installez-la depuis la bannière en bas de l'écran.`,
                    ar: `تم العثور على الإصدار v${updateReady.version ?? '?'} -- ثبّته من الشريط أسفل الشاشة.`,
                  })}
                </span>
              )}

              {checkState === 'found' && platform === 'android' && androidUpdate && (
                <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                    <span style={{ fontSize: 12, color: '#00e887' }}>
                      v{androidUpdate.version} {text(activeLang, { tr: 'mevcut', en: 'available', de: 'verfügbar', fr: 'disponible', ar: 'متاح' })}
                    </span>
                    {installState !== 'downloading' && (
                      <button
                        onClick={handleInstall}
                        style={{
                          fontSize: 14, color: '#00e887', background: 'transparent', border: 'none',
                          cursor: 'pointer', textDecoration: 'underline', fontFamily: 'Share Tech Mono, monospace',
                        }}>
                        {text(activeLang, { tr: 'Güncelle', en: 'Update', de: 'Aktualisieren', fr: 'Mettre à jour', ar: 'تحديث' })}
                      </button>
                    )}
                  </div>

                  {installState === 'downloading' && (
                    <span style={{ fontSize: 12, color: '#8abda0' }}>
                      {text(activeLang, {
                        tr: `İndiriliyor… %${downloadPercent}`,
                        en: `Downloading… ${downloadPercent}%`,
                        de: `Wird heruntergeladen… ${downloadPercent}%`,
                        fr: `Téléchargement… ${downloadPercent} %`,
                        ar: `جارٍ التنزيل… ${downloadPercent}%`,
                      })}
                    </span>
                  )}

                  {installState === 'permission-required' && (
                    <span style={{ fontSize: 12, color: '#e0b05a' }}>
                      {text(activeLang, {
                        tr: 'Kurulum için "Bilinmeyen kaynaklardan yükle" iznini açtıktan sonra tekrar deneyin.',
                        en: 'Enable "Install unknown apps" for this app, then try again.',
                        de: 'Aktivieren Sie „Unbekannte Apps installieren“ für diese App und versuchen Sie es erneut.',
                        fr: 'Activez « Installer des applications inconnues » pour cette application, puis réessayez.',
                        ar: 'فعّل "تثبيت التطبيقات غير المعروفة" لهذا التطبيق ثم أعد المحاولة.',
                      })}
                    </span>
                  )}

                  {installState === 'error' && (
                    <span style={{ fontSize: 12, color: '#e05a5a' }}>
                      {text(activeLang, {
                        tr: 'İndirme başarısız oldu. Tekrar deneyin.',
                        en: 'Download failed. Try again.',
                        de: 'Download fehlgeschlagen. Versuchen Sie es erneut.',
                        fr: 'Échec du téléchargement. Réessayez.',
                        ar: 'فشل التنزيل. حاول مرة أخرى.',
                      })}
                    </span>
                  )}
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
