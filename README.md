# BitSabi

> Understand Bitcoin. Take Control.

BitSabi is a beginner-friendly Bitcoin learning app for Nigerian users. It is
being built in small, tested milestones.

**Current status:** Milestones 0–3 — project setup, Axum API skeleton, responsive
navigation, Home dashboard, five lessons, quizzes, and device-local learning
progress. Wallet features and NGN payment quotes are not implemented yet.

## What works today

- Independent Git repository for BitSabi.
- Rust workspace with shared, server, and web crates.
- `GET /api/health` returns a small JSON health response.
- Responsive BitSabi interface with Home, Learn, Wallet, Payments, and Settings routes.
- Desktop sidebar navigation and mobile bottom navigation.
- Home dashboard with real progress and a clear next lesson.
- Five lessons with reading views, one quiz per lesson, and device-local progress.
- Clear notices that wallet features and payments are not complete.
- Security headers and a restrictive CORS origin for local development.

## What is not implemented

- Server-side progress sync; lesson progress is stored in this browser only.
- Wallet creation, recovery, addresses, QR codes, sending, or history.
- Real or mock BTC/NGN quotes.
- Authentication, PostgreSQL, and payment-provider integration.

Nothing in this version handles real money. Wallet features will be testnet-only.

## Technology

| Layer | Choice |
|---|---|
| API | Rust, Axum 0.8 |
| Web | Rust, Dioxus 0.7 WebAssembly |
| Database | PostgreSQL, introduced in Milestone 4 |
| Bitcoin | BDK and the Rust Bitcoin ecosystem, introduced in Milestone 6 |
| Testing | Rust unit/integration tests, browser tests later |

## Prerequisites

- Rust stable 1.83 or newer (`rustup` recommended)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- Dioxus CLI 0.7.10, matching the pinned Dioxus crate:
  `cargo install dioxus-cli --version 0.7.10 --locked`
  On Ubuntu without `pkg-config`/OpenSSL development packages, use the prebuilt
  `dx-x86_64-unknown-linux-gnu.tar.gz` binary from the Dioxus GitHub release.
- PostgreSQL 16 when Milestone 4 begins

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
cargo fmt --check
```

Browser checks are run manually during UI milestones. Automated end-to-end tests
will be added with the wallet milestones.

## Project structure

```text
crates/shared/   Shared product constants and API types
crates/server/   Axum API, configuration, and route handlers
crates/web/      Dioxus WebAssembly application
docs/            Architecture and security notes
```

The wallet will live entirely in the browser. The API may relay public Bitcoin
testnet data, but it must never receive a recovery phrase or private key. See
[`docs/security.md`](docs/security.md).

## Roadmap

1. ~~Project setup and responsive shell~~
2. ~~Home dashboard and navigation polish~~
3. ~~Five lessons and quizzes~~
4. ~~Device-local lesson progress and Settings~~ (current)
5. Clearly labelled mock NGN quotes
6. Testnet wallet creation, import, and receiving
7. Testnet sending, fees, history, and error handling
8. Automated tests, security review, documentation, deployment preparation

Mainnet and real NGN payments are explicitly out of scope until the wallet has
been reviewed and a licensed payment provider has been selected.
