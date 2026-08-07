# Changelog

Notable changes to Anatolia-Sim, newest first. Version numbers correspond to
`package.json`, bumped on merge to `main` per this repo's own versioning
policy (see `AGENTS.md`/`CLAUDE.md` → Versioning) — a documentation-only
push is the one exception and does not get its own version bump.

## [Unreleased]

- Rotated every credential referenced in `CLAUDE.md`'s Render environment
  backup and Android keystore-adjacent login section after a brief
  accidental public-visibility window let GitHub's, Google's, Resend's, and
  GitGuardian's secret scanners find several live keys; purged the real
  values from the entire git history (not just the current file) with
  `git filter-repo`
- Overhauled `README.md`'s structure (Research Context, Environment
  Variables, How It Works, Deployment, Performance, Security Notes,
  Roadmap, FAQ, Troubleshooting, Citation, this Changelog) and corrected
  its License section, which had drifted out of sync with the repo's
  actual `LICENSE.txt` (Proprietary, not MIT)

## [2.5.44]

- Capped the simulation tick loop's batch size by current population size
  (`population_capped_batch_size`) to stop a fast-growing young population
  from OOM-killing the Render instance mid-batch
- Fixed the SPA fallback returning a real HTTP 404 status on a hard
  refresh/deep link into a client route, and silently swallowed
  fast-forward validation errors in the client

## [2.5.40]

- Exposed the full 49-hormone breakdown in the generated civilization
  report instead of a truncated subset
- Fixed the live population/death stats undercounting on any long-running
  simulation once dead individuals aged out of the tick loop's bounded
  in-memory state (`total_ever_died`/`total_ever_born` are now dedicated
  monotonic counters, not live counts over a pruned array)
- Fixed a frozen live-watch screen specifically on Android's "Bulut"
  (cloud) mode, caused by the WebSocket host resolving to the Capacitor
  origin instead of the real cloud API
- Removed leftover Fly.io deployment remnants after the move back to Render

## [2.5.34]

- Fixed the admin panel failing to load under desktop/Android "Yerel"
  (local) mode
- Added hormone data to the generated report and surfaced each
  individual's own live hormone state in the Population panel, not just
  population-wide averages
- Expanded the dynamic hormone system from 20 to the full 49 hormones,
  closing the gap to the literature's standard ~40-60 hormone range

## [2.5.26 and earlier]

- Introduced the dynamic hormone system (`rust/sim-core/src/hormones.rs`),
  starting at 6 hormones and expanding to 20 across real HPA/HPT/HPG axes
- Moved production deployment back to Render from Fly.io
- Numerous earlier version bumps (2.5.15–2.5.25) correspond to CI/release
  workflow fixes (retriggering Android/Desktop builds after adding the
  required GitHub Actions secrets) rather than user-facing changes

---

Entries above summarize `git log`; for full detail on any change, see the
corresponding commit and, for engine-level mechanics, `AGENTS.md`.
