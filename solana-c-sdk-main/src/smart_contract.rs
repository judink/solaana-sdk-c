use borsh::{BorshDeserialize, BorshSerialize};
use sha2::{Digest, Sha256};
use solana_client::rpc_client::RpcClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    signer::EncodableKey,
    system_program,
    transaction::Transaction,
};
use spl_associated_token_account::tools::account;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::str::FromStr;

use crate::{
    client::SolClient,
    wallet::{SolKeyPair, SolPublicKey},
};

// ==================== Utility Functions ==================== //

#[no_mangle]
pub extern "C" fn get_system_program_id() -> SolPublicKey {
    SolPublicKey {
        data: system_program::ID.to_bytes(),
    }
}

#[no_mangle]
pub extern "C" fn get_account_data_c(
    client: *mut SolClient,
    account_pubkey: *mut SolPublicKey,
    data_ptr: *mut u8,
    data_len: usize,
    data_offset: usize, // Offset for skipping metadata/discriminator
) -> usize {
    let client = unsafe { &mut *client };
    let pubkey = Pubkey::new_from_array(unsafe { (*account_pubkey).data });

    // Fetch account data from Solana
    match client.rpc_client.get_account(&pubkey) {
        Ok(account) => {
            let account_data = &account.data;
            if account_data.len() <= data_offset {
                eprintln!("❌ Account data too small.");
                return 0;
            }

            let data_slice = &account_data[data_offset..];
            let copy_len = std::cmp::min(data_len, data_slice.len());

            // Copy data into provided buffer
            unsafe {
                std::ptr::copy_nonoverlapping(data_slice.as_ptr(), data_ptr, copy_len);
            }

            println!("✅ Account data fetched ({} bytes).", copy_len);
            copy_len
        }
        Err(err) => {
            eprintln!("❌ Failed to fetch account: {:?}", err);
            0
        }
    }
}

// Load Payer Keypair

// Compute Discriminator
fn get_discriminator(method_name: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{}", method_name).as_bytes());
    hasher.finalize()[..8].to_vec()
}

// Create Instruction for Anchor Methods
fn create_instruction(
    program_id: &str,
    method_name: &str,
    accounts: Vec<AccountMeta>,
    data: Vec<u8>,
) -> Instruction {
    let discriminator = get_discriminator(method_name);
    let mut instruction_data = discriminator;
    instruction_data.extend(data);
    let program_id = Pubkey::from_str(program_id).expect("Invalid program ID");

    Instruction::new_with_bytes(program_id, &instruction_data, accounts)
}

// ==================== Transaction Functions ==================== //

#[no_mangle]
pub extern "C" fn send_generic_transaction_c(
    client: *mut SolClient,
    program_id: *const c_char,
    method_name: *const c_char,
    account_pubkeys: *const SolPublicKey,
    account_count: usize,
    signers: *const *mut SolKeyPair, // List of signers
    signer_count: usize,
    data_ptr: *const u8,
    data_len: usize,
) -> *mut c_char {
    let client = unsafe { &mut *client };

    let program_id = unsafe { CStr::from_ptr(program_id).to_str().unwrap() };
    let method_name = unsafe { CStr::from_ptr(method_name).to_str().unwrap() };

    // Deserialize account pubkeys
    let mut accounts = unsafe {
        std::slice::from_raw_parts(account_pubkeys, account_count)
            .iter()
            .map(|a| AccountMeta::new(a.to_pubkey(), false)) // Default signer = false
            .collect::<Vec<AccountMeta>>()
    };

    // Process signers
    let signer_refs = unsafe {
        std::slice::from_raw_parts(signers, signer_count)
            .iter()
            .map(|s| unsafe { &**s }) // Dereference raw pointers to SolKeyPair
            .collect::<Vec<&SolKeyPair>>()
    };

    let payer = signer_refs
        .first()
        .expect("At least one signer (payer) required"); // Ensure the first signer is the payer

    // Mark signer accounts as signers
    for signer in &signer_refs {
        if let Some(account) = accounts
            .iter_mut()
            .find(|acc| acc.pubkey == signer.get_pubkey())
        {
            account.is_signer = true;
        }
    }

    // Deserialize additional data if provided
    let data = if data_ptr.is_null() {
        vec![]
    } else {
        unsafe { std::slice::from_raw_parts(data_ptr, data_len).to_vec() }
    };

    // Create the transaction instruction
    let instruction = create_instruction(program_id, method_name, accounts, data);

    // Get latest blockhash
    let blockhash = match client.rpc_client.get_latest_blockhash() {
        Ok(bh) => bh,
        Err(e) => {
            eprintln!("Failed to get blockhash: {:?}", e);
            return CString::new("Failed to fetch blockhash")
                .unwrap()
                .into_raw();
        }
    };

    // Convert signers to Keypair list
    let signer_keypairs: Vec<Keypair> = signer_refs.iter().map(|s| s.to_keypair()).collect();

    let signer_refs: Vec<&Keypair> = signer_keypairs.iter().collect();

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.get_pubkey()), // Payer must be the first signer
        &signer_refs,
        blockhash,
    );

    let result = client.rpc_client.send_and_confirm_transaction(&transaction);

    match result {
        Ok(sig) => CString::new(sig.to_string()).unwrap().into_raw(),
        Err(err) => CString::new(format!("Transaction failed: {:?}", err))
            .unwrap()
            .into_raw(),
    }
}

// Initialize Account
#[no_mangle]
pub extern "C" fn initialize_account_c(
    client: *mut SolClient,
    payer: *mut SolKeyPair,
    account: *mut SolKeyPair,
    program_id: *const c_char,
) {
    let client = unsafe { &mut *client };
    let payer = unsafe { &mut *payer };

    let program_id = unsafe { CStr::from_ptr(program_id).to_str().unwrap() };
    let account = &unsafe { &mut *account }.to_keypair();

    let instruction = create_instruction(
        program_id,
        "initialize",
        vec![
            AccountMeta::new(account.pubkey(), true),
            AccountMeta::new(payer.to_keypair().pubkey(), true),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        vec![],
    );

    let blockhash = match client.rpc_client.get_latest_blockhash() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to fetch latest blockhash: {:?}", e);
            return;
        }
    };

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.to_keypair().pubkey()),
        &[&payer.to_keypair(), &account],
        blockhash,
    );

    match client.rpc_client.send_and_confirm_transaction(&transaction) {
        Ok(sig) => {
            println!("✅ Account initialized: {}", account.pubkey());
            println!("Transaction Signature: {}", sig);
        }
        Err(err) => {
            eprintln!("❌ Failed to initialize account: {:?}", err);
        }
    }
}

// ==================== Free Memory ==================== //
#[no_mangle]
pub extern "C" fn free_client(client: *mut SolClient) {
    if client.is_null() {
        return;
    }
    unsafe {
        Box::from_raw(client);
    }
}

#[no_mangle]
pub extern "C" fn free_payer(payer: *mut SolKeyPair) {
    if payer.is_null() {
        return;
    }
    unsafe {
        Box::from_raw(payer);
    }
}
