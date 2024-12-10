#!/usr/bin/env bash

# Default values if not provided
MAX_VALIDATORS=${1:-2}
VALIDATOR_FILE=${2:-"validators.txt"}
SOL_AMOUNT=${3:-100}
STAKE_PER_VALIDATOR=$((($SOL_AMOUNT - ($MAX_VALIDATORS * 2))/$MAX_VALIDATORS))

# Directory setup
KEYS_DIR=test-validator-keys
mkdir -p $KEYS_DIR

# Clear previous validator file and test ledger
rm -f $VALIDATOR_FILE
rm -rf test-ledger

# Function to wait for validator to be ready
wait_for_validator() {
    echo "Waiting for validator to be ready..."
    while ! solana cluster-version &>/dev/null; do
        sleep 1
    done
    echo "Validator is ready!"
}

setup_test_validator() {
    # Get program IDs from the programs
    TIP_PAYMENT_ID="4R3gSG8BpU4t19KYj8CfnbtRpnT8gtk4dvTHxVRwc2r7"  # From tip-payment/src/lib.rs
    TIP_DISTRIBUTION_ID="T1pyyaTNZsKv2WcRAB8oVnk93mLJw2XzjtVYqCsaHqt"  # From tip-distribution/src/lib.rs

    # Use programs from local programs directory
    PROGRAMS_DIR="../programs"
    TIP_PAYMENT_PROGRAM="$PROGRAMS_DIR/tip-payment/src/lib.rs"
    TIP_DISTRIBUTION_PROGRAM="$PROGRAMS_DIR/tip-distribution/src/lib.rs"

    solana-test-validator \
        --slots-per-epoch 32 \
        --ticks-per-slot 8 \
        --quiet \
        --reset \
        --bpf-program "$TIP_PAYMENT_ID" "$TIP_PAYMENT_PROGRAM" \
        --bpf-program "$TIP_DISTRIBUTION_ID" "$TIP_DISTRIBUTION_PROGRAM" \
        &
    
    VALIDATOR_PID=$!
    solana config set --url http://127.0.0.1:8899
    solana config set --commitment confirmed
    echo "waiting for solana-test-validator, pid: $VALIDATOR_PID"
    wait_for_validator
    
    # Request airdrop for stake account creation
    solana airdrop $SOL_AMOUNT || {
        echo "Failed to airdrop SOL"
        exit 1
    }
    
    # Wait for airdrop to confirm
    sleep 5
}

create_keypair() {
    if test ! -f "$1"; then
        solana-keygen new --no-passphrase -s -o "$1"
    fi
}

create_vote_accounts() {
    for i in $(seq 1 $MAX_VALIDATORS); do
        echo "Creating validator $i..."
        # Create keypairs
        create_keypair "$KEYS_DIR/identity_$i.json"
        create_keypair "$KEYS_DIR/vote_$i.json"
        create_keypair "$KEYS_DIR/withdrawer_$i.json"
        create_keypair "$KEYS_DIR/stake_$i.json"

        # Create vote account
        solana create-vote-account \
            "$KEYS_DIR/vote_$i.json" \
            "$KEYS_DIR/identity_$i.json" \
            "$KEYS_DIR/withdrawer_$i.json" \
            --commission 10 || {
            echo "Failed to create vote account for validator $i"
            continue
        }

        # Create and delegate stake account
        solana create-stake-account \
            "$KEYS_DIR/stake_$i.json" \
            $STAKE_PER_VALIDATOR || {
            echo "Failed to create stake account for validator $i"
            continue
        }

        # Delegate stake
        VOTE_PUBKEY=$(solana-keygen pubkey "$KEYS_DIR/vote_$i.json")
        STAKE_PUBKEY=$(solana-keygen pubkey "$KEYS_DIR/stake_$i.json")
        
        solana delegate-stake \
            "$KEYS_DIR/stake_$i.json" \
            "$VOTE_PUBKEY" || {
            echo "Failed to delegate stake for validator $i"
            continue
        }

        # Wait for stake activation
        echo "Waiting for stake activation..."
        sleep 10  # Give some time for stake to activate
        
        # Verify stake activation
        solana stake-account "$KEYS_DIR/stake_$i.json" || {
            echo "Failed to verify stake activation for validator $i"
            continue
        }
        # Save vote pubkey to validator file
        echo "$VOTE_PUBKEY" >> "$VALIDATOR_FILE"
        echo "Created and delegated validator $i: Vote account $VOTE_PUBKEY, Stake account $STAKE_PUBKEY"
    done
}

wait_for_stake_activation() {
    echo "Waiting for all stakes to activate..."
    local max_attempts=30  # 5 minutes maximum wait time
    local attempt=0
    
    while [ $attempt -lt $max_attempts ]; do
        all_active=true
        for i in $(seq 1 $MAX_VALIDATORS); do
            stake_status=$(solana stake-account "$KEYS_DIR/stake_$i.json" | grep "Active Stake:" || echo "")
            if [[ ! $stake_status =~ "Active Stake: ".*"SOL" ]]; then
                all_active=false
                break
            fi
        done
        
        if $all_active; then
            echo "All stakes are now active!"
            # Wait a bit more to ensure everything is processed
            sleep 10
            return 0
        fi
        
        echo "Waiting for stakes to activate... (attempt $((attempt + 1))/$max_attempts)"
        sleep 10
        attempt=$((attempt + 1))
    done
    
    echo "Error: Stakes did not activate within the timeout period"
    exit 1
}

initialize_tip_payment_config() {
    echo "Initializing tip payment config..."
    
    # Create a funding keypair if it doesn't exist
    FUNDER_KEYPAIR="$KEYS_DIR/funder.json"
    if [ ! -f "$FUNDER_KEYPAIR" ]; then
        solana-keygen new --no-passphrase --force -o "$FUNDER_KEYPAIR"
        
        # Request airdrop for the funder
        solana airdrop 100 "$(solana-keygen pubkey "$FUNDER_KEYPAIR")" || {
            echo "Failed to airdrop SOL to funder"
            exit 1
        }
        sleep 5  # Wait for airdrop to confirm
    fi
    
    # Create and fund config account
    CONFIG_KEYPAIR="$KEYS_DIR/config.json"
    solana-keygen new --no-passphrase --force -o "$CONFIG_KEYPAIR"
    
    solana transfer \
        --from "$FUNDER_KEYPAIR" \
        --allow-unfunded-recipient \
        "$(solana-keygen pubkey "$CONFIG_KEYPAIR")" \
        2 \
        || {
        echo "Failed to fund config account"
        exit 1
    }
    
    # Create and fund tip payment accounts
    for SEED in {0..7}; do
        TIP_KEYPAIR="$KEYS_DIR/tip_$SEED.json"
        solana-keygen new --no-passphrase --force -o "$TIP_KEYPAIR"
        
        solana transfer \
            --from "$FUNDER_KEYPAIR" \
            --allow-unfunded-recipient \
            "$(solana-keygen pubkey "$TIP_KEYPAIR")" \
            2 \
            || {
            echo "Failed to fund tip payment account $SEED"
            continue
        }
    done
    
    sleep 5  # Wait for transactions to confirm
}
# cleanup() {
#     # Kill the validator if it's still running
#     if [ ! -z "$VALIDATOR_PID" ]; then
#         echo "Cleaning up validator process..."
#         kill $VALIDATOR_PID 2>/dev/null
#     fi
# }


# # Set up cleanup trap
# trap cleanup EXIT

main() {
    echo "Setting up test validator..."
    setup_test_validator
    
    echo "Creating $MAX_VALIDATORS validator vote accounts..."
    create_vote_accounts
    
    echo "Waiting for all stakes to activate..."
    wait_for_stake_activation

    echo "Initializing tip payment config..."
    initialize_tip_payment_config

    echo "Setup complete! Validator vote accounts are listed in $VALIDATOR_FILE"
}

main