#!/usr/bin/env bash

SBF_PROGRAM_DIR=$PWD/integration_tests/tests/fixtures
FIXTURES_DIR=$PWD/tip-router-operator-cli/tests/fixtures
LEDGER_DIR=$FIXTURES_DIR/test-ledger
TDA_ACCOUNT_DIR=$FIXTURES_DIR/tda-accounts
DESIRED_SLOT=150
max_validators=10
validator_file=$FIXTURES_DIR/local_validators.txt
sol_amount=50
stake_per_validator=$((($sol_amount - ($max_validators * 2))/$max_validators))

keys_dir=$FIXTURES_DIR/keys
mkdir -p $keys_dir

spl_stake_pool=spl-stake-pool

create_keypair () {
  if test ! -f "$1"
  then
    solana-keygen new --no-passphrase -s -o "$1"
  fi
}

# Function to create keypairs and serialize accounts
prepare_keypairs_and_serialize() {
  max_validators=$1
  validator_file=$2

  for number in $(seq 1 "$max_validators"); do
    # Create keypairs for identity, vote, and withdrawer
    create_keypair "$keys_dir/identity_$number.json"
    create_keypair "$keys_dir/vote_$number.json"
    create_keypair "$keys_dir/withdrawer_$number.json"

    # Get the public key of the vote account
    vote_pubkey=$(solana-keygen pubkey "$keys_dir/vote_$number.json")

    # Extract the public key from the identity keypair
    merkle_root_upload_authority=$(solana-keygen pubkey "$keys_dir/identity_$number.json")

    # Append the vote public key to the validator file
    echo "$vote_pubkey" >> "$validator_file"

    # Dynamically run the Rust script with the vote_pubkey
    RUST_LOG=info cargo run --bin serialize-accounts -- \
      --validator-vote-account "$vote_pubkey" \
      --merkle-root-upload-authority "$merkle_root_upload_authority" \
      --epoch-created-at 4 \
      --validator-commission-bps 1 \
      --expires-at 1000 \
      --bump 1 \
      --tda-accounts-dir $TDA_ACCOUNT_DIR
  done
}

# Function to create vote accounts
create_vote_accounts() {
  max_validators=$1
  validator_file=$2

  for number in $(seq 1 "$max_validators"); do
    # Create the vote account
    solana create-vote-account \
      "$keys_dir/vote_$number.json" \
      "$keys_dir/identity_$number.json" \
      "$keys_dir/withdrawer_$number.json" \
      --commission 1
  done
}


add_validator_stakes () {
  stake_pool=$1
  validator_list=$2
  while read -r validator
  do
    $spl_stake_pool add-validator "$stake_pool" "$validator"
  done < "$validator_list"
}

increase_stakes () {
  stake_pool_pubkey=$1
  validator_list=$2
  sol_amount=$3
  while read -r validator
  do
    {
      $spl_stake_pool increase-validator-stake "$stake_pool_pubkey" "$validator" "$sol_amount"
     } || {
      $spl_stake_pool update "$stake_pool_pubkey" && \
      $spl_stake_pool increase-validator-stake "$stake_pool_pubkey" "$validator" "$sol_amount"
     }
  done < "$validator_list"
}

# Hoist the creation of keypairs and serialization
echo "Preparing keypairs and serializing accounts"
mkdir -p $FIXTURES_DIR/tda-accounts
prepare_keypairs_and_serialize "$max_validators" "$validator_file"

# Read the TDA account files and add them to args
tda_account_args=()
for f in "$TDA_ACCOUNT_DIR"/*; do
  filename=$(basename $f)
  account_address=${filename%.*}
  tda_account_args+=( --account $account_address $f )
done

echo "tda_account_args ${tda_account_args[@]}" 

VALIDATOR_PID=
setup_test_validator() {
  solana-test-validator \
   --bpf-program SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy $SBF_PROGRAM_DIR/spl_stake_pool.so \
   --bpf-program 4R3gSG8BpU4t19KYj8CfnbtRpnT8gtk4dvTHxVRwc2r7 $SBF_PROGRAM_DIR/jito_tip_distribution.so \
   --bpf-program T1pyyaTNZsKv2WcRAB8oVnk93mLJw2XzjtVYqCsaHqt $SBF_PROGRAM_DIR/jito_tip_payment.so \
   --account-dir $FIXTURES_DIR/accounts \
   "${tda_account_args[@]}" \
   --ledger $LEDGER_DIR \
   --slots-per-epoch 32 \
   --quiet --reset &
  VALIDATOR_PID=$!
  solana config set --url http://127.0.0.1:8899
  solana config set --commitment confirmed
  echo "waiting for solana-test-validator, pid: $VALIDATOR_PID"
  sleep 15
}

# SETUP LOCAL NET (https://spl.solana.com/stake-pool/quickstart#optional-step-0-setup-a-local-network-for-testing)

echo "Setting up local test validator"
set +ex
setup_test_validator
set -ex

echo "Creating vote accounts, these accounts be added to the stake pool"
create_vote_accounts "$max_validators" "$validator_file"

echo "Done adding $max_validators validator vote accounts, their pubkeys can be found in $validator_file"

# SETUP Stake Pool (https://spl.solana.com/stake-pool/quickstart#step-1-create-the-stake-pool)

# Script to setup a stake pool from scratch.  Please modify the parameters to
# create a stake pool to your liking!
command_args=()

###################################################
### MODIFY PARAMETERS BELOW THIS LINE FOR YOUR POOL
###################################################

# Epoch fee, assessed as a percentage of rewards earned by the pool every epoch,
# represented as `numerator / denominator`
command_args+=( --epoch-fee-numerator 1 )
command_args+=( --epoch-fee-denominator 100 )

# Withdrawal fee for SOL and stake accounts, represented as `numerator / denominator`
command_args+=( --withdrawal-fee-numerator 2 )
command_args+=( --withdrawal-fee-denominator 100 )

# Deposit fee for SOL and stake accounts, represented as `numerator / denominator`
command_args+=( --deposit-fee-numerator 3 )
command_args+=( --deposit-fee-denominator 100 )

command_args+=( --referral-fee 0 ) # Percentage of deposit fee that goes towards the referrer (a number between 0 and 100, inclusive)

command_args+=( --max-validators 2350 ) # Maximum number of validators in the stake pool, 2350 is the current maximum possible

# (Optional) Deposit authority, required to sign all deposits into the pool.
# Setting this variable makes the pool "private" or "restricted".
# Uncomment and set to a valid keypair if you want the pool to be restricted.
#command_args+=( --deposit-authority keys/authority.json )

###################################################
### MODIFY PARAMETERS ABOVE THIS LINE FOR YOUR POOL
###################################################


echo "Creating pool"
stake_pool_keyfile=$keys_dir/stake-pool.json
validator_list_keyfile=$keys_dir/validator-list.json
mint_keyfile=$keys_dir/mint.json
reserve_keyfile=$keys_dir/reserve.json
create_keypair $stake_pool_keyfile
create_keypair $validator_list_keyfile
create_keypair $mint_keyfile
create_keypair $reserve_keyfile

set -ex
$spl_stake_pool \
  create-pool \
  "${command_args[@]}" \
  --pool-keypair "$stake_pool_keyfile" \
  --validator-list-keypair "$validator_list_keyfile" \
  --mint-keypair "$mint_keyfile" \
  --reserve-keypair "$reserve_keyfile"

set +ex
stake_pool_pubkey=$(solana-keygen pubkey "$stake_pool_keyfile")
set -ex

set +ex
lst_mint_pubkey=$(solana-keygen pubkey "$mint_keyfile")
set -ex

echo "Depositing SOL into stake pool"
$spl_stake_pool deposit-sol "$stake_pool_pubkey" "$sol_amount"

echo "Adding validator stake accounts to the pool"
add_validator_stakes "$stake_pool_pubkey" "$validator_file"

echo "Increasing amount delegated to each validator in stake pool"
increase_stakes "$stake_pool_pubkey" "$validator_file" "$stake_per_validator"

# Clear the validator vote pubkey file so it doesn't expand and cause errors next run
rm $validator_file

# wait for certain epoch
echo "waiting for epoch X from validator $VALIDATOR_PID"
while true
do
current_slot=$(solana slot --url http://localhost:8899)
echo "current slot $current_slot"
[[ $current_slot -gt $DESIRED_SLOT ]] && kill $VALIDATOR_PID && exit 0
sleep 5
done