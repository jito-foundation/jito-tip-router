# Tip Router Operator CLI

## Commands

## ReclaimExpiredAccounts

```bash
RUST_LOG=info cargo r --bin tip-router-operator-cli -- \
  --keypair-path ~/.config/solana/id.json \
  --operator-address "GmWQyzNGzMGQySvNCADu9pynAQfUjQm6tJL9cuN5Y3D6" \
  --rpc-url <MAINNET_RPC_URL> \
  --ledger-path /tmp/ledger \
  --backup-snapshots-dir /tmp/backup-snapshots \
  --snapshot-output-dir /tmp/snapshot-output \
  --save-path /tmp/save \
  reclaim-expired-accounts \
  --tip-distribution-program-id <TIP_DIST_PROGRAM_ID> \
  --priority-fee-distribution-program-id <PF_DIST_PROGRAM_ID> \
  --num-monitored-epochs 3
```

### Claim

```bash
RUST_LOG=info cargo r --bin tip-router-operator-cli -- \
  --keypair-path ~/.config/solana/id.json \
  --operator-address "GmWQyzNGzMGQySvNCADu9pynAQfUjQm6tJL9cuN5Y3D6" \
  --rpc-url <MAINNET_RPC_URL> \
  --ledger-path /tmp/tip-router/ledger \
  --backup-snapshots-dir /tmp/tip-router/snapshots \
  --snapshot-output-dir /tmp/tip-router/snapshots \
  --save-path /tmp/tip-router/EPOCH \
  claim-tips \
  --tip-router-program-id RouterBmuRBkPUbgEDMtdvTZ75GBdSREZR5uGUxxxpb \
  --tip-distribution-program-id 4R3gSG8BpU4t19KYj8CfnbtRpnT8gtk4dvTHxVRwc2r7 \
  --priority-fee-distribution-program-id Priority6weCZ5HwDn29NxLFpb7TDp2iLZ6XKc5e8d3 \
  --ncn-address jtoF4epChkmd75V2kxXSmywatczAomDqKu6VfWUQocT \
  --epoch EPOCH
```
