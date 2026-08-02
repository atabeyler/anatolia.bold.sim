import { describe, it, expect, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import PsychologyPanel from './PsychologyPanel';
import { useSimStore } from '../../store/simStore';

describe('PsychologyPanel hormone breakdown', () => {
  beforeEach(() => {
    useSimStore.setState({
      lang: 'en',
      activePanel: 'psychology',
      stats: {
        mean_hormones: {
          cortisol: 0.42,
          testosterone: 0.37,
          by_group: {
            female: { cortisol: 0.4, testosterone: 0.11 },
            male: { cortisol: 0.63, testosterone: 0.91 },
            child: { cortisol: 0.3, testosterone: 0.05 },
            adult: { cortisol: 0.5, testosterone: 0.55 },
            elderly: { cortisol: 0.55, testosterone: 0.2 },
          },
        },
      } as any,
    });
  });

  it('shows the overall population average by default', () => {
    render(<PsychologyPanel />);
    expect(screen.getByText('42%')).toBeInTheDocument();
  });

  it('switches to a sex/age breakdown when a group chip is clicked', () => {
    render(<PsychologyPanel />);
    fireEvent.click(screen.getByText('Male'));
    // Male-group testosterone (0.91) should now be displayed instead of the overall (0.37).
    expect(screen.getByText('91%')).toBeInTheDocument();
    expect(screen.queryByText('37%')).not.toBeInTheDocument();
  });

  it('renders every breakdown group chip', () => {
    render(<PsychologyPanel />);
    for (const label of ['Overall', 'Female', 'Male', 'Child', 'Adult', 'Elderly']) {
      expect(screen.getByText(label)).toBeInTheDocument();
    }
  });
});
