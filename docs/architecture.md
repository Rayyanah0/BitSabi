# Architecture

BitSabi is a small Rust workspace with three crates:

- `bitsabi-shared` — product constants and API types used by both sides.
- `bitsabi-server` — Axum HTTP API, configuration, and future database / payment
  modules.
- `bitsabi-web` — Dioxus WebAssembly application.

## Trust boundary

```text
Browser (Dioxus WASM)
  ├── lessons, quizzes, and device-local progress
  ├── mock payment interface
  └── wallet keys, signing, encrypted local storage
               │
               │ public data only
               ▼
Axum API
  ├── health
  ├── planned mock payment quotes
  └── planned testnet chain-data proxy
               │
               ├── PostgreSQL (non-sensitive metadata)
               └── Bitcoin testnet provider
```

The wallet is deliberately on the browser side of the trust boundary. The server
must never receive a recovery phrase, private key, or private descriptor.

## Current modules

- `crates/server/src/api` — route handlers.
- `crates/server/src/config` — environment-based configuration.
- `crates/web/src/components` — shared layout, navigation, icons, and UI pieces.
- `crates/web/src/education` — lesson content, quiz data, and device-local progress.
- `crates/web/src/pages` — Home, Learn, lesson detail, Wallet, Payments, Settings,
  and Not Found pages.
- `crates/web/src/routes` — typed Dioxus routes.

Future modules will follow the remaining product areas: wallet, bitcoin/chain,
payments, and optional progress sync. They will be added only when a milestone
needs them.

## Environment modes

Testnet, mock payments, and mainnet must remain visibly distinct. The MVP must
not enable mainnet or real payments. Configuration placeholders already exist in
`.env.example`.
