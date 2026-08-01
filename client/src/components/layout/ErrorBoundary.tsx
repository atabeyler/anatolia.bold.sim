import { Component, type ErrorInfo, type ReactNode } from 'react';
import { useSimStore } from '../../store/simStore';
import { text } from '../../utils/i18n';

interface Props {
  children: ReactNode;
  name?: string;
}

interface State {
  error: Error | null;
}

/**
 * Isolates a render crash to the subtree it wraps instead of taking down the
 * whole app (React unmounts the entire tree on an uncaught render error
 * otherwise). Each panel in SimulationPage gets its own instance so one
 * panel's bug can't blank-screen the rest of the simulation.
 */
export default class ErrorBoundary extends Component<Props, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error(`[ErrorBoundary${this.props.name ? `:${this.props.name}` : ''}]`, error, info.componentStack);
  }

  render() {
    if (this.state.error) {
      const lang = useSimStore.getState().lang;
      return (
        <div
          style={{
            position: 'fixed',
            bottom: 12,
            right: 12,
            zIndex: 9999,
            padding: '8px 12px',
            fontSize: 11,
            fontFamily: 'Share Tech Mono, monospace',
            color: '#c05050',
            background: 'rgba(20,10,10,0.9)',
            border: '1px solid #6a2020',
            borderRadius: 4,
            maxWidth: 260,
          }}
        >
          {text(lang, {
            tr: `Panel yüklenemedi${this.props.name ? ` (${this.props.name})` : ''}.`,
            en: `Panel failed to load${this.props.name ? ` (${this.props.name})` : ''}.`,
            de: `Panel konnte nicht geladen werden${this.props.name ? ` (${this.props.name})` : ''}.`,
            fr: `Échec du chargement du panneau${this.props.name ? ` (${this.props.name})` : ''}.`,
            ar: `فشل تحميل اللوحة${this.props.name ? ` (${this.props.name})` : ''}.`,
          })}
          <button
            onClick={() => this.setState({ error: null })}
            style={{
              display: 'block',
              marginTop: 6,
              fontSize: 10,
              padding: '3px 8px',
              border: '1px solid #6a2020',
              color: '#c05050',
              background: 'transparent',
              cursor: 'pointer',
              fontFamily: 'inherit',
            }}
          >
            {text(lang, { tr: 'TEKRAR DENE', en: 'RETRY', de: 'WIEDERHOLEN', fr: 'RÉESSAYER', ar: 'أعد المحاولة' })}
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
