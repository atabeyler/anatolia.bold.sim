import { describe, it, expect, vi } from 'vitest';

vi.mock('./engineClient', () => ({
  engine: {
    getStats: vi.fn().mockResolvedValue({ population: 2, happiness_index: 0.5, gini: 0.2, qol_index: 0.5, max_language_stage: 1, word_count: 3 }),
    getEvents: vi.fn().mockResolvedValue([
      {
        event_type: 'migration',
        sim_day: 100,
        sim_year: 0,
        importance: 'medium',
        description: 'migration',
        data: {
          distance_km: 12.5,
          reason: 'food_scarcity',
          from: { x: 1, y: 2 },
          to: { x: 3, y: 4 },
          food_abundance: 0.2,
          water_abundance: 0.3,
          season: 'winter',
        },
      },
    ]),
  },
}));

import { buildReport } from './report';

function fakeState() {
  return {
    id: 'sim-1',
    name: 'Test Sim',
    status: 'running',
    current_year: 2,
    current_day: 730,
    start_latitude: 39.9,
    start_longitude: 32.8,
    world_state: { biome: 'grassland' },
    individuals: [],
    discovered_techs: [],
    discovered_beliefs: [],
    discovered_arts: [],
    belief_labels: {},
  };
}

describe('buildReport()', () => {
  it('includes a simulation object with coordinates/biome, matching the server /report shape', async () => {
    const report = await buildReport(fakeState(), []);
    expect(report.simulation).toEqual({
      id: 'sim-1',
      name: 'Test Sim',
      status: 'running',
      start_latitude: 39.9,
      start_longitude: 32.8,
      biome: 'grassland',
      current_year: 2,
      current_day: 730,
    });
  });

  it('flattens migration events into the { year, day, distance_km, reason, from, to, ... } shape ReportPanel.tsx expects', async () => {
    const report = await buildReport(fakeState(), []);
    expect(report.migration_history).toEqual([
      {
        year: 0,
        day: 100,
        distance_km: 12.5,
        reason: 'food_scarcity',
        from: { x: 1, y: 2 },
        to: { x: 3, y: 4 },
        food_abundance: 0.2,
        water_abundance: 0.3,
        season: 'winter',
      },
    ]);
  });
});
