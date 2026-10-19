# DeltaSync — Known Inconsistencies

Findings from a docs refresh (2026-10-19), each verified in the repo. Delete entries as they land.

## Should fix

1. **The landing has no CI check before deploy** — `.github/workflows/ci.yml` runs only Rust and Android jobs; `deploy.yml` builds and ships the landing on every push to `main`, including docs-only pushes (no `paths` filter).

## Worth a look

2. **Legacy Vercel/GoDaddy docs still ship** — `DEPLOYMENT.md` and `DOMAIN_SETUP.md` (plus `vercel.json`, `.vercelignore`) describe a host that is no longer used; their headers say so, but the bodies are still step-by-step Vercel instructions.
3. **README "Project Structure" is a stub** — it lists only `app/`, `public/`, `package.json`, while `components/`, `lib/`, `server/`, `crates/`, `sdk/`, `trust/`, `deploy/` exist.
4. **Docs language is mixed** — `docs/RELEASES.md`, `docs/UNIFICATION.md` and `docs/external-cameras.md` are Spanish; the rest are English.
5. **`docs/UNIFICATION.md` is a plan whose PRs are all merged** — by the repo's convention it should be distilled and deleted once shipped; the Social-side adoption it mentions is the only open part.
