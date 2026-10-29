# Security notes

This document records the current security position. It must be updated before
any wallet or payment feature is enabled.

## Non-negotiable rules

- Recovery phrases and private keys never go to the API, database, logs, error
  messages, analytics, or third-party services.
- Wallet secrets are not used during development or testing. Only testnet
  wallets and test coins are allowed.
- The server never signs Bitcoin transactions on behalf of a user.
- Mainnet is disabled in the MVP.
- Mock NGN payments are clearly labelled and never presented as real money.
- A frontend success message is not proof that a payment happened.

## Current state

Only the following exist today:

- A health endpoint.
- Static UI pages and navigation.
- Security response headers (`X-Content-Type-Options`, `X-Frame-Options`,
  `Referrer-Policy`).
- A CORS policy restricted to the configured frontend origin.

There is no wallet, no key storage, and no payment integration yet.

## Planned wallet protections

- Generate and restore keys with BDK and BIP39, not custom cryptography.
- Keep keys in the browser and encrypt local storage with a key derived using
  Argon2id; encrypt wallet data with AES-256-GCM.
- Zeroize secrets where practical.
- Proxy only public testnet chain data through the API.
- Validate addresses and network before building or broadcasting a transaction.
- Show recovery-phrase warnings before wallet creation.

## Known browser-wallet limitations

Browser wallets can be harmed by XSS, malicious extensions, clipboard malware,
and compromised devices. Encryption at rest does not remove these risks. A
browser wallet should not be described as secure merely because tests pass.

Before real funds are supported, BitSabi needs:

- A documented threat model and recovery procedure.
- Tested backup, restore, signing, and transaction-monitoring flows.
- An independent security review.
- A decision on hardware-wallet or external-signer support.

## Reporting

Do not open a public issue for a suspected secret leak or wallet vulnerability.
Report it privately to the project maintainers.
