# Architecture

BitSabi is a small Rust workspace with three crates:

- `bitsabi-shared` — product constants and API types used by both sides.
- `bitsabi-server` — Axum HTTP API, configuration, and chain-data proxy.
- `bitsabi-web` — Dioxus WebAssembly application.

## Trust boundary

```text
Browser (Dioxus WASM)
  ├── lessons, quizzes, and device-local progress
  ├── mock payment interface
  ├── transient recovery phrases, BIP39 validation, and address derivation
  └── public-address scanning, balances, history, and fee display
               │
               │ public testnet addresses only
               ▼
Axum API
  ├── health
  ├── testnet wallet snapshot proxy
  └── testnet transaction broadcast proxy
               │
               └── Esplora-compatible Bitcoin testnet provider
```

The wallet is deliberately on the browser side of the trust boundary. The server
must never receive a recovery phrase, seed, private key, or private descriptor.
Only derived public addresses and already-signed transactions are sent to the
API. The server does not sign transactions and does not store wallet data.

## Current modules

- `crates/server/src/api` — route handlers and public-input validation.
- `crates/server/src/chain` — Esplora-compatible testnet client and response
  parsing.
- `crates/server/src/config` — environment-based configuration.
- `crates/web/src/api` — browser client for the BitSabi proxy.
- `crates/web/src/components` — shared layout, navigation, icons, QR, and UI
  pieces.
- `crates/web/src/education` — lesson content, quiz data, and device-local
  progress.
- `crates/web/src/wallet` — BIP39 validation, secure generation, BIP84 testnet
  address derivation, and the existing local send/signing flow.
- `crates/web/src/wallet_data` — wallet-level balance, history, and formatting
  aggregation.
- `crates/web/src/pages` — Home, Learn, lesson detail, Wallet, Payments, Settings,
  and Not Found pages.
- `crates/web/src/routes` — typed Dioxus routes.

Future modules will follow the remaining product areas: wallet persistence,
payments, and optional progress sync. They will be added only when a milestone
needs them.

## Chain data provider

The default provider is Blockstream's Esplora-compatible Bitcoin testnet API:

```text
https://blockstream.info/testnet/api
```

The URL is configuration, not a compile-time constant. `TESTNET_API_URL` sets the
primary provider and `TESTNET_API_FALLBACK_URL` optionally sets a second
Esplora-compatible provider. Both are validated as HTTP(S) URLs. `BITCOIN_NETWORK`
must be `testnet`; mainnet is rejected at startup.

The API proxies only public testnet data:

| Provider endpoint | Purpose |
|---|---|
| `GET /blocks/tip/height` | Confirmation counts |
| `GET /fee-estimates` | Fee recommendations for confirmation targets |
| `GET /address/{address}` | Confirmed and mempool balance totals |
| `GET /address/{address}/utxo` | Spendable outputs and their confirmation state |
| `GET /address/{address}/txs` | Recent transactions touching the address |
| `POST /tx` | Broadcast an already-signed transaction (existing flow) |

Requests time out after 15 seconds and use a BitSabi user agent. The client tries
the fallback provider when the primary provider is unreachable or returns a
non-success status. Provider responses are deserialized into narrow structs;
missing fields, malformed JSON, unexpected statuses, and network failures become
safe `ChainError` values. Provider error bodies are not exposed to the browser
except for a bounded message on broadcast rejection.

The browser sends a maximum of 50 unique testnet addresses per
`POST /api/testnet/wallet/snapshot` request. Larger scans are split into batches
and merged into one dashboard snapshot. The server rejects mainnet, malformed,
and empty address lists before contacting a provider.

## Balance calculation

Each address returns Esplora confirmed (`chain_stats`) and mempool
(`mempool_stats`) funded and spent totals. The browser aggregates:

```text
confirmed_sat = sum(chain_funded  - chain_spent)
unconfirmed_sat = sum(mempool_funded - mempool_spent)
total_sat = confirmed_sat + unconfirmed_sat
```

The UI displays confirmed, unconfirmed, and total balances in both BTC and
satoshis. Spending capacity is limited to confirmed UTXOs. All arithmetic uses
satoshis with checked or saturating conversion so provider values cannot overflow
the display. A reload replaces the previous snapshot only after a successful
response; failed refreshes keep the last good snapshot visible.

## Transaction history

The browser scans the derived receiving and change gap addresses (20 each by
default, up to 100 receiving addresses per session). For each address with
activity, the server fetches the provider's recent transaction list and computes
the net wallet effect from matching outputs and previous outputs. The browser
then:

- de-duplicates transactions that touch multiple wallet addresses;
- sums their net effects so internal transfers classify correctly;
- labels positive values as received, negative values as sent, and zero as a
  self-transfer;
- calculates confirmations as `max(tip_height - block_height + 1, 0)`;
- sorts pending transactions first, then confirmed transactions by height;
- shows BTC and satoshi amounts, block time when available, confirmation count,
  transaction ID, and a `mempool.space/testnet` explorer link.

Providers return a bounded recent window per address. If an address has more
transactions than that window, the UI shows a truncation warning rather than
pretending the history is complete.

## Fee estimation

The provider's `/fee-estimates` map is normalized into five targets:

| UI label | Target |
|---|---|
| Fastest | next block (`1`) |
| Normal | around 30 minutes (`3`) |
| Planned | around 1 hour (`6`) |
| Economy | around 1 day (`144`) |
| Minimum | around 1 week (`1008`) |

When an exact target is missing, the nearest provider target is used. Rates are
rounded up and clamped to a safe 1–1000 sat/vB range. Fee estimates are
informational and may change before confirmation.

Fee data is optional. If the provider fails or returns no usable estimates, the
snapshot still includes balances and history, and the UI shows an explicit
"fee estimates unavailable" state. The existing send flow requires a current
quote before preparing a transaction.

## Refreshing

Wallet data loads when a wallet session starts and can be refreshed manually at
any time. The dashboard also refreshes automatically every 60 seconds while a
wallet is open. In-flight requests are tracked so an automatic refresh does not
duplicate a manual refresh, and stale responses are ignored.

## Environment modes

Testnet, mock payments, and mainnet must remain visibly distinct. The MVP must
not enable mainnet or real payments. Configuration placeholders already exist in
`.env.example`.

## Remaining limitations

- Wallet sessions and keys are memory-only; refreshing the browser clears them.
- Chain data is only as reliable as the configured provider.
- Providers rate-limit public APIs, so aggressive polling is avoided.
- Transaction history is a recent window, not a complete archive.
- There is no server-side wallet database, authentication, payment integration,
  or NGN quote provider yet.
- Fee estimates are not a guarantee and no sending/signing work is added by the
  balance, history, and fee milestone.
