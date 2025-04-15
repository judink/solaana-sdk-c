

use solana_account_decoder::UiAccountData;

use solana_client::{
    rpc_request::TokenAccountsFilter,
    rpc_client::RpcClient, // RpcClient 임포트 확인
    rpc_config::RpcTransactionConfig, // 설정 추가
};
use solana_sdk::{
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
    signer::Signer,
    transaction::Transaction,
    commitment_config::CommitmentConfig, // 커밋먼트 설정 추가
    signature::{Signature, ParseSignatureError}, // Signature와 ParseSignatureError를 여기서 한 번에 가져옴
};
use spl_associated_token_account;
use spl_token::state::Mint; // Add this line to import the module
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding}; // 트랜잭션 정보 타입
use std::{
    ffi::{c_char, CString, NulError},
    str::FromStr, // FromStr 트레잇 사용
};

use crate::wallet::SolKeyPair;
use crate::{client::SolClient, wallet::SolPublicKey};

#[repr(C)]
pub struct SolMint {
    pub mint_authority: *mut SolPublicKey,
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority: *mut SolPublicKey,
}

#[repr(C)]
pub struct TokenInfo {
    mint: *const c_char,    // Token mint as a string
    balance: *const c_char, // Token balance as a string
    owner: *const c_char,   // Token owner as a string
}

#[repr(C)]
pub struct TokenList {
    data: *mut TokenInfo, // Pointer to an array of `TokenInfo`
    len: usize,           // Length of the array
}

#[no_mangle]
pub extern "C" fn get_all_tokens(
    client: *mut SolClient,
    wallet: *mut SolPublicKey,
) -> *mut TokenList {
    // Safety: Ensure pointers are not null
    let client = unsafe {
        assert!(!client.is_null());
        &*client
    };

    let wallet = unsafe {
        assert!(!wallet.is_null());
        &*wallet
    };

    let wallet_pubkey = Pubkey::new_from_array(wallet.data);

    // Fetch all token accounts owned by the wallet
    let token_accounts = match client.rpc_client.get_token_accounts_by_owner(
        &wallet_pubkey,
        TokenAccountsFilter::ProgramId(spl_token::id()),
    ) {
        Ok(accounts) => accounts,
        Err(err) => {
            eprintln!(
                "Error fetching token accounts for wallet {}: {:?}",
                wallet_pubkey, err
            );
            return std::ptr::null_mut();
        }
    };

    let mut tokens: Vec<TokenInfo> = Vec::new();

    for keyed_account in token_accounts {
        if let UiAccountData::Json(parsed_data) = keyed_account.account.data {
            if let Some(info) = parsed_data.parsed.get("info").and_then(|v| v.as_object()) {
                if let (Some(mint), Some(balance), Some(owner)) = (
                    info.get("mint").and_then(|v| v.as_str()),
                    info.get("tokenAmount")
                        .and_then(|v| v.get("uiAmountString"))
                        .and_then(|v| v.as_str()),
                    info.get("owner").and_then(|v| v.as_str()),
                ) {
                    let mint_c = CString::new(mint).unwrap();
                    let balance_c = CString::new(balance).unwrap();
                    let owner_c = CString::new(owner).unwrap();

                    tokens.push(TokenInfo {
                        mint: mint_c.into_raw(),
                        balance: balance_c.into_raw(),
                        owner: owner_c.into_raw(),
                    });
                }
            }
        } else {
            eprintln!(
                "Unexpected account data format for account: {}",
                keyed_account.pubkey
            );
        }
    }

    let token_list = Box::new(TokenList {
        data: tokens.as_mut_ptr(),
        len: tokens.len(),
    });

    std::mem::forget(tokens); // Prevent Rust from deallocating the vector
    Box::into_raw(token_list) // Pass ownership to C
}

#[no_mangle]
pub extern "C" fn token_list_get_data(list: *const TokenList) -> *mut TokenInfo {
    if list.is_null() {
        std::ptr::null_mut()
    } else {
        unsafe { (*list).data }
    }
}

// --- 새로운 함수: 트랜잭션 상세 정보 조회 (JSON 반환) ---
#[no_mangle]
pub extern "C" fn get_transaction_details_json(
    client: *mut SolClient,
    signature_str: *const c_char,
) -> *mut c_char {
    // Safety: 포인터 유효성 검사
    let client = unsafe {
        if client.is_null() {
            eprintln!("Error: SolClient pointer is null in get_transaction_details_json");
            return std::ptr::null_mut();
        }
        &*client
    };
    let signature_cstr = unsafe {
        if signature_str.is_null() {
            eprintln!("Error: Signature string pointer is null in get_transaction_details_json");
            return std::ptr::null_mut();
        }
        std::ffi::CStr::from_ptr(signature_str)
    };

    // C 문자열을 Rust 문자열로 변환
    let signature_slice = match signature_cstr.to_str() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error converting signature C string to Rust string: {}", e);
            return std::ptr::null_mut();
        }
    };

    // 문자열을 Signature 객체로 파싱
    let signature = match Signature::from_str(signature_slice) {
        Ok(sig) => sig,
        Err(e) => {
            eprintln!("Error parsing signature string '{}': {}", signature_slice, e);
            return std::ptr::null_mut();
        }
    };

    // RPC 호출 설정 (JsonParsed 인코딩 사용 필수)
    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::JsonParsed),
        commitment: Some(CommitmentConfig::confirmed()), // 또는 finalized()
        max_supported_transaction_version: Some(0), // 최신 버전 지원
    };

    // RPC 호출: get_transaction
    match client.rpc_client.get_transaction_with_config(&signature, config) {
        Ok(tx_with_meta) => {
            // 트랜잭션 정보를 JSON 문자열로 직렬화
            match serde_json::to_string(&tx_with_meta) {
                Ok(json_string) => {
                    // JSON 문자열을 C 문자열 포인터로 변환하여 반환
                    match CString::new(json_string) {
                        Ok(c_json) => c_json.into_raw(),
                        Err(e) => {
                            eprintln!("Error converting JSON string to CString: {}", e);
                            std::ptr::null_mut()
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error serializing transaction details to JSON: {}", e);
                    std::ptr::null_mut()
                }
            }
        }
        Err(e) => {
            eprintln!("Error fetching transaction details for signature {}: {}", signature, e);
            std::ptr::null_mut()
        }
    }
}

// --- 새로운 함수: JSON 문자열 메모리 해제 ---
#[no_mangle]
pub extern "C" fn free_transaction_details_json(json_ptr: *mut c_char) {
    if !json_ptr.is_null() {
        unsafe {
            // CString::from_raw을 호출하여 Rust가 메모리 소유권을 다시 가져오고,
            // _guard 변수가 범위를 벗어날 때 자동으로 메모리가 해제됩니다.
            let _guard = CString::from_raw(json_ptr);
        }
    }
}

#[no_mangle]
pub extern "C" fn token_list_get_len(list: *const TokenList) -> usize {
    if list.is_null() {
        0
    } else {
        unsafe { (*list).len }
    }
}

#[no_mangle]
pub extern "C" fn free_token_list(list: *mut TokenList) {
    if list.is_null() {
        return;
    }

    unsafe {
        let list = Box::from_raw(list);
        for i in 0..list.len {
            let token_info = &mut *list.data.add(i);
            CString::from_raw(token_info.mint as *mut c_char);
            CString::from_raw(token_info.balance as *mut c_char);
            CString::from_raw(token_info.owner as *mut c_char);
        }
        Vec::from_raw_parts(list.data, list.len, list.len);
    }
}

#[no_mangle]
pub extern "C" fn transfer_sol(
    client: *mut SolClient,
    sender: *mut SolKeyPair,
    recipient: *mut SolPublicKey,
    lamports: u64,
) -> *mut c_char { // Return type changed to *mut c_char
    // Safety: Ensure pointers are not null
    let client = unsafe {
        assert!(!client.is_null());
        &*client
    };

    let sender = unsafe {
        assert!(!sender.is_null());
        &*sender
    };

    let recipient = unsafe {
        assert!(!recipient.is_null());
        &*recipient
    };

    let sender_keypair = sender.to_keypair(); // Get Keypair
    let sender_pubkey = sender_keypair.pubkey(); // Get Pubkey from Keypair
    let recipient_pubkey = Pubkey::new_from_array(recipient.data);

    // Verify that the sender's account exists
    // match client.rpc_client.get_account(&sender_pubkey) {
    //     Ok(account) => {
    //         if account.lamports < lamports {
    //             eprintln!(
    //                 "Sender does not have enough SOL. Balance: {}, Required: {}",
    //                 account.lamports, lamports
    //             );
    //             return false;
    //         }
    //     }
    //     Err(err) => {
    //         eprintln!("Error checking sender's account: {:?}", err);
    //         return false;
    //     }
    // }

    // Step 1: Create the transfer instruction
    let transfer_instruction =
        solana_sdk::system_instruction::transfer(&sender_pubkey, &recipient_pubkey, lamports);

    // Step 2: Fetch the recent blockhash
    let recent_blockhash = match client.rpc_client.get_latest_blockhash() {
        Ok(blockhash) => blockhash,
        Err(err) => {
            eprintln!("Error fetching latest blockhash: {:?}", err);
            return std::ptr::null_mut(); // Return null on error
        }
    };

    // Step 3: Create and sign the transaction
    // Note: No longer needs to be mutable if you don't modify it after creation
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&sender_pubkey),    // Fee payer
        &[&sender_keypair],      // Signer (pass the actual Keypair)
        recent_blockhash,
    );

    // Step 4: Send and confirm the transaction (using send_and_confirm for simplicity)
    match client.rpc_client.send_and_confirm_transaction_with_spinner(&transaction) {
        Ok(signature) => { // Capture the signature here
            println!(
                "Successfully transferred {} lamports from {} to {}. Signature: {}",
                lamports, sender_pubkey, recipient_pubkey, signature
            );
            // Convert signature to CString and return pointer
            match CString::new(signature.to_string()) {
                Ok(c_string) => c_string.into_raw(), // Transfer ownership to C
                Err(e) => {
                    eprintln!("Error converting signature to CString: {}", e);
                    std::ptr::null_mut() // Return null if conversion fails
                }
            }
        }
        Err(err) => {
            eprintln!("Error sending and confirming transaction: {:?}", err);
            std::ptr::null_mut() // Return null on error
        }
    }
}

#[no_mangle]
pub extern "C" fn transfer_spl(
    client: *mut SolClient,
    sender: *mut SolKeyPair,
    recipient: *mut SolPublicKey,
    mint: *mut SolPublicKey,
    amount: u64,
) -> *mut c_char { // Return type changed to *mut c_char
    // Safety checks...
    // Safety: Ensure pointers are not null
    let client = unsafe {
        assert!(!client.is_null());
        &*client
    };

    let sender = unsafe {
        assert!(!sender.is_null());
        &*sender
    };

    let recipient = unsafe {
        assert!(!recipient.is_null());
        &*recipient
    };

    let mint = unsafe {
        assert!(!mint.is_null());
        &*mint
    };

    let sender_keypair = sender.to_keypair(); // Get the keypair once
    let sender_pubkey = sender_keypair.pubkey();
    let recipient_pubkey = Pubkey::new_from_array(recipient.data);
    let mint_pubkey = mint.to_pubkey(); // Assuming to_pubkey() exists

    // Step 1 & 2: Get or create recipient's associated token account & derive sender's ATA
    let recipient_assoc = match _get_or_create_associated_token_account(
        client,
        sender, // Pass the original SolKeyPair pointer
        &recipient_pubkey,
        &mint_pubkey,
    ) {
        Ok(assoc) => assoc,
        Err(err) => {
            eprintln!("Error managing recipient's associated token account: {}", err);
            return std::ptr::null_mut(); // Return null on error
        }
    };
    let sender_assoc =
        spl_associated_token_account::get_associated_token_address(&sender_pubkey, &mint_pubkey);

    // Step 3: Create the transfer instruction
    let transfer_instruction = match spl_token::instruction::transfer(
        &spl_token::id(),
        &sender_assoc,
        &recipient_assoc,
        &sender_pubkey, // Authority is the owner of the sender wallet
        &[&sender_pubkey], // Signer is the owner as well
        amount,
    ) {
        Ok(instruction) => instruction,
        Err(err) => {
            eprintln!("Error creating transfer instruction: {:?}", err);
            return std::ptr::null_mut(); // Return null on error
        }
    };

    // Step 4: Fetch the recent blockhash
    let recent_blockhash = match client.rpc_client.get_latest_blockhash() {
        Ok(blockhash) => blockhash,
        Err(err) => {
            eprintln!("Error fetching latest blockhash: {:?}", err);
            return std::ptr::null_mut(); // Return null on error
        }
    };

    // Step 5: Create and sign the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[transfer_instruction],
        Some(&sender_pubkey),    // Fee payer
        &[&sender_keypair],      // Pass the actual Keypair for signing
        recent_blockhash,
    );

    // Step 6: Send and confirm the transaction (Using send_and_confirm for simplicity)
    match client.rpc_client.send_and_confirm_transaction_with_spinner(&transaction) {
        Ok(signature) => {
            println!(
                "Successfully transferred {} tokens from {} to {}. Signature: {}",
                amount, sender_assoc, recipient_assoc, signature
            );
            // Convert signature to CString and return pointer
            match CString::new(signature.to_string()) {
                Ok(c_string) => c_string.into_raw(), // Transfer ownership to C
                Err(e) => {
                    eprintln!("Error converting signature to CString: {}", e);
                    std::ptr::null_mut() // Return null if conversion fails
                }
            }
        }
        Err(err) => {
            eprintln!("Error sending and confirming transaction: {:?}", err);
            std::ptr::null_mut() // Return null on error
        }
    }
}

// Function to free the memory allocated for the transaction signature string
#[no_mangle]
pub extern "C" fn free_transaction_signature(signature: *mut c_char) {
    if !signature.is_null() {
        unsafe {
            // Takes ownership back from C and frees the memory when `_cstring` goes out of scope
            let _cstring = CString::from_raw(signature);
        }
    }
}

#[no_mangle]
pub extern "C" fn create_spl_token(
    client: *mut SolClient,
    payer: *mut SolKeyPair,
    mint: *mut SolKeyPair,
) -> bool {
    // Safety: Ensure the client pointer is not null
    let client = unsafe {
        assert!(!client.is_null());
        &*client
    };

    let payer = unsafe {
        assert!(!payer.is_null());
        &*payer
    };

    let mint = unsafe {
        assert!(!mint.is_null());
        &*mint
    };

    let minimum_balance_for_rent_exemption = match client
        .rpc_client
        .get_minimum_balance_for_rent_exemption(Mint::LEN)
    {
        Ok(balance) => balance,
        Err(err) => {
            eprintln!(
                "Error getting minimum balance for rent exemption: {:?}",
                err
            );
            return false;
        }
    };

    let create_account_instruction: Instruction = solana_sdk::system_instruction::create_account(
        &&payer.to_keypair().pubkey(),
        &mint.to_keypair().pubkey(),
        minimum_balance_for_rent_exemption,
        Mint::LEN as u64,
        &spl_token::ID,
    );

    // Create the mint instruction
    let mint_instruction = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &mint.to_keypair().pubkey(),
        &mint.to_keypair().pubkey(),
        None,
        9, // Decimals
    );

    let mint_instruction = match mint_instruction {
        Ok(instruction) => instruction,
        Err(err) => {
            eprintln!("Error creating mint instruction: {:?}", err);
            return false;
        }
    };

    let recent_blockhash = match client.rpc_client.get_latest_blockhash() {
        Ok(blockhash) => blockhash,
        Err(err) => {
            eprintln!("Error getting latest blockhash: {:?}", err);
            return false;
        }
    };

    // Create and sign the transaction
    let mut transaction = Transaction::new_signed_with_payer(
        &[create_account_instruction, mint_instruction],
        Some(&payer.to_keypair().pubkey()),
        &[&mint.to_keypair(), &payer.to_keypair()],
        recent_blockhash,
    );

    // Send the transaction
    match client.rpc_client.send_transaction(&transaction) {
        Ok(_) => true,
        Err(err) => {
            eprintln!("Error sending and confirming transaction: {:?}", err);
            return false;
        }
    }
}

#[no_mangle]
pub extern "C" fn get_mint_info(
    client: *mut SolClient,
    mint_pubkey: *mut SolPublicKey,
) -> *mut SolMint {
    // Safety: Ensure the client pointer is not null
    let client = unsafe {
        assert!(!client.is_null());
        &*client
    };

    let mint = unsafe {
        assert!(!mint_pubkey.is_null());
        &*mint_pubkey
    };

    let mint_pubkey = mint.to_pubkey();
    let mint_info = match client.rpc_client.get_account_data(&mint_pubkey) {
        Ok(data) => data,
        Err(_) => return std::ptr::null_mut(),
    };

    let mint_info = match Mint::unpack(&mint_info) {
        Ok(mint_info) => mint_info,
        Err(_) => return std::ptr::null_mut(),
    };

    let mint_authority = SolPublicKey {
        data: mint_info
            .mint_authority
            .map_or([0u8; 32], |pubkey| pubkey.to_bytes()),
    };

    let freeze_authority = SolPublicKey {
        data: mint_info.freeze_authority.unwrap_or_default().to_bytes(),
    };

    let sol_mint = SolMint {
        mint_authority: Box::into_raw(Box::new(mint_authority)),
        supply: mint_info.supply,
        decimals: mint_info.decimals,
        is_initialized: mint_info.is_initialized,
        freeze_authority: Box::into_raw(Box::new(freeze_authority)),
    };

    Box::into_raw(Box::new(sol_mint))
}

#[no_mangle]
pub extern "C" fn get_or_create_associated_token_account(
    client: *mut SolClient,
    payer: *mut SolKeyPair,
    owner: *mut SolPublicKey,
    mint: *mut SolKeyPair,
) -> *mut SolPublicKey {
    // Safety: Ensure pointers are not null
    let client = unsafe {
        assert!(!client.is_null());
        &*client
    };
    let payer = unsafe {
        assert!(!payer.is_null());
        &*payer
    };
    let owner = unsafe {
        assert!(!owner.is_null());
        &*owner
    };
    let mint = unsafe {
        assert!(!mint.is_null());
        &*mint
    };

    // Extract public keys
    let owner_pubkey = Pubkey::new_from_array(owner.data);
    let mint_pubkey = mint.to_keypair().pubkey();

    // Call the helper function to get or create the associated token account
    match _get_or_create_associated_token_account(client, payer, &owner_pubkey, &mint_pubkey) {
        Ok(assoc) => Box::into_raw(Box::new(SolPublicKey {
            data: assoc.to_bytes(),
        })),
        Err(err) => {
            eprintln!("Error managing associated token account: {}", err);
            std::ptr::null_mut()
        }
    }
}

pub fn _get_or_create_associated_token_account(
    client: &SolClient,
    payer: &SolKeyPair,
    recipient_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
) -> Result<Pubkey, String> {
    let assoc =
        spl_associated_token_account::get_associated_token_address(recipient_pubkey, mint_pubkey);

    match client.rpc_client.get_account(&assoc) {
        Ok(account) => {
            // Associated token account exists
            println!("Associated token account already exists at: {}", assoc);
            Ok(assoc)
        }
        Err(ref err)
            if matches!(
                err.kind(),
                solana_client::client_error::ClientErrorKind::RpcError(_)
            ) =>
        {
            // Create the associated token account
            println!("Associated token account does not exist. Proceeding to create...");
            let assoc_instruction =
                spl_associated_token_account::instruction::create_associated_token_account(
                    &payer.to_keypair().pubkey(),
                    recipient_pubkey,
                    mint_pubkey,
                    &spl_token::id(),
                );

            let recent_blockhash = client
                .rpc_client
                .get_latest_blockhash()
                .map_err(|err| format!("Error fetching latest blockhash: {:?}", err))?;

            let mut assoc_transaction = Transaction::new_signed_with_payer(
                &[assoc_instruction],
                Some(&payer.to_keypair().pubkey()),
                &[&payer.to_keypair()],
                recent_blockhash,
            );

            client
                .rpc_client
                .send_transaction(&assoc_transaction)
                .map_err(|err| format!("Error creating associated token account: {:?}", err))?;

            println!(
                "Associated token account created successfully at: {}",
                assoc
            );
            Ok(assoc)
        }
        Err(err) => Err(format!(
            "Unexpected error checking associated token account: {:?}",
            err
        )),
    }
}

#[no_mangle]
pub extern "C" fn mint_spl(
    client: *mut SolClient,
    payer: *mut SolKeyPair,
    mint_authority: *mut SolKeyPair,
    recipient: *mut SolPublicKey,
    amount: u64,
) -> bool {
    // Safety: Ensure pointers are not null
    let client = unsafe {
        assert!(!client.is_null());
        &*client
    };
    let payer = unsafe {
        assert!(!payer.is_null());
        &*payer
    };
    let mint_authority = unsafe {
        assert!(!mint_authority.is_null());
        &*mint_authority
    };
    let recipient = unsafe {
        assert!(!recipient.is_null());
        &*recipient
    };

    let mint_authority_pubkey = mint_authority.to_keypair().pubkey();
    let recipient_pubkey = Pubkey::new_from_array(recipient.data);

    // Get or create associated token account
    let assoc = match _get_or_create_associated_token_account(
        client,
        payer,
        &recipient_pubkey,
        &mint_authority_pubkey,
    ) {
        Ok(assoc) => assoc,
        Err(err) => {
            eprintln!("Error managing associated token account: {}", err);
            return false;
        }
    };

    // Step 3: Create the mint_to instruction
    let mint_instruction = match spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint_authority_pubkey,
        &assoc,
        &mint_authority.to_keypair().pubkey(),
        &[&mint_authority.to_keypair().pubkey()],
        amount,
    ) {
        Ok(instruction) => instruction,
        Err(err) => {
            eprintln!("Error creating mint instruction: {:?}", err);
            return false;
        }
    };

    // Step 4: Fetch the recent blockhash
    let recent_blockhash = match client.rpc_client.get_latest_blockhash() {
        Ok(blockhash) => blockhash,
        Err(err) => {
            eprintln!("Error fetching latest blockhash: {:?}", err);
            return false;
        }
    };

    // Step 5: Create and sign the mint transaction
    let mut transaction = Transaction::new_signed_with_payer(
        &[mint_instruction],
        Some(&payer.to_keypair().pubkey()), // Fee payer
        &[&mint_authority.to_keypair(), &payer.to_keypair()], // Required signers
        recent_blockhash,
    );

    // Step 6: Send and confirm the mint transaction
    match client.rpc_client.send_transaction(&transaction) {
        Ok(_) => {
            println!("Successfully minted {} tokens to {}", amount, assoc);
            true
        }
        Err(err) => {
            eprintln!("Error minting tokens: {:?}", err);
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn get_associated_token_balance(
    client: *mut SolClient,
    owner: *mut SolPublicKey,
    mint: *mut SolPublicKey,
) -> u64 {
    // Safety: Ensure the client pointer is not null
    let client = unsafe {
        assert!(!client.is_null());
        &*client
    };

    let owner = unsafe {
        assert!(!owner.is_null());
        &*owner
    };

    let mint = unsafe {
        assert!(!mint.is_null());
        &*mint
    };

    let owner_pubkey = owner.to_pubkey();
    let mint_pubkey = mint.to_pubkey();

    let assoc =
        spl_associated_token_account::get_associated_token_address(&owner_pubkey, &mint_pubkey);

    let balance = match client.rpc_client.get_token_account_balance(&assoc) {
        Ok(balance) => balance,
        Err(err) => {
            eprintln!("Error getting token account balance: {:?}", err);
            return 0;
        }
    };

    match balance.amount.parse::<u64>() {
        Ok(amount) => amount,
        Err(err) => {
            eprintln!("Error parsing token account balance: {:?}", err);
            0
        }
    }
}
