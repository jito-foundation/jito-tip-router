#!/bin/bash

#------------------------------------------------------------------------------
# Configuration
#------------------------------------------------------------------------------

# Add your pubkeys here, one per line for readability
PUBKEYS=(
    "5S7ErPSkFmyXuq2aE3rZ6ofwVyZpwzUt6w7m6kqekvMe"  # JTO/SOL Binance Price
    "4Z1SLH9g4ikNBV8uP2ZctEouqjYmVqB2Tz5SZxKYBN7z"  # JITOSOL/SOL Redemption Price
)

# Output file path
OUTPUT_FILE="./integration_tests/tests/fixtures/generated_switchboard_accounts.rs"

#------------------------------------------------------------------------------
# Function Definitions
#------------------------------------------------------------------------------

write_file_header() {
    cat << EOF > "$OUTPUT_FILE"
use std::str::FromStr;

use solana_sdk::{account::Account, pubkey::Pubkey};

// Auto-generated by format_switchboard_accounts.sh
pub fn get_switchboard_accounts() -> Vec<(Pubkey, Account)> {
    let mut accounts = Vec::new();

EOF
}

write_account_header() {
    local ACCOUNT="$1"
    local OWNER="$2"
    local LAMPORTS="$3"
    local SPACE="$4"

    cat << EOF >> "$OUTPUT_FILE"
    {
        let address = Pubkey::from_str("$ACCOUNT").unwrap();
        let owner = Pubkey::from_str("$OWNER").unwrap();
        let lamports = $LAMPORTS;
        let space = $SPACE;
        let mut account = Account::new(lamports, space, &owner);

        let bytes = vec![
EOF
}

write_account_data() {
    local ACCOUNT_DATA="$1"
    
    echo "$ACCOUNT_DATA" | awk '
        BEGIN { count = 0 }
        /^[0-9a-fA-F]{4}:/ {
            for(i=2; i<=9; i++) {
                if($i ~ /^[0-9a-fA-F]{2}$/) {
                    if (count > 0) printf ", "
                    if (count % 15 == 0 && count > 0) printf "\n        "
                    printf "0x%s", $i
                    count++
                }
            }
        }
    ' >> "$OUTPUT_FILE"
}

write_account_footer() {
    cat << EOF >> "$OUTPUT_FILE"

        ];

        account.data = bytes;
        accounts.push((address, account));
    }

EOF
}

write_file_footer() {
    cat << EOF >> "$OUTPUT_FILE"
    accounts
}
EOF
}

process_account() {
    local ACCOUNT="$1"
    echo "Processing account: $ACCOUNT"
    
    # Run solana account command and capture output
    local ACCOUNT_DATA=$(solana account "$ACCOUNT" --lamports)
    
    # Check if the command failed
    if [ $? -ne 0 ]; then
        echo "Error: Failed to fetch data for account $ACCOUNT"
        return 1
    fi

    # Extract key information using grep and awk
    local OWNER=$(echo "$ACCOUNT_DATA" | grep "Owner:" | awk '{print $2}')
    local LAMPORTS=$(echo "$ACCOUNT_DATA" | grep "Balance:" | awk '{print $2}')
    local SPACE=$(echo "$ACCOUNT_DATA" | grep "Length:" | awk '{print $2}')

    write_account_header "$ACCOUNT" "$OWNER" "$LAMPORTS" "$SPACE"
    write_account_data "$ACCOUNT_DATA"
    write_account_footer
}

#------------------------------------------------------------------------------
# Main Script
#------------------------------------------------------------------------------

main() {
    # Create the output directory if it doesn't exist
    mkdir -p "$(dirname "$OUTPUT_FILE")"

    # Write the file header
    write_file_header

    # Process each account
    for ACCOUNT in "${PUBKEYS[@]}"; do
        process_account "$ACCOUNT"
    done

    # Write the file footer
    write_file_footer

    echo "Generated Rust code has been written to $OUTPUT_FILE"
}

# Execute main function
main