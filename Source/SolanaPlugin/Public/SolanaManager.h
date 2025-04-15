// Fill out your copyright notice in the Description page of Project Settings.

#pragma once

#include "CoreMinimal.h"
#include "solana_sdk.h"
#include "UObject/NoExportTypes.h"
#include "SolanaManager.generated.h"

USTRUCT(BlueprintType)
struct FTokenInfo
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "Solana")
	FString MintAddress; // Address of the token's mint

	UPROPERTY(BlueprintReadOnly, Category = "Solana")
	FString Balance; // Balance of the token

	UPROPERTY(BlueprintReadOnly, Category = "Solana")
	FString OwnerAddress; // Owner of the token

	FTokenInfo()
		: MintAddress(TEXT("")), Balance(TEXT("")), OwnerAddress(TEXT(""))
	{
	}
};

// Struct for Solana Public Key
USTRUCT(BlueprintType)
struct FSolanaPublicKey
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "Solana")
	TArray<uint8> Data; // 32-byte public key

	FSolanaPublicKey()
	{
		Data.SetNum(32); // Initialize to 32 bytes
	}
};

// Struct for Solana Key Pair
USTRUCT(BlueprintType)
struct FSolanaKeyPair
{
	GENERATED_BODY()

	UPROPERTY(BlueprintReadOnly, Category = "Solana")
	TArray<uint8> PrivateKey; // 64-byte private key

	UPROPERTY(BlueprintReadOnly, Category = "Solana")
	FSolanaPublicKey PublicKey; // Associated public key

	FSolanaKeyPair()
	{
		PrivateKey.SetNum(64); // Initialize to 64 bytes
	}
};

// Manager for Solana Blockchain Interactions
UCLASS(Blueprintable)
class SOLANAPLUGIN_API USolanaManager : public UObject
{
	GENERATED_BODY()

public:
	USolanaManager();
	~USolanaManager();

	// Static function to create an instance of SolanaManager
	UFUNCTION(BlueprintCallable, Category = "Solana")
	static USolanaManager *CreateSolanaManager();

	// Connect to Solana RPC
	UFUNCTION(BlueprintCallable, Category = "Solana")
	void ConnectToSolana(const FString &RpcUrl);

	// Create a new wallet
	UFUNCTION(BlueprintCallable, Category = "Solana")
	static FSolanaKeyPair CreateWallet();

	// Retrieve balance for a wallet
	UFUNCTION(BlueprintCallable, Category = "Solana")
	void GetBalance(const FSolanaPublicKey &PublicKey, FString &Balance, FString &ErrorMessage);

	// Display Solana Public Key
	UFUNCTION(BlueprintCallable, Category = "Solana")
	static FString DisplayPublicKey(const FSolanaPublicKey &PublicKey, const FString &Description);

	// Display Solana Key Pair
	UFUNCTION(BlueprintCallable, Category = "Solana")
	static FString DisplayKeyPair(const FSolanaKeyPair &KeyPair, const FString &Description);

	UFUNCTION(BlueprintCallable, Category = "Solana")
	static void ShowPopup(const FString &Title, const FString &Message);

	// Get Wallet Address from Solana Key Pair
	UFUNCTION(BlueprintCallable, Category = "Solana")
	static FString GetWalletAddress(const FSolanaKeyPair &KeyPair);

	// Create and save wallet to a file
	UFUNCTION(BlueprintCallable, Category = "Solana")
	static FSolanaKeyPair CreateAndSaveWallet(const FString &FilePath, FString &ErrorMessage);

	// Load a wallet from a file
	UFUNCTION(BlueprintCallable, Category = "Solana")
	static FSolanaKeyPair LoadWalletFromFile(const FString &FilePath, FString &ErrorMessage);

	// Request a faucet for a wallet
	UFUNCTION(BlueprintCallable, Category = "Solana")
	bool RequestFaucet(const FSolanaPublicKey &PublicKey, int64 Amount, FString &ErrorMessage);

	UFUNCTION(BlueprintCallable, Category = "Solana")
	static FSolanaPublicKey GetPublicKey(const FSolanaKeyPair &KeyPair);

	UFUNCTION(BlueprintCallable, Category = "Solana")
	bool CreateSPLToken(const FSolanaKeyPair &PayerKeyPair, const FSolanaKeyPair &MintKeyPair, FString &ErrorMessage);

	UFUNCTION(BlueprintCallable, Category = "Solana")
	bool GetMintInfo(const FSolanaPublicKey &MintPublicKey, FString &MintAuthority, int64 &Supply, int32 &Decimals, bool &IsInitialized, FString &FreezeAuthority, FString &ErrorMessage);

	UFUNCTION(BlueprintCallable, Category = "Solana")
	bool MintSPL(const FSolanaKeyPair &PayerKeyPair, const FSolanaKeyPair &MintAuthorityKeyPair, const FSolanaPublicKey &RecipientPublicKey, int64 Amount, FString &ErrorMessage);

	UFUNCTION(BlueprintCallable, Category = "Solana")
	bool GetAssociatedTokenBalance(const FSolanaPublicKey &OwnerPublicKey, const FSolanaPublicKey &MintPublicKey, int64 &Balance, FString &ErrorMessage);

	UFUNCTION(BlueprintCallable, Category = "Solana|SPL Token", meta = (DisplayName = "Transfer SPL Token"))
	FString TransferSPL(
		const FSolanaKeyPair& SenderKeyPair,
		const FSolanaPublicKey& RecipientPublicKey,
		const FSolanaPublicKey& MintPublicKey,
		int64 Amount,
		UPARAM(DisplayName="Error Message") FString& ErrorMessage // Output parameter for errors
	);
	
	UFUNCTION(BlueprintCallable, Category = "Solana|Native Token", meta = (DisplayName = "Transfer SOL")) // Category updated for clarity
	FString TransferSOL( // Return type changed to FString
		const FSolanaKeyPair& SenderKeyPair,
		const FSolanaPublicKey& RecipientPublicKey,
		int64 Amount, // Amount in lamports
		UPARAM(DisplayName="Error Message") FString& ErrorMessage // Output parameter for errors
	);

	UFUNCTION(BlueprintCallable, Category = "Solana")
	TArray<FTokenInfo> GetAllTokens(const FSolanaPublicKey &WalletPublicKey, FString &ErrorMessage);

	UFUNCTION(BlueprintCallable, Category = "Solana")
	static void ShowTokensInPopup(const TArray<FTokenInfo> &Tokens, const FString &Title);

	// Convert address (string) to public key
	UFUNCTION(BlueprintCallable, Category = "Solana")
	static FSolanaPublicKey AddressToPublicKey(const FString &Address, FString &ErrorMessage);

	// Convert public key to address (string)
	UFUNCTION(BlueprintCallable, Category = "Solana")
	static FString PublicKeyToAddress(const FSolanaPublicKey &PublicKey, FString &ErrorMessage);

	UFUNCTION(BlueprintPure, Category = "Solana|Wallet")
	static bool IsValidKeyPair(const FSolanaKeyPair& KeyPair);
	
	UFUNCTION(BlueprintCallable, Category = "Solana|Transaction", meta=(DisplayName="Get Transaction Details as JSON"))
	FString GetTransactionDetailsAsJson(const FString& TransactionSignature, UPARAM(DisplayName="Error Message") FString& ErrorMessage);
	

private:
	SolClient *SolanaClient; // Pointer to Solana client
};