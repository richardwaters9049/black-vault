//! JSON bridge for cross-language tests. Never use with real secrets.
use base64::{engine::general_purpose::STANDARD, Engine};
use black_vault_api::crypto::{
    derivation::{derive_key, DerivationParams},
    encryption::{decrypt, encrypt, EncryptedData},
};
use serde::Deserialize;
use serde_json::json;
use std::io;

#[derive(Deserialize)]
struct Request {
    password: String,
    params: DerivationParams,
    plaintext: Option<String>,
    encrypted: Option<EncryptedData>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let request: Request = serde_json::from_reader(io::stdin())?;
    let key = derive_key(&request.password, &request.params).map_err(|e| format!("{e:?}"))?;
    let result = if let Some(encrypted) = request.encrypted {
        match decrypt(&encrypted, &key) {
            Ok(plaintext) => {
                json!({"key": STANDARD.encode(&key), "plaintext": STANDARD.encode(plaintext)})
            }
            Err(_) => json!({"rejected": true}),
        }
    } else {
        let plaintext = STANDARD.decode(request.plaintext.ok_or("Missing plaintext")?)?;
        json!({"key": STANDARD.encode(&key), "encrypted": encrypt(&plaintext, &key).map_err(|e| format!("{e:?}"))?})
    };
    println!("{result}");
    Ok(())
}
