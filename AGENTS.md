# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Rust (Cargo workspace, nightly) + TypeScript (React 19) monorepo. A Discord/Vencord webpack explorer / AST-viewer web app plus Rust tooling (LSP, reporter, pretty-printer, AST parsers). Frontend: TanStack Start/Router/Query + Tailwind v4 + Zustand, bundled with Vite 8 (rolldown), deployed to Cloudflare Workers. The Rust `libsadancore` crate compiles to WASM and links into the frontend as `@sadan4/libsadancore`.

**README.md is stale TanStack starter boilerplate** — it references `pnpm format`, `pnpm check`, ESLint+Prettier that do NOT exist. Ignore it; use this file.

## Build / test / lint

Package manager: **pnpm** (enforced; Node 24 via `.nvmrc`). Rust: nightly (`rust-toolchain.toml`, edition 2024). Nix flake + direnv provide the dev shell.

- **Build the site**: `cargo xtask build client` — NOT plain `pnpm build`. The frontend needs the WASM crate built first; `pnpm build` alone fails without the `@sadan4/libsadancore` artifact. `cargo xtask` must run from the workspace root.
- Other xtask targets: `cargo xtask {build|run|gen|clean|package} {client|server|extension}`.
- **JS tests**: `pnpm test` (Vitest). **Rust tests**: `cargo nextest run --all-targets --all-features` (nextest, not `cargo test`).
- **Lint JS/TS**: `pnpm lint` runs `tsc -b` + `stylelint` + `oxlint` concurrently. Fix: `pnpm lint:js:fix` (oxlint --fix). There is NO Prettier.
- **Lint Rust**: `cargo clippy --all-targets --all-features` (pedantic; `undocumented_unsafe_blocks` / `missing_safety_doc` are deny).
- Deploy: `pnpm run deploy` (wrangler). Regen worker types: `pnpm typegen`.

## Code style (differs from defaults)

TypeScript (enforced by oxlint, config `oxlint.config.ts`):
- **4-space indent**, **double quotes**, semicolons required, trailing commas multiline, max-len 120.
- **`interface` over `type`** for object definitions. Enforced consistent type imports/exports.
- Import alias **`@/*` → `./src/*`**. Imports auto-sorted (simple-import-sort); unused imports error.
- CSS modules must be imported as namespace (custom oxlint rule `require-css-as-namespace`).

Rust (`.rustfmt.toml`): **hard tabs**, max_width 80, `imports_granularity = Crate`.

## Layout & conventions

- `src/`: `components/` (incl. `components/layout/`), `routes/` (TanStack file-based routing — `routeTree.gen.ts` is generated, do not edit), `stores/` (Zustand), `hooks/`, `utils/`.
- **Deploy/production branch is `web`, not `main`.** Push to `web` triggers the Cloudflare deploy workflow. One-branch-per-feature; PRs run clippy + nextest.
