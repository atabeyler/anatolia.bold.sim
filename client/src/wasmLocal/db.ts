// IndexedDB persistence for WASM-local simulations -- the browser-only
// counterpart to sim-server's sqlite backend. No simulation server involved:
// every record here lives entirely in this browser profile until the user
// explicitly exports it or uploads it to the cloud (the account itself is
// real -- see BrowserModeGate.tsx -- via the existing, unchanged
// /api/simulations/:id/upload-to-cloud endpoint).
const DB_NAME = 'anatolia_wasm_local';
const DB_VERSION = 1;
const SIM_STORE = 'simulations';
const CHECKPOINT_STORE = 'checkpoints';

export interface StoredSimRecord {
  id: string;
  name: string;
  status: 'running' | 'paused' | 'completed';
  current_day: number;
  current_year: number;
  start_latitude: number;
  start_longitude: number;
  speed_multiplier: number;
  stateJson: string;
  created_at: number;
  updated_at: number;
}

export interface StoredCheckpoint {
  id?: number;
  simulation_id: string;
  sim_day: number;
  sim_year: number;
  population_count: number;
  stats: Record<string, unknown>;
  stateJson: string;
  created_at: number;
}

let dbPromise: Promise<IDBDatabase> | null = null;

function openDb(): Promise<IDBDatabase> {
  if (dbPromise) return dbPromise;
  dbPromise = new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(SIM_STORE)) {
        db.createObjectStore(SIM_STORE, { keyPath: 'id' });
      }
      if (!db.objectStoreNames.contains(CHECKPOINT_STORE)) {
        const store = db.createObjectStore(CHECKPOINT_STORE, { keyPath: 'id', autoIncrement: true });
        store.createIndex('simulation_id', 'simulation_id', { unique: false });
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error ?? new Error('IndexedDB open failed'));
  });
  return dbPromise;
}

function promisify<T>(request: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error('IndexedDB request failed'));
  });
}

export async function dbSaveSimulation(record: StoredSimRecord): Promise<void> {
  const db = await openDb();
  const tx = db.transaction(SIM_STORE, 'readwrite');
  tx.objectStore(SIM_STORE).put(record);
  await promisify(tx.objectStore(SIM_STORE).get(record.id));
}

export async function dbLoadSimulation(id: string): Promise<StoredSimRecord | undefined> {
  const db = await openDb();
  const tx = db.transaction(SIM_STORE, 'readonly');
  return promisify(tx.objectStore(SIM_STORE).get(id));
}

export async function dbListSimulations(): Promise<StoredSimRecord[]> {
  const db = await openDb();
  const tx = db.transaction(SIM_STORE, 'readonly');
  return promisify(tx.objectStore(SIM_STORE).getAll());
}

export async function dbDeleteSimulation(id: string): Promise<void> {
  const db = await openDb();
  const tx = db.transaction([SIM_STORE, CHECKPOINT_STORE], 'readwrite');
  tx.objectStore(SIM_STORE).delete(id);
  const checkpoints = await promisify(tx.objectStore(CHECKPOINT_STORE).index('simulation_id').getAllKeys(id));
  for (const key of checkpoints) tx.objectStore(CHECKPOINT_STORE).delete(key);
}

export async function dbCreateCheckpoint(checkpoint: StoredCheckpoint): Promise<number> {
  const db = await openDb();
  const tx = db.transaction(CHECKPOINT_STORE, 'readwrite');
  const key = await promisify(tx.objectStore(CHECKPOINT_STORE).add(checkpoint));
  return key as number;
}

export async function dbListCheckpoints(simulationId: string): Promise<StoredCheckpoint[]> {
  const db = await openDb();
  const tx = db.transaction(CHECKPOINT_STORE, 'readonly');
  const rows = await promisify(tx.objectStore(CHECKPOINT_STORE).index('simulation_id').getAll(simulationId));
  return rows.sort((a, b) => a.sim_day - b.sim_day);
}

export async function dbGetCheckpoint(checkpointId: number): Promise<StoredCheckpoint | undefined> {
  const db = await openDb();
  const tx = db.transaction(CHECKPOINT_STORE, 'readonly');
  return promisify(tx.objectStore(CHECKPOINT_STORE).get(checkpointId));
}

// Rough IndexedDB usage estimate for a WASM-local counterpart of
// GET /:id/db-status -- StorageManager.estimate() isn't available in every
// browser (notably not in Safari's private mode), so this degrades to nulls
// rather than throwing.
export async function dbEstimateUsage(): Promise<{ usage: number | null; quota: number | null }> {
  if (!navigator.storage?.estimate) return { usage: null, quota: null };
  try {
    const { usage, quota } = await navigator.storage.estimate();
    return { usage: usage ?? null, quota: quota ?? null };
  } catch {
    return { usage: null, quota: null };
  }
}
