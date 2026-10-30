# BitSabi

> Understand Bitcoin. Take Control.

BitSabi is a beginner-friendly Bitcoin learning app for Nigerian users. It is
being built in small, tested milestones.

**Current status:** Milestones 0–4 plus the testnet wallet creation/import and
receiving-address foundation. Sending, wallet persistence, and NGN payment quotes
are not implemented yet.

## What works today

- Independent Git repository for BitSabi.
- Rust workspace with shared, server, and web crates.
- `GET /api/health` returns a small JSON health response.
- Responsive BitSabi interface with Home, Learn, Wallet, Payments, and Settings routes.
- Desktop sidebar navigation and mobile bottom navigation.
- Home dashboard with real progress and a clear next lesson.
- Five lessons with reading views, one quiz per lesson, and device-local progress.
- Testnet wallet creation with 12 secure recovery words and BIP84 address derivation.
- Testnet wallet import with local 12/24-word BIP39 validation.
- A receiving-address dashboard with clipboard copy and explicit testnet warnings.
- Clear labels that wallet practice is testnet-only and naira payments are simulated.
- Security headers and a restrictive CORS origin for local development.

## What is not implemented

- Server-side progress sync; lesson progress is stored in this browser only.
- Persistent wallets, wallet balances, QR codes, sending, transaction history, or fees.
- Real or mock BTC/NGN quotes.
- Authentication, PostgreSQL, and payment-provider integration.

Nothing in this version handles real money. Wallet features are testnet-only.
Wallet phrases are never persisted or sent to the API; refreshing the page clears
the receiving address.

## Technology

| Layer | Choice |
|---|---|
| API | Rust, Axum 0.8 |
| Web | Rust, Dioxus 0.7 WebAssembly |
| Database | PostgreSQL, planned for a later persistence milestone |
| Bitcoin | Rust Bitcoin and BIP39 for testnet address derivation; BDK not integrated yet |
| Testing | Rust unit tests; manual browser checks during UI milestones |

## Prerequisites

- Rust stable 1.83 or newer (`rustup` recommended)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- Dioxus CLI 0.7.10, matching the pinned Dioxus crate:
  `cargo install dioxus-cli --version 0.7.10 --locked`
  On Ubuntu without `pkg-config`/OpenSSL development packages, use the prebuilt
  `dx-x86_64-unknown-linux-gnu.tar.gz` binary from the Dioxus GitHub release.
- PostgreSQL 16 when persistence work begins

## Local setup

```bash
cp .env.example .env
```

Review `.env` and keep it out of Git. The defaults are for local development.

### Run the API

```bash
cargo run -p bitsabi-server
```

The API listens on `http://127.0.0.1:3000` by default. Verify it with:

```bash
curl http://127.0.0.1:3000/api/health
```

### Run the web app

In a second terminal:

```bash
dx serve --package bitsabi-web --web
```

Open the URL printed by the Dioxus CLI, usually `http://localhost:8080`.

## Tests and checks

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

Wallet tests cover secure generation failure, 12/24-word imports, BIP39 checksum
errors, deterministic derivation, and testnet address formatting. Browser checks
are run manually during UI milestones; automated end-to-end tests are not set up yet.

## Project structure

```text
crates/shared/   Shared product constants and API types
crates/server/   Axum API, configuration, and route handlers
crates/web/      Dioxus WebAssembly application
docs/            Architecture and security notes
```

The wallet runs entirely in the browser. The API may later relay public Bitcoin
testnet data, but it must never receive a recovery phrase, seed, or private key. See
[`docs/security.md`](docs/security.md).

## Roadmap

1. ~~Project setup and responsive shell~~
2. ~~Home dashboard and navigation polish~~
3. ~~Five lessons and quizzes~~
4. ~~Device-local lesson progress and Settings~~
5. Clearly labelled mock NGN quotes
6. ~~Testnet wallet creation, import, and receiving~~ (current)
7. Testnet sending, fees, history, and error handling
8. Automated tests, security review, documentation, deployment preparation

Mainnet and real NGN payments are explicitly out of scope until the wallet has
been reviewed and a licensed payment provider has been selected.
