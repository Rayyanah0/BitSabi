# Security notes

This document records the current security position and must be updated as wallet
or payment capabilities expand.

## Non-negotiable rules

- Recovery phrases and private keys never go to the API, database, logs, error
  messages, analytics, or third-party services.
- Wallet secrets are not used during development or testing. Tests use public
  BIP39 vectors only. Only testnet wallets and test coins are allowed.
- The server never signs Bitcoin transactions on behalf of a user.
- Mainnet is disabled in the MVP. Only testnet addresses are accepted by the API.
- Mock NGN payments are clearly labelled and never presented as real money.
- A frontend success message is not proof that a payment happened.

## Current state

The following exist today:

- A health endpoint.
- Static UI pages and navigation.
- Security response headers (`X-Content-Type-Options`, `X-Frame-Options`,
  `Referrer-Policy`).
- A CORS policy restricted to the configured frontend origins.
- Testnet-only wallet creation and import in the Dioxus browser app.
- Secure 12-word generation through `getrandom`, with failure returned as an
  error instead of a fallback.
- Local BIP39 validation and BIP84 native SegWit testnet address derivation.
- A recovery-phrase confirmation step and an address dashboard with clipboard
  copy and a QR code.
- Public-address wallet snapshots with confirmed, unconfirmed, and total balances.
- De-duplicated transaction history with confirmation counts and explorer links.
- Optional testnet fee estimates normalized into sat/vB confirmation targets.
- Manual refresh plus a 60-second automatic refresh while the wallet is open.
- An existing local send/signing flow that signs in the browser and sends only the
  signed transaction to the API for broadcast.

The API receives derived public testnet addresses and, when the existing send flow
is used, already-signed transactions. It never receives a recovery phrase, seed,
private key, or private descriptor. It does not store wallet data.

There is no mainnet support, wallet persistence, balance caching, authentication,
or payment integration yet. The app does not retain a signing key across a page
refresh.

## Wallet data handling

- Recovery phrases are held in browser memory only and are zeroized where
  practical.
- The seed phrase and private keys are never written to localStorage,
  sessionStorage, cookies, IndexedDB, logs, or API requests.
- Wallet snapshots contain only public testnet addresses and public chain data.
- Refresh errors are user-facing summaries; provider response bodies are not
  echoed into the UI except for bounded broadcast rejection messages.
- The app refreshes wallet data automatically every 60 seconds. Requests already
  in flight suppress duplicate automatic refreshes, and stale responses are
  ignored.
- Fee estimates are optional. A provider failure does not block balance or
  history display.

## Planned wallet protections

- Generate and restore wallets with BIP39 and the Rust Bitcoin ecosystem, not
  custom cryptography.
- Use the browser's cryptographically secure random source and fail closed if it
  is unavailable.
- Accept only 12- or 24-word English BIP39 imports and validate the checksum
  before deriving an address. Optional BIP39 passphrases are not supported in
  this milestone.
- Zeroize recovery phrases, entropy, and seeds where practical.
- Keep wallet secrets out of browser storage.
- Keep wallet derivation and signing separate from UI code and cover them with
  tests.
- Proxy only public testnet chain data through the API.
- Validate addresses and network before building or broadcasting a transaction.
- Show recovery-phrase warnings before wallet creation.
- Add a persistent, encrypted wallet only in a dedicated milestone with a
  documented key-storage design.

## Known browser-wallet limitations

Browser wallets can be harmed by XSS, malicious extensions, clipboard malware,
and compromised devices. Recovery phrases and derived keys can exist transiently
in WebAssembly and browser memory. Zeroization reduces exposure but cannot
guarantee removal from every memory, swap file, crash dump, or browser snapshot.

Chain balances and history come from a third-party public testnet provider. A
provider can be unavailable, rate-limited, or return incomplete data. The app
keeps the last successful snapshot after a refresh failure, but it cannot verify
the provider's view of the chain. Fee estimates are recommendations, not quotes,
and may change before confirmation. Transaction history is a recent provider
window and may be truncated.

This milestone does not make the app secure for real funds. A browser wallet
should not be described as secure merely because tests pass.

Before real funds are supported, BitSabi needs:

- A documented threat model and recovery procedure.
- Tested backup, restore, signing, and transaction-monitoring flows.
- An independent security review.
- A decision on hardware-wallet or external-signer support.

## Reporting

Do not open a public issue for a suspected secret leak or wallet vulnerability.
Report it privately to the project maintainers.
