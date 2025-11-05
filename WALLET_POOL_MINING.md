# Wallet Pool Mining Mode

This document describes the new Wallet Pool Mining feature for Shadow Harvester.

## Overview

Wallet Pool Mining allows you to mine with multiple wallets from a JSON file concurrently. When one wallet completes a challenge, it automatically rotates to the next wallet in the pool, maintaining a constant number of active miners to maximize earning potential.

## Usage

```bash
./shadow-harvester \
  --api-url <API_URL> \
  --wallets-file wallets.json \
  --concurrent-wallets 5 \
  --threads 24 \
  --data-dir . \
  --accept-tos
```

### Command-Line Options

- `--wallets-file <PATH>`: Path to the wallets.json file containing wallet configurations (required for this mode)
- `--concurrent-wallets <N>`: Number of wallets to mine with simultaneously (default: 1)
- `--threads <N>`: Number of CPU threads per wallet (default: 24)
- `--data-dir <PATH>`: Directory to store receipts and state (default: .)
- `--donate-to <ADDRESS>`: Optional Cardano address to donate all mining rewards to

## Wallets JSON Format

The `wallets.json` file should contain an array of wallet objects:

```json
[
  {
    "id": 1,
    "name": "Wallet 1",
    "mnemonic": "word1 word2 word3 ... word24",
    "password": "optional_password",
    "profile_dir": "optional_profile_directory",
    "created_at": "2025-11-01T15:43:35.513742",
    "status": "completed",
    "total_solved": 15,
    "total_unsolved": 3,
    "estimated_tokens": "121.4122 NIGHT",
    "last_updated": "2025-11-05T21:27:32.574731"
  },
  {
    "id": 2,
    "name": "Wallet 2",
    "mnemonic": "another set of 24 words here",
    "password": "optional_password",
    "profile_dir": "optional_profile_directory",
    "created_at": "2025-11-01T15:44:23.544354",
    "status": "completed",
    "total_solved": 13,
    "total_unsolved": 3,
    "estimated_tokens": "101.9341 NIGHT",
    "last_updated": "2025-11-05T20:56:17.834072"
  }
]
```

### Required Fields

- `id`: Unique identifier for the wallet
- `name`: Human-readable name for the wallet
- `mnemonic`: 24-word BIP39 mnemonic phrase

### Optional Fields

All other fields are optional and used for tracking/metadata purposes. The miner only uses the `id`, `name`, and `mnemonic` fields.

## How It Works

1. **Loading**: All wallets are loaded from the JSON file at startup
2. **Address Derivation**: Each wallet derives its address at index 0 using the path: `m/1852'/1815'/0'/0/0`
3. **Concurrent Mining**: N wallets (specified by `--concurrent-wallets`) mine simultaneously
4. **Rotation**: When a wallet completes a challenge (finds a solution), the next wallet from the pool is automatically started
5. **Completion**: Mining continues until all wallets have mined for the current challenge

## Features

- **Automatic Rotation**: Maintains constant mining capacity by replacing completed wallets
- **Progress Tracking**: Shows which wallet is mining and completion status
- **Receipt Management**: Automatically skips wallets that have already solved the current challenge
- **Recovery**: Checks for unsubmitted solutions from previous runs
- **Donation Support**: All wallets can donate to a specified address using `--donate-to`

## Example Output

```
⛏️  Shadow Harvester: WALLET POOL MINING Mode (DYNAMIC POLLING)
Wallets File: wallets.json
Concurrent Wallets: 5

✅ Loaded 50 wallets from file
Mining with 5 concurrent wallets

📋 Challenge Active: challenge_123
Difficulty: 0000000000000000ffffffffffffffffffffffffffffffffffffffffffffffff

[WALLET START] Mining with: Wallet 1 (ID: 1)
  Address: addr1v...
  Crypto Receipts: 15
  Night Allocation: 121

[WALLET START] Mining with: Wallet 2 (ID: 2)
  Address: addr1v...
  ...

✅ Wallet Wallet 1 (ID: 1) completed. (1/50)
[WALLET START] Mining with: Wallet 6 (ID: 6)
  ...

✅ All 50 wallets have completed mining for challenge challenge_123
```

## Notes

- Each wallet uses the same number of threads (specified by `--threads`)
- Total CPU usage = `concurrent_wallets × threads`
- Receipts are stored separately for each wallet in the mnemonic directory structure
- The miner automatically registers addresses with the API if needed
- Wallets with existing receipts or pending solutions are automatically skipped
