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

/// Generate N wallets with random mnemonics and append to JSON file
fn generate_wallets_file(count: usize, output_file: &str) -> Result<(), String> {
    if count == 0 {
        return Err("Cannot generate 0 wallets".to_string());
    }

    if count > 1000 {
        return Err("Maximum 1000 wallets allowed per generation".to_string());
    }

    // Try to load existing wallets
    let mut existing_wallets: Vec<WalletConfig> = if std::path::Path::new(output_file).exists() {
        println!("📂 Loading existing wallets from '{}'...", output_file);
        let existing_json = fs::read_to_string(output_file)
            .map_err(|e| format!("Failed to read existing wallets file '{}': {}", output_file, e))?;

        match serde_json::from_str(&existing_json) {
            Ok(wallets) => {
                let wallet_vec: Vec<WalletConfig> = wallets;
                println!("   Found {} existing wallet(s)", wallet_vec.len());
                wallet_vec
            },
            Err(e) => {
                println!("   ⚠️  Could not parse existing file ({}). Creating backup and starting fresh.", e);
                // Backup the corrupted file
                let backup_file = format!("{}.backup.{}", output_file, chrono::Utc::now().timestamp());
                let _ = fs::copy(output_file, &backup_file);
                println!("   Backed up to: {}", backup_file);
                Vec::new()
            }
        }
    } else {
        println!("📂 No existing wallets file found. Creating new '{}'...", output_file);
        Vec::new()
    };

    // Find the highest ID to continue numbering
    let start_id = existing_wallets.iter().map(|w| w.id).max().unwrap_or(0) + 1;
    let existing_count = existing_wallets.len();

    println!("🔑 Generating {} new wallet(s) (starting from ID {})...", count, start_id);

    for i in 0..count {
        let wallet_id = start_id + i as u32;
        let mnemonic = cardano::generate_mnemonic();

        // Derive address for display purposes
        let key_pair = cardano::derive_key_pair_from_mnemonic(&mnemonic, 0, 0);
        let address = key_pair.2.to_bech32().unwrap();

        println!("   Wallet {} - {}", wallet_id, address);

        let wallet = WalletConfig {
            id: wallet_id,
            name: format!("Wallet {}", wallet_id),
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
        existing_wallets.push(wallet);
    }

    let json = serde_json::to_string_pretty(&existing_wallets)
        .map_err(|e| format!("Failed to serialize wallets: {}", e))?;

    fs::write(output_file, json)
        .map_err(|e| format!("Failed to write wallets file '{}': {}", output_file, e))?;

    println!("\n✅ Successfully generated {} new wallet(s) and appended to '{}'", count, output_file);
    println!("   Total wallets in file: {} (was: {}, added: {})", existing_wallets.len(), existing_count, count);
    println!("\n⚠️  IMPORTANT: Back up this file securely! It contains your wallet mnemonics.");
    println!("   You can now start mining with: --wallets-file {}", output_file);

    Ok(())
}

/// Setup donations for all wallets in wallets.json to a destination address (one-time operation)
fn setup_donate_all_wallets(wallets_file: &str, destination_address: &str, api_url: &str) -> Result<(), String> {
    println!("💸 Setting up donation consolidation for all wallets...");
    println!("   Source: {}", wallets_file);
    println!("   Destination: {}", destination_address);
    println!();

    // Load wallets from file
    if !std::path::Path::new(wallets_file).exists() {
        return Err(format!("Wallets file '{}' not found", wallets_file));
    }

    let wallets_json = fs::read_to_string(wallets_file)
        .map_err(|e| format!("Failed to read wallets file '{}': {}", wallets_file, e))?;

    let wallets: Vec<WalletConfig> = serde_json::from_str(&wallets_json)
        .map_err(|e| format!("Failed to parse wallets JSON: {}", e))?;

    if wallets.is_empty() {
        return Err("No wallets found in file".to_string());
    }

    println!("📂 Loaded {} wallet(s) from file", wallets.len());
    println!();

    // Create HTTP client with User-Agent (required to avoid 403 errors from WAF)
    let client = reqwest::blocking::Client::builder()
        .user_agent(constants::USER_AGENT)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    // Track results
    let mut success_count = 0;
    let mut already_donated_count = 0;
    let mut failed_count = 0;
    let mut failed_wallets: Vec<(String, String)> = Vec::new();

    // Process each wallet
    for (idx, wallet) in wallets.iter().enumerate() {
        let wallet_num = idx + 1;

        // Derive keypair from mnemonic (using account 0, index 0)
        let key_pair = cardano::derive_key_pair_from_mnemonic(&wallet.mnemonic, 0, 0);
        let source_address = key_pair.2.to_bech32().unwrap();

        // Truncate addresses for cleaner display
        let source_short = if source_address.len() > 20 {
            format!("{}...{}", &source_address[..10], &source_address[source_address.len()-8..])
        } else {
            source_address.clone()
        };

        let dest_short = if destination_address.len() > 20 {
            format!("{}...{}", &destination_address[..10], &destination_address[destination_address.len()-8..])
        } else {
            destination_address.to_string()
        };

        // Create donation message
        let donation_message = format!("Assign accumulated Scavenger rights to: {}", destination_address);

        // Sign the message using CIP-8
        let (donation_signature, _pubkey) = cardano::cip8_sign(&key_pair, &donation_message);

        // Call the donation API (silent version for batch operations)
        print!("[{:>3}/{:<3}] {} -> {} ", wallet_num, wallets.len(), source_short, dest_short);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        match api::donate_to_silent(&client, api_url, &source_address, destination_address, &donation_signature) {
            Ok(donation_id) => {
                println!("✅ (ID: {})", donation_id);
                success_count += 1;
            }
            Err(e) => {
                // Check if it's a 409 conflict (already donated)
                if e.contains("409") || e.contains("Conflict") || e.contains("already") {
                    println!("⏭️  Already donated");
                    already_donated_count += 1;
                } else {
                    println!("❌ Failed: {}", e);
                    failed_count += 1;
                    failed_wallets.push((source_address.clone(), e));
                }
            }
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 DONATION SETUP SUMMARY");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("   Total wallets:       {}", wallets.len());
    println!("   ✅ Newly donated:     {}", success_count);
    println!("   ⏭️  Already donated:   {}", already_donated_count);
    println!("   ❌ Failed:            {}", failed_count);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if !failed_wallets.is_empty() {
        println!();
        println!("Failed wallets:");
        for (addr, err) in failed_wallets.iter() {
            println!("   {} - {}", addr, err);
        }
    }

    println!();
    if failed_count == 0 {
        println!("🎉 All wallets successfully configured to donate to: {}", destination_address);
        println!("   All future mining rewards from these wallets will be consolidated to the destination address.");
    } else {
        println!("⚠️  Some wallets failed to set up donations. Please review the errors above.");
    }

    Ok(())
}

/// Runs the main application logic based on CLI flags.
fn run_app(cli: Cli) -> Result<(), String> {
    // Handle wallet generation mode (no API needed)
    if let Some(count) = cli.generate_wallets {
        return generate_wallets_file(count, cli.wallets_file.as_deref().unwrap_or("wallets.json"));
    }

    // Handle one-time donation setup (requires API URL)
    if let Some(destination_address) = &cli.donate_all_to {
        // Get API URL
        let api_url = cli.api_url.as_ref()
            .ok_or_else(|| "Error: --api-url is required when using --donate-all-to".to_string())?;

        // Get wallets file path
        let wallets_file = cli.wallets_file.as_deref().unwrap_or("wallets.json");

        return setup_donate_all_wallets(wallets_file, destination_address, api_url);
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
