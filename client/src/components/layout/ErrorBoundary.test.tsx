import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import ErrorBoundary from './ErrorBoundary';
import { useSimStore } from '../../store/simStore';

function Bomb({ shouldThrow }: { shouldThrow: boolean }) {
  if (shouldThrow) throw new Error('boom');
  return <div>salim panel</div>;
}

describe('ErrorBoundary', () => {
  let consoleErrorSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    // React logs the caught error to console.error too; keep test output clean.
    consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    // Fallback text is language-dependent; pin it so assertions are deterministic.
    useSimStore.setState({ lang: 'tr' });
  });

  afterEach(() => {
    consoleErrorSpy.mockRestore();
  });

  it('hata fırlatmayan alt bileşeni normal şekilde render eder', () => {
    render(
      <ErrorBoundary name="Test">
        <Bomb shouldThrow={false} />
      </ErrorBoundary>
    );
    expect(screen.getByText('salim panel')).toBeInTheDocument();
  });

  it('alt bileşen hata fırlatınca tüm ağacı değil sadece kendi alanını düşürür', () => {
    render(
      <div>
        <div>diğer panel</div>
        <ErrorBoundary name="Biology">
          <Bomb shouldThrow={true} />
        </ErrorBoundary>
      </div>
    );
    // Kardeş bileşen hâlâ ayakta olmalı — tüm uygulama çökmemeli.
    expect(screen.getByText('diğer panel')).toBeInTheDocument();
    expect(screen.getByText(/Panel yüklenemedi/)).toBeInTheDocument();
  });

  it('panel adını hata mesajına dahil eder', () => {
    render(
      <ErrorBoundary name="Biology">
        <Bomb shouldThrow={true} />
      </ErrorBoundary>
    );
    expect(screen.getByText(/\(Biology\)/)).toBeInTheDocument();
  });

  it('TEKRAR DENE butonu hata durumunu sıfırlayıp yeniden render dener', () => {
    const { rerender } = render(
      <ErrorBoundary name="Test">
        <Bomb shouldThrow={true} />
      </ErrorBoundary>
    );
    expect(screen.getByText(/Panel yüklenemedi/)).toBeInTheDocument();

    rerender(
      <ErrorBoundary name="Test">
        <Bomb shouldThrow={false} />
      </ErrorBoundary>
    );
    fireEvent.click(screen.getByText('TEKRAR DENE'));
    expect(screen.getByText('salim panel')).toBeInTheDocument();
  });
});
