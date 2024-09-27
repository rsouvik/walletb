use bitcoin::util::bip32::{DerivationPath, ExtendedPubKey};
use bitcoin::network::constants::Network;
use bitcoin::secp256k1::Secp256k1;
use bitcoin::Address;
use reqwest::Client;
use serde::Deserialize;
use std::error::Error;
use structopt::StructOpt;
use tokio;
use base58::{FromBase58, FromBase58Error};  // Base58 decoding

// Command-line argument parsing
#[derive(StructOpt)]
struct Cli {
    /// The extended public key (xpub) in base58 format
    xpub: String,
    /// Number of derived addresses to query for balance
    #[structopt(default_value = "20")]
    count: u32,
}

#[derive(Deserialize)]
struct ApiResponse {
    confirmed: u64,
}

//https://blockstream.info/testnet/api/address/${address}/utxo
async fn get_balance(client: &Client, address: &str) -> Result<u64, Box<dyn Error>> {
    let url = format!("https://blockstream.info/testnet/api/address/{}/utxo", address);
    let response = client.get(&url).send().await?.json::<Vec<ApiResponse>>().await?;

    // Sum confirmed balances
    let total_balance: u64 = response.iter().map(|utxo| utxo.confirmed).sum();

    Ok(total_balance)
}

fn decode_xpub(xpub: &str) -> Result<Vec<u8>, anyhow::Error> {
    let decoded_xpub = xpub
        .from_base58()
        .map_err(|e: FromBase58Error| anyhow::Error::msg(format!("{:?}", e)))?;
    Ok(decoded_xpub)
}

fn derive_addresses(xpub: &str, count: u32) -> Result<Vec<String>, Box<dyn Error>> {
    let secp = Secp256k1::new();

    // Decode the xpub key
    let decoded_xpub = decode_xpub(xpub)?;

    let xpub = ExtendedPubKey::decode(&decoded_xpub)?;

    let mut addresses = Vec::new();

    for i in 0..count {
        // Build the derivation path
        let path = build_derivation_path(i);
        let child_pubkey = xpub.derive_pub(&secp, &path)?;

        // Convert to Bitcoin address (P2PKH format)
        let address = Address::p2pkh(&child_pubkey.public_key, Network::Testnet);
        addresses.push(address.to_string());
    }

    Ok(addresses)
}

fn build_derivation_path(index: u32) -> DerivationPath {
    let path = format!("m/0/{}", index);
    path.parse().expect("Invalid derivation path")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::from_args();
    println!("Fetching balances for xpub: {}", args.xpub);

    let addresses = derive_addresses(&args.xpub, args.count)?;

    let client = Client::new();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_xpub_valid() {
        let valid_xpub = "xpub661MyMwAqRbcFxWNRRv6HoGmRuFZ2a43FAPX1YHgSoXQQFF4MumH9Sx5ecxa9GZcEqBeRBxHLXa5xnupTg6FpjoowHmg69vKwZYjt5mx5zt";
        let decoded = decode_xpub(valid_xpub);
        assert!(decoded.is_ok(), "Decoding should succeed for a valid xpub");
    }

    #[test]
    fn test_decode_xpub_invalid() {
        let invalid_xpub = "invalidkey";
        let decoded = decode_xpub(invalid_xpub);
        assert!(decoded.is_err(), "Decoding should fail for an invalid xpub");
    }

    #[test]
    fn test_derive_addresses() {
        let valid_xpub = "xpub661MyMwAqRbcFxWNRRv6HoGmRuFZ2a43FAPX1YHgSoXQQFF4MumH9Sx5ecxa9GZcEqBeRBxHLXa5xnupTg6FpjoowHmg69vKwZYjt5mx5zt";
        let addresses = derive_addresses(valid_xpub, 5);
        assert!(addresses.is_ok(), "Address derivation should succeed for a valid xpub");
        assert_eq!(addresses.unwrap().len(), 5, "5 addresses should be derived");
    }

    #[test]
    fn test_build_derivation_path() {
        let path = build_derivation_path(0);
        assert_eq!(path.to_string(), "m/0/0", "Path for index 0 should be m/0/0");
    }
}
