# Repository Guidelines

## Project Structure & Module Organization
`src/` holds the TypeScript/React core plus ANTLR grammars in `src/g4/` whose generated output belongs in `src/generated-parser/`. Static assets stay in `public/`, while demos and docs live in `playground/` and `docs/`. Bun specs reside in `test/unit/` (fixtures in `test/`); Playwright suites and snapshots live in `tests/`. Tooling sits in `scripts/`, `antlr/`, `types/`, and the repo-level configs (including `wrangler.toml`).

## Build, Test, and Development Commands
- `bun install` – install dependencies (Bun 1.x, Node ≥20).
- `bun run dev` – Vite dev server on `http://localhost:8080`.
- `bun run build` – library bundle via `vite.config.lib.ts`.
- `bun run build:site` / `bun run build:gh-pages` – demo/docs build (standard vs GitHub Pages).
- `bun run test` – Bun unit tests covering `src` and `test/unit`.
- `bun run pw`, `bun run pw:smoke`, `bun run pw:update` – Playwright full suite, smoke subset, snapshot refresh.
- `bun run worker:dev` / `worker:deploy(:staging)` – build site then run or ship the Cloudflare Worker.
- `bun run antlr:lexer && bun run antlr:parser` (plus `antlr:clear`) – regenerate parser artifacts after grammar edits.

## Coding Style & Naming Conventions
Stick to 2-space indentation, TypeScript, and arrow components with PascalCase names. Hooks/atoms stay camelCase (`modeAtom`), constants SCREAMING_SNAKE_CASE, and specs follow `FeatureName.spec.ts`. Scope Tailwind utilities under `.zenuml`; styles belong in `src/assets/tailwind.css` or the relevant component SCSS. Run `bun run eslint` and `bun run prettier` before committing.

## Testing Guidelines
Touching parser, rendering, or positioning code requires matching `*.spec.ts/tsx` updates, with fixtures held in `test/unit/parser/fixture`. Keep Bun tests deterministic (fake timers, no global state leaks) and prefer small diagrams. UI or regression work belongs in `tests/*.spec.ts` with stable selectors; refresh snapshots via `pw:update`. Every fix ships with a failing test first and, when visuals change, the updated snapshot artifacts.

## Commit & Pull Request Guidelines
Follow the conventional pattern (`fix:`, `feat:`, `refactor:`, etc.) and reference issues using `(#123)` in the subject. PRs summarize intent, list the verification commands (`bun run test`, `bun run pw:smoke`, etc.), and attach screenshots or GIFs for UI updates. Call out parser regeneration, config edits, or worker deploy steps so reviewers can verify the correct artifacts.

## Security & Configuration Tips
Keep secrets out of git—Wrangler reads Cloudflare credentials from your local profile, and environment overrides stay in untracked `.env`. Document worker route or KV changes in the PR, verify them in staging, and scrub proprietary diagrams before committing docs or tests.
