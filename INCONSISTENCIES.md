# DeltaSync — Known Inconsistencies

Findings from a docs refresh (2026-10-19), each verified in the repo. Delete entries as they land.

## Should fix

1. **The landing has no CI check before deploy** — `.github/workflows/ci.yml` runs only Rust and Android jobs; `deploy.yml` builds and ships the landing on every push to `main`, including docs-only pushes (no `paths` filter).

## Worth a look

2. **Unused Vercel config still ships** — `vercel.json` and `.vercelignore` describe a host that is no longer used.
3. **README "Project Structure" is a stub** — it lists only `app/`, `public/`, `package.json`, while `components/`, `lib/`, `server/`, `crates/`, `sdk/`, `trust/`, `deploy/` exist.
4. **Docs language is mixed** — `docs/RELEASES.md` and `docs/external-cameras.md` are Spanish; the rest are English.
