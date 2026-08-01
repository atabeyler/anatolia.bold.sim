import { useEffect, useState } from 'react';
import DetailPanel from './DetailPanel';
import { useSimStore } from '../../store/simStore';
import axios from 'axios';
import { Clock, RefreshCw, Save } from 'lucide-react';
import { text, type LangCode } from '../../utils/i18n';

export default function TimeMachinePanel() {
  const { currentSim, accessToken, lang, setCurrentSim, setStats, setEvents } = useSimStore();
  const t = (tr: string, en: string, de = en, fr = en, ar = en) => text(lang as LangCode, { tr, en, de, fr, ar });
  const [checkpoints, setCheckpoints] = useState<any[]>([]);
  const [restoring, setRestoring] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState('');

  function flash(txt: string) {
    setMsg(txt);
    setTimeout(() => setMsg(''), 5000);
  }

  async function loadCheckpoints() {
    if (!currentSim || !accessToken) return;
    axios.get(`/api/simulations/${currentSim.id}/checkpoints`, { headers: { Authorization: `Bearer ${accessToken}` } })
      .then(r => setCheckpoints(r.data))
      .catch(() => setCheckpoints([]));
  }

  useEffect(() => { loadCheckpoints(); }, [currentSim?.id]);

  async function saveNow() {
    if (!currentSim || !accessToken) return;
    setSaving(true);
    try {
      await axios.post(`/api/simulations/${currentSim.id}/checkpoint`, {}, { headers: { Authorization: `Bearer ${accessToken}` } });
      flash(t('✓ Kontrol noktası kaydedildi.', '✓ Checkpoint saved.', '✓ Kontrollpunkt gespeichert.', '✓ Point de contrôle sauvegardé.', '✓ تم حفظ نقطة التحقق.'));
      await loadCheckpoints();
    } catch {
      flash(t('✗ Kayıt başarısız.', '✗ Save failed.', '✗ Speichern fehlgeschlagen.', '✗ Échec de la sauvegarde.', '✗ فشل الحفظ.'));
    }
    setSaving(false);
  }

  async function restore(cpId: string) {
    if (!currentSim) return;
    setRestoring(cpId);
    const headers = { Authorization: `Bearer ${accessToken}` };
    try {
      await axios.post(`/api/simulations/${currentSim.id}/restore/${cpId}`, {}, { headers });
      // Restoring rewinds sim_day/sim_year, population and history server-side,
      // but nothing pushes that back into the client -- without refetching here,
      // every panel (stats, events, header day/year) stayed frozen on the
      // pre-restore state until the next WebSocket tick or a manual page reload.
      const [simRes, statsRes, eventsRes] = await Promise.allSettled([
        axios.get(`/api/simulations/${currentSim.id}`, { headers }),
        axios.get(`/api/simulations/${currentSim.id}/stats`, { headers }),
        axios.get(`/api/simulations/${currentSim.id}/events?limit=100`, { headers }),
      ]);
      if (simRes.status === 'fulfilled') setCurrentSim(simRes.value.data);
      if (statsRes.status === 'fulfilled') setStats(statsRes.value.data);
      if (eventsRes.status === 'fulfilled') setEvents(eventsRes.value.data);
      await loadCheckpoints();
      flash(t('✓ Geri yüklendi.', '✓ Restored.', '✓ Wiederhergestellt.', '✓ Restauré.', '✓ تمت الاستعادة.'));
    } catch {
      flash(t('✗ Geri yükleme başarısız.', '✗ Restore failed.', '✗ Wiederherstellung fehlgeschlagen.', '✗ Échec de la restauration.', '✗ فشلت الاستعادة.'));
    }
    setRestoring(null);
  }

  return (
    <DetailPanel panelId="timemachine" title="Time Machine" titleTr="Zaman Makinesi">
      <div className="bg-sim-surface rounded-lg p-3 mb-3 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <Clock size={24} className="text-sim-accent" />
          <div>
            <div className="text-sim-accent font-bold text-lg">{checkpoints.length}</div>
            <div className="text-sim-muted text-sm">
              {t('Kayıtlı Kontrol Noktaları', 'Saved Checkpoints', 'Gespeicherte Kontrollpunkte', 'Points de contrôle sauvegardés', 'نقاط التحقق المحفوظة')}
            </div>
          </div>
        </div>
        <button
          onClick={saveNow}
          disabled={saving}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded border border-sim-accent/50 bg-sim-accent/10 hover:bg-sim-accent/25 text-sim-accent transition-colors text-sm font-share-tech"
        >
          <Save size={13} className={saving ? 'animate-pulse' : ''} />
          {t('Şimdi Kaydet', 'Save Now', 'Jetzt speichern', 'Sauvegarder', 'احفظ الآن')}
        </button>
      </div>

      <p className="text-sim-muted text-sm italic mb-3">
        {t(
          'Kontrol noktaları yalnızca "Şimdi Kaydet" ile oluşturulur — otomatik kayıt yoktur. Herhangi bir kaydedilmiş duruma geri dönebilirsiniz.',
          'Checkpoints are only ever created via "Save Now" — there is no automatic save. Restore any saved state.',
          'Kontrollpunkte werden nur über "Jetzt speichern" erstellt — es gibt keine automatische Speicherung. Jeden gespeicherten Zustand wiederherstellen.',
          'Les points de contrôle ne sont créés que via « Sauvegarder maintenant » — il n\'y a pas de sauvegarde automatique. Restaurez n\'importe quel état sauvegardé.',
          'تُنشأ نقاط التحقق فقط عبر "احفظ الآن" — لا يوجد حفظ تلقائي. يمكنك استعادة أي حالة محفوظة.'
        )}
      </p>

      {msg && (
        <div className="bg-sim-accent/20 border border-sim-accent/40 rounded px-3 py-2 text-sm text-sim-text mb-3">
          {msg}
        </div>
      )}

      {checkpoints.length === 0 ? (
        <div className="text-center py-8">
          <Clock size={32} className="text-sim-border mx-auto mb-2" />
          <p className="text-sim-muted italic text-sm">
            {t('Henüz kontrol noktası yok. Simülasyonu çalıştırın veya manuel kaydedin.', 'No checkpoints yet. Run the simulation or save manually.', 'Noch keine Kontrollpunkte. Simulation starten oder manuell speichern.', 'Pas encore de points de contrôle.', 'لا نقاط تحقق بعد.')}
          </p>
        </div>
      ) : (
        <div className="space-y-2 max-h-80 overflow-y-auto">
          {checkpoints.map(cp => (
            <div key={cp.id} className="bg-sim-surface rounded-lg p-3 flex items-center justify-between border border-sim-border hover:border-sim-accent/40 transition-colors">
              <div>
                <div className="text-sm font-medium text-sim-text">
                  {t('Yıl', 'Year', 'Jahr', 'Année', 'سنة')} {cp.sim_year} · {t('Gün', 'Day', 'Tag', 'Jour', 'يوم')} {cp.sim_day}
                </div>
                <div className="text-sm text-sim-muted">
                  {t('Nüfus:', 'Pop:', 'Bev.:', 'Pop.:', 'سكان:')} {cp.population_count}
                </div>
              </div>
              <button
                onClick={() => restore(cp.id)}
                disabled={restoring === cp.id}
                className="flex items-center gap-1 px-2 py-1 rounded bg-sim-accent/20 hover:bg-sim-accent/40 text-sim-accent transition-colors text-sm"
              >
                <RefreshCw size={11} className={restoring === cp.id ? 'animate-spin' : ''} />
                {t('Geri Al', 'Restore', 'Wiederherstellen', 'Restaurer', 'استعادة')}
              </button>
            </div>
          ))}
        </div>
      )}
    </DetailPanel>
  );
}
