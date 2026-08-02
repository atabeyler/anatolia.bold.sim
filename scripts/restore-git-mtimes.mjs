// actions/checkout resets every file's modification time to "now" (checkout
// time), even when the file's actual content is byte-identical to a prior
// checkout of the same commit. Cargo's own fingerprint/dirty-check is
// partially mtime-based, so a freshly-checked-out source tree can look
// "newer" than a perfectly-restored, byte-identical compiled target/ cache
// (Swatinem/rust-cache's own restore can report "full match: true" and
// still be followed by cargo recompiling crates that never actually
// changed) -- this cost ~3 minutes of pure waste on Desktop Release's
// sim-core/sim-server rebuild alone, confirmed by comparing two consecutive
// runs' own logs (one showed a full rust-cache restore immediately followed
// by real "Compiling sim-core"/"Compiling sim-server" lines with no source
// changes between them).
//
// This restores each tracked file's mtime to the timestamp of the last
// commit that actually touched it (always older than whenever a prior CI
// run's cargo build produced its cached target/ artifacts), so cargo's own
// "is source newer than the compiled output" check comes out correctly
// negative and skips the unnecessary recompile. Deliberately best-effort:
// any failure here should degrade to "no speedup" (the previous, already-
// working behavior), never break the build -- see the top-level try/catch.
import { execFileSync } from 'child_process';
import { existsSync, utimesSync } from 'fs';

function main() {
  // --reverse: earlier commits are read first, so a later commit's entry
  // for the same path naturally overwrites it in the Map below, leaving
  // each path's *most recent* touching commit's timestamp.
  const log = execFileSync('git', ['log', '--reverse', '--format=%x00%cI', '--name-only'], {
    encoding: 'utf-8',
    maxBuffer: 1024 * 1024 * 512,
  });

  const mtimeByPath = new Map();
  let currentTimestamp = null;
  for (const line of log.split('\n')) {
    if (line.startsWith('\x00')) {
      currentTimestamp = line.slice(1);
    } else if (line.trim() !== '' && currentTimestamp) {
      mtimeByPath.set(line, currentTimestamp);
    }
  }

  let touched = 0;
  for (const [path, iso] of mtimeByPath) {
    if (!existsSync(path)) continue; // deleted since, or not part of this checkout
    const date = new Date(iso);
    if (Number.isNaN(date.getTime())) continue;
    utimesSync(path, date, date);
    touched++;
  }
  console.log(`restore-git-mtimes: restored ${touched} file mtimes from git history.`);
}

try {
  main();
} catch (err) {
  console.warn('restore-git-mtimes: skipped (non-fatal) --', err?.message ?? err);
}
