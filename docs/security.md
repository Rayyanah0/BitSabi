# Security notes

This document records the current security position and must be updated as wallet
or payment capabilities expand.

## Non-negotiable rules

- Recovery phrases and private keys never go to the API, database, logs, error
  messages, analytics, or third-party services.
- Wallet secrets are not used during development or testing. Tests use public
  BIP39 vectors only. Only testnet wallets and test coins are allowed.
- The server never signs Bitcoin transactions on behalf of a user.
- Mainnet is disabled in the MVP.
- Mock NGN payments are clearly labelled and never presented as real money.
- A frontend success message is not proof that a payment happened.

## Current state

The following exist today:

- A health endpoint.
- Static UI pages and navigation.
- Security response headers (`X-Content-Type-Options`, `X-Frame-Options`,
  `Referrer-Policy`).
- A CORS policy restricted to the configured frontend origin.
- Testnet-only wallet creation and import in the Dioxus browser app.
- Secure 12-word generation through `getrandom`, with failure returned as an
  error instead of a fallback.
- Local BIP39 validation and BIP84 native SegWit testnet address derivation.
- A recovery-phrase confirmation step and an address dashboard with clipboard copy.

There is no mainnet support, wallet persistence, balance lookup, signing,
broadcasting, history, or payment integration yet. The app does not retain the
private key after deriving the receiving address.

## Planned wallet protections

- Generate and restore wallets with BIP39 and the Rust Bitcoin ecosystem, not
  custom cryptography.
- Use the browser's cryptographically secure random source and fail closed if it
  is unavailable.
- Accept only 12- or 24-word English BIP39 imports and validate the checksum
  before deriving an address. Optional BIP39 passphrases are not supported in
  this milestone.
- Zeroize recovery phrases, entropy, and seeds where practical.
- Do not persist wallet secrets in local storage, session storage, cookies, or a
  browser database.
- Never send recovery phrases, seeds, or private keys to the API or third parties.
- Keep wallet derivation separate from UI code and cover it with tests.
- Proxy only public testnet chain data through the API in later milestones.
- Validate addresses and network before building or broadcasting a transaction.
- Show recovery-phrase warnings before wallet creation.

## Known browser-wallet limitations

Browser wallets can be harmed by XSS, malicious extensions, clipboard malware,
and compromised devices. Recovery phrases and derived keys can exist transiently
in WebAssembly and browser memory. Zeroization reduces exposure but cannot
guarantee removal from every memory, swap file, crash dump, or browser snapshot.
This milestone avoids retaining a signing key, but it does not make the app
secure for real funds. A browser wallet should not be described as secure merely
because tests pass.

Before real funds are supported, BitSabi needs:

- A documented threat model and recovery procedure.
- Tested backup, restore, signing, and transaction-monitoring flows.
- An independent security review.
- A decision on hardware-wallet or external-signer support.

## Reporting

Do not open a public issue for a suspected secret leak or wallet vulnerability.
Report it privately to the project maintainers.
