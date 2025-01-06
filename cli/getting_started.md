## Setup

Build and install the CLI

In the root of the repo:

```bash
cargo build --release
cargo install --path ./cli --bin jito-tip-router-cli --locked
```

Ensure it has been installed

```bash
jito-tip-router-cli --help
```

## Create an NCN

<https://jito-foundation.gitbook.io/mev/mev-payment-and-distribution/on-chain-addresses>

```rust
Restaking: RestkWeAVL8fRGgzhfeoqFhsqKRchg6aa1XrcH96z4Q
Vault:     Vau1t6sLNxnzB7ZDsef8TLbPLfyZMYXH8WTNqUdm9g8
JitoSOL:   Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb
```
