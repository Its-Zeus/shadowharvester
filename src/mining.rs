// src/mining.rs

use crate::api;
use crate::data_types::{DataDir, DataDirMnemonic, MiningContext, OwnedMiningContext, MiningResult, ChallengeData, PendingSolution, FILE_NAME_FOUND_SOLUTION, is_solution_pending_in_queue, FILE_NAME_RECEIPT, WalletConfig};
use crate::cli::Cli;
use crate::cardano;
use crate::utils::{self, next_wallet_deriv_index_for_challenge, print_mining_setup, print_statistics, receipt_exists_for_index, run_single_mining_cycle};
use std::{fs, path::PathBuf, sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}}, thread, time::{Duration, Instant}}; // Added fs, path::PathBuf, sync, thread

// Live statistics tracking
#[derive(Debug, Clone, PartialEq)]
enum WalletStatus {
    Waiting,
    Mining,
    Solved,
    Failed,
    Skipped,
}

#[derive(Debug, Clone)]
struct WalletStats {
    name: String,
    address: String,
    status: WalletStatus,
    solved_count: u32,
    estimated_night: f64,
}

#[derive(Debug, Clone)]
struct LiveStats {
    wallets: Vec<WalletStats>,
    challenge_id: String,
    challenge_deadline: String,
    challenge_day: u8,
    next_challenge_time: Option<String>,
    start_time: Instant,
    total_network_solutions: u32,
    night_per_solution: f64,
}

impl LiveStats {
    fn display(&self) {
        // Build entire display string first for smooth rendering
        let mut output = String::with_capacity(10000);

        let elapsed = self.start_time.elapsed().as_secs();
        let solved = self.wallets.iter().filter(|w| w.status == WalletStatus::Solved).count();
        let failed = self.wallets.iter().filter(|w| w.status == WalletStatus::Failed).count();
        let skipped = self.wallets.iter().filter(|w| w.status == WalletStatus::Skipped).count();
        let mining = self.wallets.iter().filter(|w| w.status == WalletStatus::Mining).count();
        let waiting = self.wallets.iter().filter(|w| w.status == WalletStatus::Waiting).count();
        let total_solved_count: u32 = self.wallets.iter().map(|w| w.solved_count).sum();
        let total_estimated_night: f64 = self.wallets.iter().map(|w| w.estimated_night).sum();

        // Calculate next challenge countdown
        let next_challenge_str = if let Some(ref next_time) = self.next_challenge_time {
            if let Ok(next_dt) = chrono::DateTime::parse_from_rfc3339(next_time) {
                let now = chrono::Utc::now();
                let diff = next_dt.signed_duration_since(now);
                if diff.num_seconds() > 0 {
                    format!("{}m {}s", diff.num_minutes(), diff.num_seconds() % 60)
                } else {
                    "Now!".to_string()
                }
            } else {
                "Unknown".to_string()
            }
        } else {
            "N/A".to_string()
        };

        // Header
        output.push_str("╔═══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗\n");
        output.push_str("║                                        🚀 Shadow Harvester - Live Mining Dashboard                                                   ║\n");
        output.push_str("╠═══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣\n");
        output.push_str(&format!("║ 📋 Challenge: {:<15} │ Day: {:<3} │ ⏰ Deadline: {:<23} │ ⏭️  Next: {:<15} │ ⏱️  Elapsed: {}m {}s {:>15}║\n",
            self.challenge_id.chars().take(15).collect::<String>(),
            self.challenge_day,
            self.challenge_deadline.chars().take(23).collect::<String>(),
            next_challenge_str,
            elapsed / 60, elapsed % 60, ""));
        output.push_str("╠═══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣\n");
        output.push_str(&format!("║ 📊 Summary: Total: {:<3} │ ✅ Solved: {:<3} │ ⛏️  Mining: {:<3} │ ⏳ Waiting: {:<3} │ ✗ Failed: {:<3} │ 💰 Total NIGHT: {:.6} │ 🌐 Network: {:<6}║\n",
            self.wallets.len(), solved, mining, waiting, failed, total_estimated_night, self.total_network_solutions));
        output.push_str("╠════╤══════════════╤════════════════════════════════════════════════════════════╤══════════╤═══════════╤════════════════════════════════════╣\n");
        output.push_str("║ #  │ Wallet       │ Address                                                    │ Status   │ Solved    │ Est. NIGHT                         ║\n");
        output.push_str("╠════╪══════════════╪════════════════════════════════════════════════════════════╪══════════╪═══════════╪════════════════════════════════════╣\n");

        // Wallet rows
        for (i, wallet) in self.wallets.iter().enumerate() {
            let status_icon = match wallet.status {
                WalletStatus::Mining => "⛏️  Mining",
                WalletStatus::Solved => "✅ Solved",
                WalletStatus::Failed => "✗  Failed",
                WalletStatus::Skipped => "✓  Skipped",
                WalletStatus::Waiting => "⏳ Waiting",
            };

            let addr_display = if wallet.address.len() > 58 {
                format!("{}...", &wallet.address[..55])
            } else {
                format!("{:<58}", wallet.address)
            };

            output.push_str(&format!("║ {:<2} │ {:<12} │ {} │ {:<8} │ {:<9} │ {:.6} {:>25}║\n",
                i + 1,
                wallet.name.chars().take(12).collect::<String>(),
                addr_display,
                status_icon,
                wallet.solved_count,
                wallet.estimated_night, ""));
        }

        output.push_str("╚════╧══════════════╧════════════════════════════════════════════════════════════╧══════════╧═══════════╧════════════════════════════════════╝\n");

        // Print everything at once for smooth rendering
        print!("\x1B[2J\x1B[H{}", output);
    }
}

// ===============================================
// SOLUTION RECOVERY FUNCTION
// ===============================================

/// Checks the local storage for any solution that was found but not yet queued
/// and queues it if found.
fn check_for_unsubmitted_solutions(base_dir: &str, challenge_id: &str, mining_address: &str, data_dir_variant: &DataDir) -> Result<(), String> {
    // Determine the base path for the specific wallet/challenge
    let mut path = data_dir_variant.receipt_dir(base_dir, challenge_id)?;
    path.push(FILE_NAME_FOUND_SOLUTION);

    if path.exists() {
        println!("\n⚠️ Recovery file detected at {:?}. Recovering solution...", path);

        let solution_json = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read recovery file {:?}: {}", path, e))?;

        let pending_solution: PendingSolution = serde_json::from_str(&solution_json)
            .map_err(|e| format!("Failed to parse recovery solution JSON {:?}: {}", path, e))?;

        // 1. Save to the main submission queue
        if let Err(e) = data_dir_variant.save_pending_solution(base_dir, &pending_solution) {
            return Err(format!("FATAL RECOVERY ERROR: Could not queue recovered solution: {}", e));
        }

        // 2. Delete the recovery file
        if let Err(e) = fs::remove_file(&path) {
            eprintln!("WARNING: Successfully queued recovered solution but FAILED TO DELETE RECOVERY FILE {:?}: {}", path, e);
        } else {
            println!("✅ Successfully recovered and queued solution for address {} / challenge {}.", mining_address, challenge_id);
        }
    }
    Ok(())
}

// ===============================================
// MINING MODE FUNCTIONS (Core Logic Only)
// ===============================================

/// MODE A: Persistent Key Continuous Mining
#[allow(unused_assignments)] // Suppress warnings for final_hashes/final_elapsed assignments
pub fn run_persistent_key_mining(context: MiningContext, skey_hex: &String) -> Result<(), String> {
    let key_pair = cardano::generate_cardano_key_pair_from_skey(skey_hex);
    let mining_address = key_pair.2.to_bech32().unwrap();
    let mut final_hashes: u64 = 0;
    let mut final_elapsed: f64 = 0.0;
    let reg_message = context.tc_response.message.clone();
    let data_dir = DataDir::Persistent(&mining_address);

    println!("\n[REGISTRATION] Attempting initial registration for address: {}", mining_address);
    let reg_signature = cardano::cip8_sign(&key_pair, &reg_message);
    if let Err(e) = api::register_address(
        &context.client, &context.api_url, &mining_address, &context.tc_response.message, &reg_signature.0, &hex::encode(key_pair.1.as_ref()),
    ) {
        eprintln!("Address registration failed: {}. Cannot start mining.", e);
        return Err("Address registration failed.".to_string());
    }

    println!("\n==============================================");
    println!("⛏️  Shadow Harvester: PERSISTENT KEY MINING Mode ({})", if context.cli_challenge.is_some() { "FIXED CHALLENGE" } else { "DYNAMIC POLLING" });
    println!("==============================================");
    if context.donate_to_option.is_some() { println!("Donation Target: {}", context.donate_to_option.unwrap()); }

    let mut current_challenge_id = String::new();
    let mut last_active_challenge_data: Option<ChallengeData> = None;
    loop {
        let challenge_params = match utils::get_challenge_params(&context.client, &context.api_url, context.cli_challenge, &mut current_challenge_id) {
            Ok(Some(params)) => {
                last_active_challenge_data = Some(params.clone());
                params
            },
            Ok(None) => continue,
            Err(e) => {
                // If a challenge ID is set AND we detect a network failure, continue mining.
                if !current_challenge_id.is_empty() && e.contains("API request failed") {
                    eprintln!("⚠️ Challenge API poll failed (Network Error): {}. Continuing mining with previous challenge parameters (ID: {})...", e, current_challenge_id);
                    last_active_challenge_data.as_ref().cloned().ok_or_else(|| {
                        format!("FATAL LOGIC ERROR: Challenge ID {} is set but no previous challenge data was stored.", current_challenge_id)
                    })?
                } else {
                    eprintln!("⚠️ Critical API Error during challenge check: {}. Retrying in 1 minute...", e);
                    std::thread::sleep(std::time::Duration::from_secs(60));
                    continue;
                }
            }
        };

        // Check for unsubmitted solutions from previous run
        if let Some(base_dir) = context.data_dir {
            check_for_unsubmitted_solutions(base_dir, &challenge_params.challenge_id, &mining_address, &data_dir)?;
        }

        if let Some(base_dir) = context.data_dir { data_dir.save_challenge(base_dir, &challenge_params)?; }
        print_mining_setup(&context.api_url, Some(mining_address.as_str()), context.threads, &challenge_params);

        loop {
            // UPDATED CALL: Removed client and api_url
            let (result, total_hashes, elapsed_secs) = run_single_mining_cycle(
                mining_address.clone(), context.threads, context.donate_to_option, &challenge_params, context.data_dir, None,
            );
            final_hashes = total_hashes; final_elapsed = elapsed_secs;

            match result {
                MiningResult::FoundAndQueued => {
                    if let Some(ref destination_address) = context.donate_to_option {
                        let donation_message = format!("Assign accumulated Scavenger rights to: {}", destination_address);
                        let donation_signature = cardano::cip8_sign(&key_pair, &donation_message);

                        // Intentionally perform donation attempt synchronously here.
                        match api::donate_to(
                            &context.client, &context.api_url, &mining_address, destination_address, &donation_signature.0,
                        ) {
                            Ok(id) => println!("🚀 Donation initiated successfully. ID: {}", id),
                            Err(e) => eprintln!("⚠️ Donation failed (synchronous attempt): {}", e),
                        }
                    }

                    println!("\n✅ Solution queued. Checking for new challenge/expiration.");
                    break; // Break the inner loop to re-poll the challenge API.
                },
                MiningResult::AlreadySolved => {
                    println!("\n✅ Challenge already solved on network. Stopping current mining.");
                    // Solution saved by submitter/already exists, so check for a new challenge.
                    break;
                }
                MiningResult::MiningFailed => {
                    eprintln!("\n⚠️ Mining cycle failed. Checking if challenge is still valid before retrying...");
                    if context.cli_challenge.is_none() {
                        match api::get_active_challenge_data(&context.client,&context.api_url) {
                            Ok(active_params) if active_params.challenge_id == current_challenge_id => {
                                eprintln!("Challenge is still valid. Retrying mining cycle in 1 minute...");
                                std::thread::sleep(std::time::Duration::from_secs(60));
                            },
                            Ok(_) | Err(_) => {
                                eprintln!("Challenge appears to have changed or API is unreachable. Stopping current mining and checking for new challenge...");
                                break;
                            }
                        }
                    } else {
                        eprintln!("Fixed challenge. Retrying mining cycle in 1 minute...");
                        std::thread::sleep(std::time::Duration::from_secs(60));
                    }
                }
            }
        }
        let stats_result = api::fetch_statistics(&context.client, &context.api_url, &mining_address);
        print_statistics(stats_result, final_hashes, final_elapsed);
    }
}


/// MODE B: Mnemonic Sequential Mining
pub fn run_mnemonic_sequential_mining(cli: &Cli, context: MiningContext, mnemonic_phrase: String) -> Result<(), String> {
    let reg_message = context.tc_response.message.clone();
    let mut wallet_deriv_index: u32 = 0;
    let mut first_run = true;
    let mut max_registered_index = None;
    let mut backoff_challenge = crate::backoff::Backoff::new(5, 300, 2.0);
    let mut backoff_reg = crate::backoff::Backoff::new(5, 300, 2.0);
    let mut last_seen_challenge_id = String::new();
    let mut current_challenge_id = String::new();
    let mut last_active_challenge_data: Option<ChallengeData> = None;

    println!("\n==============================================");
    println!("⛏️  Shadow Harvester: MNEMONIC SEQUENTIAL MINING Mode ({})", if context.cli_challenge.is_some() { "FIXED CHALLENGE" } else { "DYNAMIC POLLING" });
    println!("==============================================");
    if context.donate_to_option.is_some() { println!("Donation Target: {}", context.donate_to_option.unwrap()); }

    loop {
        // --- 1. Challenge Discovery and Initial Index Reset ---
        backoff_challenge.reset();
        let old_challenge_id = last_seen_challenge_id.clone();
        current_challenge_id.clear();

        let challenge_params: ChallengeData = match utils::get_challenge_params(&context.client, &context.api_url, context.cli_challenge, &mut current_challenge_id) {
            Ok(Some(params)) => {
                backoff_challenge.reset();
                last_active_challenge_data = Some(params.clone());
                if first_run || (context.cli_challenge.is_none() && params.challenge_id != old_challenge_id) {
                    // Create a dummy DataDir with index 0 to calculate the base path for scanning
                    let temp_data_dir = DataDir::Mnemonic(DataDirMnemonic { mnemonic: &mnemonic_phrase, account: cli.mnemonic_account, deriv_index: 0 });

                    let next_index_from_receipts = next_wallet_deriv_index_for_challenge(&cli.data_dir, &params.challenge_id, &temp_data_dir)?;

                    // FIX: Take the maximum of the index derived from receipts and the CLI starting index.
                    wallet_deriv_index = next_index_from_receipts.max(cli.mnemonic_starting_index);
                }
                last_seen_challenge_id = params.challenge_id.clone();
                params
            },
            Ok(None) => { backoff_challenge.reset(); continue; },
            Err(e) => {
                // If a challenge ID is set AND we detect a network failure, continue mining.
                if !current_challenge_id.is_empty() && e.contains("API request failed") {
                    eprintln!("⚠️ Challenge API poll failed (Network Error): {}. Continuing mining with previous challenge parameters (ID: {})...", e, current_challenge_id);
                    backoff_challenge.reset();
                    last_active_challenge_data.as_ref().cloned().ok_or_else(|| {
                        format!("FATAL LOGIC ERROR: Challenge ID {} is set but no previous challenge data was stored.", current_challenge_id)
                    })?
                } else {
                    eprintln!("⚠️ Critical API Error during challenge polling: {}. Retrying with exponential backoff...", e);
                    backoff_challenge.sleep();
                    continue;
                }
            }
        };
        first_run = false;

        // Save challenge details
        let temp_data_dir = DataDir::Mnemonic(DataDirMnemonic { mnemonic: &mnemonic_phrase, account: cli.mnemonic_account, deriv_index: 0 });
        if let Some(base_dir) = context.data_dir { temp_data_dir.save_challenge(base_dir, &challenge_params)?; }

        // --- 2. Continuous Index Skip Check ---
        // This loop ensures we skip indices with existing receipts, even if the index hasn't changed.
        'skip_check: loop {
            let wallet_config = DataDirMnemonic { mnemonic: &mnemonic_phrase, account: cli.mnemonic_account, deriv_index: wallet_deriv_index };
            let data_dir = DataDir::Mnemonic(wallet_config); // Full DataDir for recovery check

            // Get the temporary mining address for this index (needed for queue file lookup/recovery)
            let mining_address_temp = cardano::derive_key_pair_from_mnemonic(&mnemonic_phrase, cli.mnemonic_account, wallet_deriv_index).2.to_bech32().unwrap();

            // Check for unsubmitted solutions (recovery file or pending queue)
            if let Some(base_dir) = context.data_dir {
                if wallet_deriv_index >= cli.mnemonic_starting_index {
                    // 1. Check for crash recovery file (found.json)
                    check_for_unsubmitted_solutions(base_dir, &challenge_params.challenge_id, &mining_address_temp, &data_dir)?;

                    // 2. Check if a solution for this address/challenge is already in the pending queue
                    if is_solution_pending_in_queue(base_dir, &mining_address_temp, &challenge_params.challenge_id)? {
                        println!("\nℹ️ Index {} has a pending submission in the queue. Skipping and checking next index.", wallet_deriv_index);
                        wallet_deriv_index = wallet_deriv_index.wrapping_add(1);
                        continue 'skip_check;
                    }
                }
            }

            // --- Final Receipt Check (Multi-Path Resumption) ---
            if let Some(base_dir) = context.data_dir {
                // 1. Check Correct Mnemonic Path (where it should be)
                if receipt_exists_for_index(base_dir, &challenge_params.challenge_id, &wallet_config)? {
                    println!("\nℹ️ Index {} already has a local receipt (Mnemonic path). Skipping.", wallet_deriv_index);
                    wallet_deriv_index = wallet_deriv_index.wrapping_add(1);
                    continue 'skip_check;
                }

                // 2. Check INCORRECT Persistent Path (where submitter currently writes receipts due to heuristic)
                let mut persistent_path = data_dir.challenge_dir(base_dir, &challenge_params.challenge_id)?;
                persistent_path.push("persistent");
                persistent_path.push(&mining_address_temp); // The address derived for this index
                persistent_path.push(FILE_NAME_RECEIPT);

                if persistent_path.exists() {
                    println!("\n⚠️ Index {} found receipt in Persistent path (Submitter heuristic failure). Skipping.", wallet_deriv_index);
                    wallet_deriv_index = wallet_deriv_index.wrapping_add(1);
                    continue 'skip_check;
                }
            }

            // If none of the above conditions met, we break and mine.
            break 'skip_check;
        }

        // --- 3. Key Generation, Registration, and Mining ---
        let key_pair = cardano::derive_key_pair_from_mnemonic(&mnemonic_phrase, cli.mnemonic_account, wallet_deriv_index);
        let mining_address = key_pair.2.to_bech32().unwrap();

        println!("\n[CYCLE START] Deriving Address Index {}: {}", wallet_deriv_index, mining_address);
        if match max_registered_index { Some(idx) => wallet_deriv_index > idx, None => true } {
            let stats_result = api::fetch_statistics(&context.client, &context.api_url, &mining_address);
            match stats_result {
                Ok(stats) => { println!("  Crypto Receipts (Solutions): {}", stats.crypto_receipts); println!("  Night Allocation: {}", stats.night_allocation); }
                Err(_) => {
                    let reg_signature = cardano::cip8_sign(&key_pair, &reg_message);
                    if let Err(e) = api::register_address(&context.client, &context.api_url, &mining_address, &reg_message, &reg_signature.0, &hex::encode(key_pair.1.as_ref())) {
                        eprintln!("Registration failed: {}. Retrying with exponential backoff...", e); backoff_reg.sleep(); continue;
                    }
                }
            }
            max_registered_index = Some(wallet_deriv_index); backoff_reg.reset();
        }

        print_mining_setup(&context.api_url, Some(mining_address.as_str()), context.threads, &challenge_params);

        // UPDATED CALL: Removed client and api_url
        let (result, total_hashes, elapsed_secs) = run_single_mining_cycle(
            mining_address.clone(), context.threads, context.donate_to_option, &challenge_params, context.data_dir, None,
        );

        // --- 4. Post-Mining Index Advancement ---
        match result {
            MiningResult::FoundAndQueued => {
                if let Some(ref destination_address) = context.donate_to_option {
                    // key_pair is available locally in this loop scope
                    let donation_message = format!("Assign accumulated Scavenger rights to: {}", destination_address);
                    let donation_signature = cardano::cip8_sign(&key_pair, &donation_message);

                    // Attempt donation synchronously. Ignore result here to keep the main flow clean.
                    match api::donate_to(
                        &context.client, &context.api_url, &mining_address, destination_address, &donation_signature.0,
                    ) {
                        Ok(id) => println!("🚀 Donation initiated successfully. ID: {}", id),
                        Err(e) => eprintln!("⚠️ Donation failed (synchronous attempt): {}", e),
                    }
                }

                wallet_deriv_index = wallet_deriv_index.wrapping_add(1);
                println!("\n✅ Solution queued. Incrementing index to {}.", wallet_deriv_index);
            },
            MiningResult::AlreadySolved => {
                // This scenario means the submitter/API reported it was already solved
                wallet_deriv_index = wallet_deriv_index.wrapping_add(1);
                println!("\n✅ Challenge already solved. Incrementing index to {}.", wallet_deriv_index);
            }
            MiningResult::MiningFailed => {
                eprintln!("\n⚠️ Mining cycle failed. Retrying with the SAME index {}.", wallet_deriv_index);
            }
        }
        let stats_result = api::fetch_statistics(&context.client, &context.api_url, &mining_address);
        print_statistics(stats_result, total_hashes, elapsed_secs);
    }
}

/// MODE C: Ephemeral Key Per Cycle Mining
#[allow(unused_assignments)] // Suppress warnings for final_hashes/final_elapsed assignments
pub fn run_ephemeral_key_mining(context: MiningContext) -> Result<(), String> {
    println!("\n==============================================");
    println!("⛏️  Shadow Harvester: EPHEMERAL KEY MINING Mode ({})", if context.cli_challenge.is_some() { "FIXED CHALLENGE" } else { "DYNAMIC POLLING" });
    println!("==============================================");
    if context.donate_to_option.is_some() { println!("Donation Target: {}", context.donate_to_option.unwrap()); }

    let mut final_hashes: u64 = 0;
    let mut final_elapsed: f64 = 0.0;
    let mut current_challenge_id = String::new();
    let mut last_active_challenge_data: Option<ChallengeData> = None;

    loop {
        let challenge_params: ChallengeData = match utils::get_challenge_params(&context.client, &context.api_url, context.cli_challenge, &mut current_challenge_id) {
            Ok(Some(p)) => {
                last_active_challenge_data = Some(p.clone());
                p
            },
            Ok(None) => continue,
            Err(e) => {
                // If a challenge ID is set AND we detect a network failure, continue mining.
                if !current_challenge_id.is_empty() && e.contains("API request failed") {
                    eprintln!("⚠️ Challenge API poll failed (Network Error): {}. Continuing mining with previous challenge parameters (ID: {})...", e, current_challenge_id);
                    last_active_challenge_data.as_ref().cloned().ok_or_else(|| {
                        format!("FATAL LOGIC ERROR: Challenge ID {} is set but no previous challenge data was stored.", current_challenge_id)
                    })?
                } else {
                    eprintln!("⚠️ Could not fetch active challenge (Ephemeral Key Mode): {}. Retrying in 5 minutes...", e);
                    std::thread::sleep(std::time::Duration::from_secs(5 * 60));
                    continue;
                }
            }
        };

        let key_pair = cardano::generate_cardano_key_and_address();
        let generated_mining_address = key_pair.2.to_bech32().unwrap();
        let data_dir = DataDir::Ephemeral(&generated_mining_address);

        if let Some(base_dir) = context.data_dir { data_dir.save_challenge(base_dir, &challenge_params)?; }
        println!("\n[CYCLE START] Generated Address: {}", generated_mining_address);

        let reg_message = context.tc_response.message.clone();
        let reg_signature = cardano::cip8_sign(&key_pair, &reg_message);

        if let Err(e) = api::register_address(&context.client, &context.api_url, &generated_mining_address, &context.tc_response.message, &reg_signature.0, &hex::encode(key_pair.1.as_ref())) {
            eprintln!("Registration failed: {}. Retrying in 5 minutes...", e); std::thread::sleep(std::time::Duration::from_secs(5 * 60)); continue;
        }

        print_mining_setup(&context.api_url, Some(&generated_mining_address.to_string()), context.threads, &challenge_params);

        // UPDATED CALL: Removed client and api_url
        let (result, total_hashes, elapsed_secs) = run_single_mining_cycle(
                generated_mining_address.to_string(), context.threads, context.donate_to_option, &challenge_params, context.data_dir, None,
            );
        final_hashes = total_hashes; final_elapsed = elapsed_secs;

        match result {
            MiningResult::FoundAndQueued => {
                if let Some(ref destination_address) = context.donate_to_option {
                    // key_pair is available locally in this loop scope
                    let donation_message = format!("Assign accumulated Scavenger rights to: {}", destination_address);
                    let donation_signature = cardano::cip8_sign(&key_pair, &donation_message);

                    // Attempt donation synchronously. Ignore result here to keep the main thread fast.
                    match api::donate_to(
                        &context.client, &context.api_url, &generated_mining_address, destination_address, &donation_signature.0,
                    ) {
                        Ok(id) => println!("🚀 Donation initiated successfully. ID: {}", id),
                        Err(e) => eprintln!("⚠️ Donation failed (synchronous attempt): {}", e),
                    }
                }
                eprintln!("Solution queued. Starting next cycle immediately...");
            }
            MiningResult::AlreadySolved => { eprintln!("Solution was already accepted by the network. Starting next cycle immediately..."); }
            MiningResult::MiningFailed => { eprintln!("Mining cycle failed. Retrying next cycle in 1 minute..."); std::thread::sleep(std::time::Duration::from_secs(60)); }
        }

        let stats_result = api::fetch_statistics(&context.client, &context.api_url, &generated_mining_address);
        print_statistics(stats_result, final_hashes, final_elapsed);
        println!("\n[CYCLE END] Starting next mining cycle immediately...");
    }
}

/// MODE D: Wallet Pool Mining - Multiple wallets from JSON file, concurrent mining with rotation
pub fn run_wallet_pool_mining(context: MiningContext, wallets_file: &str, concurrent_wallets: usize) -> Result<(), String> {
    use std::sync::mpsc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    println!("\n╔════════════════════════════════════════════╗");
    println!("║  Shadow Harvester - Wallet Pool Mining    ║");
    println!("╚════════════════════════════════════════════╝");
    println!("📁 Wallets: {} | 🔄 Concurrent: {}", wallets_file, concurrent_wallets);
    if context.donate_to_option.is_some() {
        println!("💝 Donations: {}", context.donate_to_option.unwrap());
    }

    // Load wallets from JSON file
    let wallets_json = fs::read_to_string(wallets_file)
        .map_err(|e| format!("Failed to read wallets file '{}': {}", wallets_file, e))?;

    let wallets: Vec<WalletConfig> = serde_json::from_str(&wallets_json)
        .map_err(|e| format!("Failed to parse wallets JSON: {}", e))?;

    if wallets.is_empty() {
        return Err("No wallets found in wallets file".to_string());
    }

    let total_wallets = wallets.len();
    let reg_message = context.tc_response.message.clone();

    println!("\n✅ Loaded {} wallets\n", total_wallets);

    let mut last_challenge_id = String::new();

    loop {
        // Get current challenge
        let mut current_challenge_id = String::new();
        let challenge_params: ChallengeData = match utils::get_challenge_params(&context.client, &context.api_url, context.cli_challenge, &mut current_challenge_id) {
            Ok(Some(params)) => params,
            Ok(None) => {
                std::thread::sleep(std::time::Duration::from_secs(30));
                continue;
            }
            Err(e) => {
                eprintln!("⚠️ API Error: {}. Retrying in 5 minutes...", e);
                std::thread::sleep(std::time::Duration::from_secs(5 * 60));
                continue;
            }
        };

        // Detect challenge change
        if !last_challenge_id.is_empty() && last_challenge_id != challenge_params.challenge_id {
            println!("\n🔄 NEW CHALLENGE DETECTED!");
            println!("   Previous: {} → Current: {}", last_challenge_id, challenge_params.challenge_id);
            println!("   ⚠️  Stopping all active mining to switch to new challenge...\n");
        }
        last_challenge_id = challenge_params.challenge_id.clone();

        // Fetch work_to_star_rate for NIGHT estimation
        let star_rates = api::fetch_work_to_star_rate(&context.client, &context.api_url)
            .unwrap_or(crate::data_types::WorkToStarRate(vec![]));

        // Get challenge response for next_challenge_starts_at
        let next_challenge_time = match api::fetch_challenge_status(&context.client, &context.api_url) {
            Ok(challenge_response) => challenge_response.next_challenge_starts_at,
            Err(_) => None,
        };

        // Initialize per-wallet statistics
        println!("🔄 Initializing wallet statistics...");
        let mut wallet_stats_vec = Vec::new();
        let mut total_network_solutions = 0;
        let mut night_per_solution = 0.0;

        for wallet in &wallets {
            let mnemonic = &wallet.mnemonic;
            let key_pair = cardano::derive_key_pair_from_mnemonic(mnemonic, 0, 0);
            let address = key_pair.2.to_bech32().unwrap();

            // Fetch individual wallet stats
            let (solved_count, estimated_night) = match api::fetch_statistics(&context.client, &context.api_url, &address) {
                Ok(stats) => {
                    total_network_solutions = stats.recent_crypto_receipts;

                    // Calculate NIGHT per solution for display
                    let day_index = (challenge_params.day as usize).saturating_sub(1);
                    if let Some(&stars_per_day) = star_rates.0.get(day_index) {
                        if stats.recent_crypto_receipts > 0 {
                            night_per_solution = (stars_per_day as f64 / stats.recent_crypto_receipts as f64) / 1_000_000.0;
                        }
                    }

                    // Use the API-provided night_allocation (already calculated by server)
                    let wallet_night = stats.night_allocation as f64 / 1_000_000.0;
                    (stats.crypto_receipts, wallet_night)
                },
                Err(_) => (0, 0.0),
            };

            wallet_stats_vec.push(WalletStats {
                name: wallet.name.clone(),
                address: address.clone(),
                status: WalletStatus::Waiting,
                solved_count,
                estimated_night,
            });
        }

        // Initialize live stats
        let live_stats = Arc::new(Mutex::new(LiveStats {
            wallets: wallet_stats_vec,
            challenge_id: challenge_params.challenge_id.clone(),
            challenge_deadline: challenge_params.latest_submission.clone(),
            challenge_day: challenge_params.day,
            next_challenge_time,
            start_time: Instant::now(),
            total_network_solutions,
            night_per_solution,
        }));

        // Start display update thread with periodic stats fetching
        let stats_clone = Arc::clone(&live_stats);
        let display_running = Arc::new(AtomicBool::new(true));
        let display_running_clone = Arc::clone(&display_running);
        let client_clone = context.client.clone();
        let api_url_clone = context.api_url.clone();
        let star_rates_clone = star_rates.clone();
        let challenge_day = challenge_params.day;

        let display_handle = thread::spawn(move || {
            let mut loop_count = 0;
            while display_running_clone.load(Ordering::SeqCst) {
                // Display current stats every iteration
                if let Ok(stats) = stats_clone.lock() {
                    stats.display();
                }

                // Update network statistics every 30 seconds (15 display cycles)
                if loop_count % 15 == 0 {
                    if let Ok(mut stats) = stats_clone.lock() {
                        // Update network stats from first wallet
                        if let Some(first_wallet) = stats.wallets.first() {
                            let address = first_wallet.address.clone();
                            drop(stats); // Release lock before API call

                            if let Ok(network_stats) = api::fetch_statistics(&client_clone, &api_url_clone, &address) {
                                if let Ok(mut stats) = stats_clone.lock() {
                                    stats.total_network_solutions = network_stats.recent_crypto_receipts;

                                    // Update NIGHT per solution for display
                                    let day_index = (challenge_day as usize).saturating_sub(1);
                                    if let Some(&stars_per_day) = star_rates_clone.0.get(day_index) {
                                        if network_stats.recent_crypto_receipts > 0 {
                                            stats.night_per_solution = (stars_per_day as f64 / network_stats.recent_crypto_receipts as f64) / 1_000_000.0;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                loop_count += 1;
                thread::sleep(Duration::from_secs(2));
            }
        });

        // Dynamic wallet rotation: maintain N concurrent miners at all times
        use std::sync::mpsc;
        let (result_tx, result_rx) = mpsc::channel();

        // Start challenge monitoring thread
        let monitor_running = Arc::new(AtomicBool::new(true));
        let monitor_running_clone = Arc::clone(&monitor_running);
        let current_challenge_id = challenge_params.challenge_id.clone();
        let context_clone_monitor = context.to_owned();

        let monitor_handle = thread::spawn(move || {
            while monitor_running_clone.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_secs(30));

                let mut temp_challenge_id = String::new();
                if let Ok(Some(new_params)) = utils::get_challenge_params(
                    &context_clone_monitor.client,
                    &context_clone_monitor.api_url,
                    context_clone_monitor.cli_challenge.as_ref(),
                    &mut temp_challenge_id
                ) {
                    if new_params.challenge_id != current_challenge_id {
                        eprintln!("\n🔄 NEW CHALLENGE DETECTED: {} → {}", current_challenge_id, new_params.challenge_id);
                        eprintln!("   Stopping current mining to switch challenges...");
                        monitor_running_clone.store(false, Ordering::SeqCst);
                        return true; // Signal new challenge detected
                    }
                }
            }
            false // Normal exit
        });

        let mut wallet_index = 0;
        let mut active_miners = 0;

        // Store thread handles and stop signals to ensure proper cleanup
        use std::collections::HashMap;
        let mut thread_handles: HashMap<String, thread::JoinHandle<()>> = HashMap::new();
        let mut stop_signals: HashMap<String, Arc<AtomicBool>> = HashMap::new();

        // Start initial batch of concurrent wallets
        let initial_batch = concurrent_wallets.min(wallets.len());
        for i in 0..initial_batch {
            let wallet = &wallets[i];

            // Update wallet status to Mining
            {
                let mut stats = live_stats.lock().unwrap();
                if let Some(w) = stats.wallets.iter_mut().find(|w| w.name == wallet.name) {
                    w.status = WalletStatus::Mining;
                }
            }

            // Create stop signal for this wallet
            let stop_signal = Arc::new(AtomicBool::new(false));
            stop_signals.insert(wallet.name.clone(), Arc::clone(&stop_signal));

            let wallet_clone = wallet.clone();
            let context_clone = context.to_owned();
            let challenge_params_clone = challenge_params.clone();
            let reg_message_clone = reg_message.clone();
            let stats_clone = Arc::clone(&live_stats);
            let tx = result_tx.clone();

            let handle = thread::spawn(move || {
                let result = mine_single_wallet_quiet(
                    wallet_clone.clone(),
                    context_clone,
                    challenge_params_clone,
                    reg_message_clone,
                    stats_clone.clone(),
                    stop_signal, // Pass stop signal
                );
                let _ = tx.send((wallet_clone.name.clone(), result));
            });

            // CRITICAL: Store thread handle for proper cleanup
            thread_handles.insert(wallet.name.clone(), handle);

            wallet_index += 1;
            active_miners += 1;
        }

        println!("\n🔄 Started {} concurrent wallets (Total: {})", active_miners, wallets.len());

        // Process results and dynamically rotate wallets
        let mut total_completed = 0;
        let mut new_challenge_detected = false;
        while total_completed < wallets.len() {
            // Check if monitor detected new challenge
            if !monitor_running.load(Ordering::SeqCst) {
                eprintln!("⚠️  New challenge detected! Stopping wallet rotation early.");
                new_challenge_detected = true;
                break;
            }

            // Use recv_timeout to periodically check monitor status
            match result_rx.recv_timeout(Duration::from_secs(1)) {
                Ok((wallet_name, result)) => {
                active_miners -= 1;
                total_completed += 1;

                // CRITICAL: Join the completed thread to ensure ROM is fully released
                if let Some(handle) = thread_handles.remove(&wallet_name) {
                    let _ = handle.join(); // Wait for thread to fully exit and clean up
                }
                // Clean up stop signal for completed thread
                stop_signals.remove(&wallet_name);

                // Get wallet address if we need fresh stats
                let wallet_address = if result == MiningResult::FoundAndQueued {
                    let stats = live_stats.lock().unwrap();
                    stats.wallets.iter().find(|w| w.name == wallet_name).map(|w| w.address.clone())
                } else {
                    None
                };

                // Fetch fresh stats from API if needed
                let fresh_stats = if let Some(ref addr) = wallet_address {
                    api::fetch_statistics(&context.client, &context.api_url, addr).ok()
                } else {
                    None
                };

                // Update the wallet status IMMEDIATELY
                {
                    let mut stats = live_stats.lock().unwrap();
                    if let Some(w) = stats.wallets.iter_mut().find(|w| w.name == wallet_name) {
                        w.status = match result {
                            MiningResult::FoundAndQueued => {
                                if let Some(ref wallet_stats) = fresh_stats {
                                    w.solved_count = wallet_stats.crypto_receipts;
                                    w.estimated_night = wallet_stats.night_allocation as f64 / 1_000_000.0;
                                }
                                WalletStatus::Solved
                            },
                            MiningResult::AlreadySolved => WalletStatus::Skipped,
                            MiningResult::MiningFailed => WalletStatus::Failed,
                        };
                    }
                }

                // ROTATION: Immediately start next wallet if available
                if wallet_index < wallets.len() {
                    let next_wallet = &wallets[wallet_index];
                    println!("🔄 '{}' completed → Starting '{}'  ({}/{})",
                        wallet_name, next_wallet.name, total_completed, wallets.len());

                    // Update next wallet status to Mining
                    {
                        let mut stats = live_stats.lock().unwrap();
                        if let Some(w) = stats.wallets.iter_mut().find(|w| w.name == next_wallet.name) {
                            w.status = WalletStatus::Mining;
                        }
                    }

                    // Create stop signal for this wallet
                    let stop_signal = Arc::new(AtomicBool::new(false));
                    stop_signals.insert(next_wallet.name.clone(), Arc::clone(&stop_signal));

                    let wallet_clone = next_wallet.clone();
                    let context_clone = context.to_owned();
                    let challenge_params_clone = challenge_params.clone();
                    let reg_message_clone = reg_message.clone();
                    let stats_clone = Arc::clone(&live_stats);
                    let tx = result_tx.clone();

                    let handle = thread::spawn(move || {
                        let result = mine_single_wallet_quiet(
                            wallet_clone.clone(),
                            context_clone,
                            challenge_params_clone,
                            reg_message_clone,
                            stats_clone.clone(),
                            stop_signal, // Pass stop signal
                        );
                        let _ = tx.send((wallet_clone.name.clone(), result));
                    });

                    // CRITICAL: Store new thread handle
                    thread_handles.insert(next_wallet.name.clone(), handle);

                    wallet_index += 1;
                    active_miners += 1;
                } else {
                    println!("✓ '{}' completed  ({}/{})",
                        wallet_name, total_completed, wallets.len());
                }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Timeout - just loop again to check monitor status
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    // All senders dropped - shouldn't happen but break if it does
                    eprintln!("⚠️  Channel disconnected unexpectedly");
                    break;
                }
            }
        }

        // Stop the monitor thread
        monitor_running.store(false, Ordering::SeqCst);
        let _ = monitor_handle.join();

        if new_challenge_detected {
            println!("\n⚡ New challenge detected! Cleaning up before switching...");

            // CRITICAL: Signal all active mining threads to stop immediately
            println!("   Stopping {} active mining threads...", stop_signals.len());
            for (_name, stop_signal) in stop_signals.iter() {
                stop_signal.store(true, Ordering::SeqCst);
            }

            // CRITICAL: Wait for all active mining threads to complete
            // They should abort quickly now that stop signal is set
            println!("   Waiting for {} active miners to abort and exit...", active_miners);
            let mut remaining = active_miners;
            while remaining > 0 {
                if let Ok((wallet_name, _result)) = result_rx.recv_timeout(Duration::from_secs(5)) {
                    // CRITICAL: Join thread to ensure full cleanup
                    if let Some(handle) = thread_handles.remove(&wallet_name) {
                        let _ = handle.join();
                    }
                    remaining -= 1;
                    println!("   {} miners remaining...", remaining);
                } else {
                    println!("   Timeout waiting for miners - continuing anyway");
                    break;
                }
            }

            // Join any remaining threads (should complete quickly since stop signal was set)
            if !thread_handles.is_empty() {
                println!("   Joining {} remaining threads...", thread_handles.len());
                for (_name, handle) in thread_handles.drain() {
                    let _ = handle.join();
                }
                println!("   All threads stopped.");
            }

            // Stop display thread
            display_running.store(false, Ordering::SeqCst);
            let _ = display_handle.join();

            // CRITICAL: Explicitly drop large objects to free memory
            // Drop live_stats (contains wallet data)
            drop(live_stats);
            // Drop challenge_params (contains 1GB Arc<Rom>)
            drop(challenge_params);

            println!("   Memory cleanup complete. Switching to new challenge...");

            // Force garbage collection by sleeping briefly
            thread::sleep(Duration::from_millis(100));

            // Immediately loop back to get the new challenge
            continue;
        }

        println!("\n✅ All wallets processed for this challenge!");

        // Wait a moment for background submitter to process any pending solutions
        println!("⏳ Waiting for background submissions to complete...");
        thread::sleep(Duration::from_secs(5));

        // Refresh all wallet stats one final time to get accurate counts
        println!("🔄 Refreshing final statistics from API...");
        {
            let mut stats = live_stats.lock().unwrap();
            for wallet_stat in stats.wallets.iter_mut() {
                if let Ok(fresh) = api::fetch_statistics(&context.client, &context.api_url, &wallet_stat.address) {
                    wallet_stat.solved_count = fresh.crypto_receipts;
                    wallet_stat.estimated_night = fresh.night_allocation as f64 / 1_000_000.0;
                }
            }
        }

        // Stop display thread
        display_running.store(false, Ordering::SeqCst);
        let _ = display_handle.join();

        // Final display
        {
            let stats = live_stats.lock().unwrap();
            stats.display();

            let total_time = stats.start_time.elapsed().as_secs();
            let solved = stats.wallets.iter().filter(|w| w.status == WalletStatus::Solved).count();
            let skipped = stats.wallets.iter().filter(|w| w.status == WalletStatus::Skipped).count();
            let failed = stats.wallets.iter().filter(|w| w.status == WalletStatus::Failed).count();

            println!();
            println!("╔══════════════════════════════════════════════════════════╗");
            println!("║              ✅ Challenge Complete!                      ║");
            println!("╠══════════════════════════════════════════════════════════╣");
            println!("║  Solved:   {}  |  Skipped: {}  |  Failed: {}              ║", solved, skipped, failed);
            println!("║  Total Time: {}m {}s                                      ║", total_time / 60, total_time % 60);
            println!("╚══════════════════════════════════════════════════════════╝");
        }

        // CRITICAL: Join any remaining thread handles to ensure all ROMs are released
        println!("🧹 Joining {} remaining mining threads...", thread_handles.len());
        for (_name, handle) in thread_handles.drain() {
            let _ = handle.join();
        }

        // CRITICAL: Explicitly drop large objects to free memory before next challenge
        // This prevents memory accumulation across multiple challenges
        drop(live_stats);
        drop(challenge_params);
        println!("🧹 Memory cleanup complete for challenge cycle.");

        // Wait for new challenge or exit based on mode
        if context.cli_challenge.is_some() {
            println!("\n✅ Fixed challenge mode - all wallets completed. Exiting.");
            break;
        } else {
            println!("\n⏳ Checking for next challenge...");
            // Force a brief pause to allow memory to be reclaimed
            thread::sleep(Duration::from_millis(500));

            // Poll for new challenge instead of sleeping for 5 minutes
            let mut attempts = 0;
            let max_attempts = 60; // Check for up to 30 minutes (60 * 30 seconds)
            loop {
                std::thread::sleep(std::time::Duration::from_secs(30));
                attempts += 1;

                // Check if new challenge is available
                let mut temp_challenge_id = String::new();
                match utils::get_challenge_params(&context.client, &context.api_url, context.cli_challenge, &mut temp_challenge_id) {
                    Ok(Some(new_params)) if new_params.challenge_id != last_challenge_id => {
                        println!("✅ New challenge {} detected! Starting immediately...", new_params.challenge_id);
                        break;
                    }
                    Ok(Some(_)) => {
                        if attempts % 10 == 0 {
                            println!("   Still waiting for new challenge... ({} minutes elapsed)", attempts / 2);
                        }
                    }
                    Ok(None) => {
                        println!("   No active challenge. Waiting...");
                    }
                    Err(e) => {
                        eprintln!("   ⚠️ Error checking for new challenge: {}", e);
                    }
                }

                if attempts >= max_attempts {
                    println!("   Maximum wait time reached. Will retry on next cycle.");
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Helper function to mine with a single wallet (quiet version for live stats)
fn mine_single_wallet_quiet(
    wallet: WalletConfig,
    context: OwnedMiningContext,
    challenge_params: ChallengeData,
    reg_message: String,
    _live_stats: Arc<Mutex<LiveStats>>,
    stop_signal: Arc<AtomicBool>, // NEW: Stop signal to abort mining when new challenge detected
) -> MiningResult {
    let mnemonic = wallet.mnemonic.clone();
    let key_pair = cardano::derive_key_pair_from_mnemonic(&mnemonic, 0, 0);
    let mining_address = key_pair.2.to_bech32().unwrap();

    let wallet_config = DataDirMnemonic {
        mnemonic: &mnemonic,
        account: 0,
        deriv_index: 0,
    };
    let data_dir = DataDir::Mnemonic(wallet_config);

    // Check for unsubmitted solutions (silent)
    if let Some(ref base_dir) = context.data_dir {
        let _ = check_for_unsubmitted_solutions(base_dir, &challenge_params.challenge_id, &mining_address, &data_dir);
    }

    // Check if already solved (silent)
    if let Some(ref base_dir) = context.data_dir {
        if let Ok(true) = is_solution_pending_in_queue(base_dir, &mining_address, &challenge_params.challenge_id) {
            return MiningResult::AlreadySolved;
        }

        if let Ok(true) = receipt_exists_for_index(base_dir, &challenge_params.challenge_id, &wallet_config) {
            return MiningResult::AlreadySolved;
        }
    }

    // Register address (silent)
    let _stats_result = api::fetch_statistics(&context.client, &context.api_url, &mining_address);
    if _stats_result.is_err() {
        let reg_signature = cardano::cip8_sign(&key_pair, &reg_message);
        if let Err(e) = api::register_address(
            &context.client,
            &context.api_url,
            &mining_address,
            &reg_message,
            &reg_signature.0,
            &hex::encode(key_pair.1.as_ref()),
        ) {
            let error_str = e.to_string();
            if !error_str.contains("400") && !error_str.contains("Bad Request") {
                return MiningResult::MiningFailed;
            }
        }
    }

    // Save challenge (silent)
    if let Some(ref base_dir) = context.data_dir {
        let _ = data_dir.save_challenge(base_dir, &challenge_params);
    }

    // Run mining cycle (silent)
    let (result, _total_hashes, _elapsed_secs) = run_single_mining_cycle(
        mining_address.clone(),
        context.threads,
        context.donate_to_option.as_ref(),
        &challenge_params,
        context.data_dir.as_deref(),
        Some(stop_signal), // Pass stop signal to allow early abort
    );

    // Handle donation (silent)
    if result == MiningResult::FoundAndQueued {
        if let Some(ref destination_address) = context.donate_to_option {
            let donation_message = format!("Assign accumulated Scavenger rights to: {}", destination_address);
            let donation_signature = cardano::cip8_sign(&key_pair, &donation_message);
            let _ = api::donate_to(&context.client, &context.api_url, &mining_address, destination_address, &donation_signature.0);
        }
    }

    result
}

/// Helper function to mine with a single wallet (legacy verbose version)
fn mine_single_wallet(
    wallet: WalletConfig,
    context: OwnedMiningContext,
    challenge_params: ChallengeData,
    reg_message: String,
) {
    println!("⛏️  [{}] Starting...", wallet.name);

    // Store mnemonic separately to create references
    let mnemonic = wallet.mnemonic.clone();

    // Derive key pair from mnemonic at index 0
    let key_pair = cardano::derive_key_pair_from_mnemonic(&mnemonic, 0, 0);
    let mining_address = key_pair.2.to_bech32().unwrap();

    // Create DataDir for this wallet
    let wallet_config = DataDirMnemonic {
        mnemonic: &mnemonic,
        account: 0,
        deriv_index: 0,
    };
    let data_dir = DataDir::Mnemonic(wallet_config);

    // Check for unsubmitted solutions from previous run
    if let Some(ref base_dir) = context.data_dir {
        let _ = check_for_unsubmitted_solutions(base_dir, &challenge_params.challenge_id, &mining_address, &data_dir);
    }

    // Check if wallet already has receipt for this challenge
    if let Some(ref base_dir) = context.data_dir {
        if let Ok(true) = is_solution_pending_in_queue(base_dir, &mining_address, &challenge_params.challenge_id) {
            println!("✓ [{}] Already has pending solution", wallet.name);
            return;
        }

        // Check for existing receipt
        if let Ok(true) = receipt_exists_for_index(base_dir, &challenge_params.challenge_id, &wallet_config) {
            println!("✓ [{}] Already solved", wallet.name);
            return;
        }
    }

    // Register address (silently)
    let _stats_result = api::fetch_statistics(&context.client, &context.api_url, &mining_address);
    if _stats_result.is_err() {
        let reg_signature = cardano::cip8_sign(&key_pair, &reg_message);
        if let Err(e) = api::register_address(
            &context.client,
            &context.api_url,
            &mining_address,
            &reg_message,
            &reg_signature.0,
            &hex::encode(key_pair.1.as_ref()),
        ) {
            let error_str = e.to_string();
            if !error_str.contains("400") && !error_str.contains("Bad Request") {
                eprintln!("✗ [{}] Registration failed: {}", wallet.name, e);
                return;
            }
        }
    }

    // Save challenge (silently)
    if let Some(ref base_dir) = context.data_dir {
        let _ = data_dir.save_challenge(base_dir, &challenge_params);
    }

    println!("⚡ [{}] Mining with {} threads...", wallet.name, context.threads);

    // Run mining cycle (suppress progress bar for cleaner output)
    let (result, total_hashes, elapsed_secs) = run_single_mining_cycle(
        mining_address.clone(),
        context.threads,
        context.donate_to_option.as_ref(),
        &challenge_params,
        context.data_dir.as_deref(),
        None, // No stop signal for sequential mining
    );

    match result {
        MiningResult::FoundAndQueued => {
            let hash_rate = if elapsed_secs > 0.0 { total_hashes as f64 / elapsed_secs } else { 0.0 };
            println!("✓ [{}] Solution found! ({:.0} H/s, {:.1}s)", wallet.name, hash_rate, elapsed_secs);

            // Handle donation if specified
            if let Some(ref destination_address) = context.donate_to_option {
                let donation_message = format!("Assign accumulated Scavenger rights to: {}", destination_address);
                let donation_signature = cardano::cip8_sign(&key_pair, &donation_message);
                let _ = api::donate_to(&context.client, &context.api_url, &mining_address, destination_address, &donation_signature.0);
            }
        }
        MiningResult::AlreadySolved => {
            println!("✓ [{}] Already solved", wallet.name);
        }
        MiningResult::MiningFailed => {
            println!("✗ [{}] Mining failed", wallet.name);
        }
    }
}
