import { beforeEach, describe, expect, it, vi } from 'vitest';

async function loadCloud(hostname: string, native = { android: false, yerel: false }) {
  vi.resetModules();
  vi.stubGlobal('window', { location: { hostname } } as any);
  vi.doMock('./nativeMode', () => ({
    isNativeAndroidApp: () => native.android,
    isYerelModeActive: () => native.yerel,
  }));
  return import('./cloud');
}

describe('cloud URL routing', () => {
  beforeEach(() => {
    vi.unstubAllGlobals();
  });

  it('keeps relative URLs on the cloud origin', async () => {
    const { authUrl, cloudUrl } = await loadCloud('example.com');
    expect(authUrl('/api/auth/login')).toBe('/api/auth/login');
    expect(cloudUrl('/api/admin/users')).toBe('/api/admin/users');
  });

  it('routes desktop local-origin cloud calls to the cloud', async () => {
    const { authUrl, cloudUrl, CLOUD_API_URL } = await loadCloud('localhost');
    expect(authUrl('/api/auth/login')).toBe(`${CLOUD_API_URL}/api/auth/login`);
    expect(cloudUrl('/api/admin/users')).toBe(`${CLOUD_API_URL}/api/admin/users`);
  });

  it('routes native Android Yerel cloud calls to the cloud', async () => {
    const { authUrl, cloudUrl, CLOUD_API_URL } = await loadCloud('example.com', { android: true, yerel: true });
    expect(authUrl('/api/auth/login')).toBe(`${CLOUD_API_URL}/api/auth/login`);
    expect(cloudUrl('/api/admin/users')).toBe(`${CLOUD_API_URL}/api/admin/users`);
  });
});
