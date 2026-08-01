import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const bumpType = process.argv[2] ?? 'patch';

function bumpSemver(version, type) {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(version);
  if (!match) throw new Error(`Invalid semver: ${version}`);
  const major = Number(match[1]);
  const minor = Number(match[2]);
  const patch = Number(match[3]);
  if (type === 'major') return `${major + 1}.0.0`;
  if (type === 'minor') return `${major}.${minor + 1}.0`;
  if (type === 'patch') return `${major}.${minor}.${patch + 1}`;
  if (/^\d+\.\d+\.\d+$/.test(type)) return type;
  throw new Error(`Unsupported version bump: ${type}`);
}

function readJson(file) {
  return JSON.parse(readFileSync(resolve(file), 'utf8'));
}

function writeJson(file, value) {
  writeFileSync(resolve(file), `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function updateText(file, replacer) {
  const abs = resolve(file);
  const next = replacer(readFileSync(abs, 'utf8'));
  writeFileSync(abs, next, 'utf8');
}

const rootPkgPath = 'package.json';
const rootPkg = readJson(rootPkgPath);
const nextVersion = bumpSemver(rootPkg.version, bumpType);

rootPkg.version = nextVersion;
writeJson(rootPkgPath, rootPkg);

const rootLockPath = 'package-lock.json';
const rootLock = readJson(rootLockPath);
rootLock.version = nextVersion;
if (rootLock.packages?.['']) {
  rootLock.packages[''].version = nextVersion;
}
writeJson(rootLockPath, rootLock);

const clientPkgPath = 'client/package.json';
const clientPkg = readJson(clientPkgPath);
clientPkg.version = nextVersion;
writeJson(clientPkgPath, clientPkg);

const clientLockPath = 'client/package-lock.json';
const clientLock = readJson(clientLockPath);
clientLock.version = nextVersion;
if (clientLock.packages?.['']) {
  clientLock.packages[''].version = nextVersion;
}
writeJson(clientLockPath, clientLock);

updateText('desktop/src-tauri/Cargo.toml', (text) => text.replace(/^version = ".*"$/m, `version = "${nextVersion}"`));
updateText('desktop/src-tauri/Cargo.lock', (text) =>
  text.replace(/(name = "anatolia-sim-desktop"\r?\nversion = ")[^"]+(")/, `$1${nextVersion}$2`),
);

console.log(nextVersion);
