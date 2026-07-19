# sadan.zip

personal site with 5000 other random things.

Hybrid **Rust (Cargo workspace) + TypeScript (React 19)** monorepo. The frontend runs on TanStack Start/Router/Query + Tailwind CSS v4 + Zustand, bundled with Vite 8 (rolldown) and deployed to Cloudflare Workers. The Rust `libsadancore` crate compiles to WASM and links into the frontend as `@sadan4/libsadancore`.

## Prerequisites

- **pnpm** (Node 24, see `.nvmrc`) — package manager, enforced via `packageManager`.
- **Rust nightly** (`rust-toolchain.toml`, edition 2024).
- A Nix flake + direnv provide the full dev shell (emscripten, wasm-bindgen, mold, clang). `direnv allow` to enter it.

## Development

```bash
cargo xtask run client
```

## Building

The frontend depends on the WASM crate being built first, so **build through xtask**, not plain `pnpm build`:

```bash
cargo xtask build client      # builds the WASM crate then the frontend
```

`cargo xtask` must be run from the workspace root. Other targets: `cargo xtask {build|run|gen|clean|package} {client|server|extension}`.

## Testing

- Rust: `cargo nextest run --all-targets --all-features` ([nextest](https://nexte.st/), not `cargo test`).

## Linting

No ESLint or Prettier. Linting is [oxlint](https://oxc.rs/docs/guide/usage/linter) + [stylelint](https://stylelint.org/) for JS/TS/CSS and clippy for Rust; formatting is enforced by oxlint / rustfmt.

```bash
pnpm lint              # tsc -b + stylelint + oxlint (concurrent)
pnpm lint:js:fix       # oxlint --fix
cargo clippy --all-targets --all-features
```

## Deployment

Push to the **`web`** branch (the production branch, not `main`) triggers the Cloudflare Workers deploy workflow. Manual deploy: `pnpm run deploy` (wrangler). Regen worker types with `pnpm typegen`.

## Layout

- `src/` — frontend: `components/` (incl. `components/layout/`), `routes/` (TanStack file-based routing; `routeTree.gen.ts` is generated — don't edit), `stores/` (Zustand), `hooks/`, `utils/`. Import alias `@/*` → `./src/*`.
- `crates/` — Rust workspace (`libsadancore` → WASM, plus reporter, LSP, pretty-printer, AST parsers).
- `packages/` — pnpm workspace packages (e.g. `VencordCompanion`).
