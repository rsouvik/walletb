use bitcoin::util::bip32::{DerivationPath, ExtendedPubKey};
use bitcoin::network::constants::Network;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Address;
use reqwest::Client;
use serde::Deserialize;
use std::error::Error;
use structopt::StructOpt;
use tokio;
use base58::FromBase58;  // You'll need to add this crate to handle base58 decoding

// Command-line argument parsing
#[derive(StructOpt)]
struct Cli {
    /// The extended public key (xpub) in base58 format
    xpub: String,
    /// Number of derived addresses to query for balance
    #[structopt(default_value = "20")]
    count: u32,
}

// API response structure for balance
#[derive(Deserialize)]
struct ApiResponse {
    confirmed: u64,
}

// Function to fetch balance of a single address
async fn get_balance(client: &Client, address: &str) -> Result<u64, Box<dyn Error>> {
    let url = format!("https://blockstream.info/api/address/{}/utxo", address);
    let response = client.get(&url).send().await?.json::<Vec<ApiResponse>>().await?;

    // Sum confirmed balances
    let total_balance: u64 = response.iter().map(|utxo| utxo.confirmed).sum();

    Ok(total_balance)
}

// Function to derive Bitcoin addresses from xpub
fn derive_addresses(xpub: &str, count: u32) -> Result<Vec<String>, Box<dyn Error>> {
    let secp = Secp256k1::new();

    // Decode base58 xpub
    let decoded_xpub = xpub.from_base58()?;

    // Use ExtendedPubKey::decode to create an extended public key from bytes
    let xpub = ExtendedPubKey::decode(&decoded_xpub)?;

    let mut addresses = Vec::new();

    for i in 0..count {
        // Derive the address using the derivation path m/0/i
        let derivation_path = format!("m/0/{}", i);
        let path = DerivationPath::from_str(&derivation_path)?;
        let child_pubkey = xpub.derive_pub(&secp, &path)?;

        // Convert to Bitcoin address (P2PKH format)
        let address = Address::p2pkh(&child_pubkey.public_key, Network::Bitcoin);
        addresses.push(address.to_string());
    }

    Ok(addresses)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::from_args();
    println!("Fetching balances for xpub: {}", args.xpub);

    // Derive addresses
    let addresses = derive_addresses(&args.xpub, args.count)?;

    // Initialize Reqwest client
    let client = Client::new();

    // Total balance variable
    let mut total_balance: u64 = 0;

    // Fetch and display balance for each address
    for (i, address) in addresses.iter().enumerate() {
        let balance = get_balance(&client, address).await?;
        println!("Address {}: {} satoshis", i + 1, balance);
        total_balance += balance;
    }

    // Display total balance
    println!("Total balance: {} satoshis", total_balance);

    Ok(())
}