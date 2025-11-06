// src/main.rs - Final Minimal Version

use clap::Parser;
use std::thread; // ADDED
use chrono;

// Declare modules
mod api;
mod backoff;
mod cli;
mod constants;
mod cardano;
mod data_types;
mod utils; // The helpers module
mod mining;
mod submitter;

use mining::{run_persistent_key_mining, run_mnemonic_sequential_mining, run_ephemeral_key_mining, run_wallet_pool_mining};
use utils::{setup_app, print_mining_setup}; // Importing refactored helpers
use cli::Cli;
use api::get_active_challenge_data;
use data_types::WalletConfig;
use std::fs;

/// Generate N wallets with random mnemonics and save to JSON file
fn generate_wallets_file(count: usize, output_file: &str) -> Result<(), String> {
    if count == 0 {
        return Err("Cannot generate 0 wallets".to_string());
    }

    if count > 1000 {
        return Err("Maximum 1000 wallets allowed per file".to_string());
    }

    println!("🔑 Generating {} new wallet(s)...", count);

    let mut wallets = Vec::new();
    for i in 1..=count {
        let mnemonic = cardano::generate_mnemonic();

        // Derive address for display purposes
        let key_pair = cardano::derive_key_pair_from_mnemonic(&mnemonic, 0, 0);
        let address = key_pair.2.to_bech32().unwrap();

        println!("   Wallet {} - {}", i, address);

        let wallet = WalletConfig {
            id: i as u32,
            name: format!("Wallet {}", i),
            mnemonic,
            password: None,
            profile_dir: None,
            created_at: Some(chrono::Utc::now().to_rfc3339()),
            status: Some("active".to_string()),
            total_solved: Some(0),
            total_unsolved: Some(0),
            estimated_tokens: Some("0.0".to_string()),
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
        };
        wallets.push(wallet);
    }

    let json = serde_json::to_string_pretty(&wallets)
        .map_err(|e| format!("Failed to serialize wallets: {}", e))?;

    fs::write(output_file, json)
        .map_err(|e| format!("Failed to write wallets file '{}': {}", output_file, e))?;

    println!("\n✅ Successfully generated {} wallet(s) and saved to '{}'", count, output_file);
    println!("\n⚠️  IMPORTANT: Back up this file securely! It contains your wallet mnemonics.");
    println!("   You can now start mining with: --wallets-file {}", output_file);

    Ok(())
}

/// Runs the main application logic based on CLI flags.
fn run_app(cli: Cli) -> Result<(), String> {
    // Handle wallet generation mode (no API needed)
    if let Some(count) = cli.generate_wallets {
        return generate_wallets_file(count, cli.wallets_file.as_deref().unwrap_or("wallets.json"));
    }

    let context = match setup_app(&cli) {
        Ok(c) => c,
        // Exit the app if a command like 'Challenges' was run successfully
        Err(e) if e == "COMMAND EXECUTED" => return Ok(()),
        Err(e) => return Err(e),
    };

    // --- Start Background Submitter Thread ---
    // Clone client, API URL, and data_dir for the background thread
    let _submitter_handle = if let Some(base_dir) = context.data_dir {
        let client_clone = context.client.clone();
        let api_url_clone = context.api_url.clone();
        let data_dir_clone = base_dir.to_string();

        println!("📦 Starting background submitter thread...");
        let handle = thread::spawn(move || {
            match submitter::run_submitter_thread(client_clone, api_url_clone, data_dir_clone) {
                Ok(_) => {},
                Err(e) => eprintln!("FATAL SUBMITTER ERROR: {}", e),
            }
        });
        Some(handle)
    } else {
        println!("⚠️ No --data-dir specified. Submissions will be synchronous (blocking) and lost on API error.");
        None
    };
    // ---------------------------------------------

    // --- Pre-extract mnemonic logic ---
    let mnemonic: Option<String> = if let Some(mnemonic) = cli.mnemonic.clone() {
        Some(mnemonic)
    } else if let Some(mnemonic_file) = cli.mnemonic_file.clone() {
        Some(std::fs::read_to_string(mnemonic_file)
            .map_err(|e| format!("Could not read mnemonic from file: {}", e))?)
    } else {
        None
    };

    // 1. Default mode: display info and exit
    if cli.payment_key.is_none() && !cli.ephemeral_key && mnemonic.is_none() && cli.challenge.is_none() && cli.wallets_file.is_none() {
        // Fetch challenge for info display
        match get_active_challenge_data(&context.client, &context.api_url) {
            Ok(challenge_params) => {
                 print_mining_setup(
                    &context.api_url,
                    cli.address.as_deref(),
                    context.threads,
                    &challenge_params
                );
            },
            Err(e) => eprintln!("Could not fetch active challenge for info display: {}", e),
        };
        println!("MODE: INFO ONLY. Provide '--payment-key', '--mnemonic', '--mnemonic-file', '--wallets-file', or '--ephemeral-key' to begin mining.");
        return Ok(())
    }

    // 2. Determine Operation Mode and Start Mining
    let result = if let Some(wallets_file) = cli.wallets_file.as_ref() {
        // Mode D: Wallet Pool Mining (Priority mode)
        run_wallet_pool_mining(context, wallets_file, cli.concurrent_wallets)
    }
    else if let Some(skey_hex) = cli.payment_key.as_ref() {
        // Mode A: Persistent Key Mining
        run_persistent_key_mining(context, skey_hex)
    }
    else if let Some(mnemonic_phrase) = mnemonic {
        // Mode B: Mnemonic Sequential Mining
        run_mnemonic_sequential_mining(&cli, context, mnemonic_phrase)
    }
    else if cli.ephemeral_key {
        // Mode C: Ephemeral Key Mining (New key per cycle)
        run_ephemeral_key_mining(context)
    } else {
        // This should be unreachable due to the validation in utils::setup_app
        Ok(())
    };

    // NOTE: In a production app, you would join the submitter thread here.
    // if let Some(handle) = submitter_handle { handle.join().unwrap(); }

    result
}

fn main() {
    let cli = Cli::parse();

    match run_app(cli) {
        Ok(_) => {},
        Err(e) => {
            if e != "COMMAND EXECUTED" { // Don't print fatal error if a command ran successfully
                eprintln!("FATAL ERROR: {}", e);
                std::process::exit(1);
            }
        }
    }
}
