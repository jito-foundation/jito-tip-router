# Tip Router Operator CLI

## Commands

### Claim

```bash
RUST_LOG=info /tmp/tip-router-operator-cli \
  --keypair-path <KEYPAIR_PATH> \
  --operator-address "GmWQyzNGzMGQySvNCADu9pynAQfUjQm6tJL9cuN5Y3D6" \
  --rpc-url <MAINNET_RPC_URL> \
  --ledger-path /tmp/tip-router/ledger \
  --backup-snapshots-dir /tmp/tip-router/snapshots \
  --snapshot-output-dir /tmp/tip-router/snapshots \
  --save-path /tmp/tip-router/<EPOCH> \
  claim-tips \
  --tip-router-program-id RouterBmuRBkPUbgEDMtdvTZ75GBdSREZR5uGUxxxpb \
  --tip-distribution-program-id 4R3gSG8BpU4t19KYj8CfnbtRpnT8gtk4dvTHxVRwc2r7 \
  --priority-fee-distribution-program-id Priority6weCZ5HwDn29NxLFpb7TDp2iLZ6XKc5e8d3 \
  --ncn-address jtoF4epChkmd75V2kxXSmywatczAomDqKu6VfWUQocT \
  --epoch <EPOCH>
```
