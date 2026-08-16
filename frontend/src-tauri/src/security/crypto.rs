// frontend/src-tauri/src/security/crypto.rs
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::prelude::*;
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

const STATIC_SALT: &[u8] = b"scramjet-salt-static";
const PBKDF2_ROUNDS: u32 = 100_000;

#[derive(Serialize, Deserialize, Debug)]
pub struct EncryptedPayload {
    pub payload: String, // Base64 ciphertext + tag
    pub iv: String,      // Base64 12-byte IV
}

/// Выводит 256-битный AES-GCM ключ из мастер-пароля пользователя с помощью PBKDF2-HMAC-SHA256
fn derive_key(password: &str) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), STATIC_SALT, PBKDF2_ROUNDS, &mut key);
    key
}

/// Шифрует произвольную JSON-строку в формат AES-256-GCM (Zero-Knowledge)
pub fn encrypt_string(data_json: &str, password: &str) -> Result<EncryptedPayload, String> {
    let key_bytes = derive_key(password);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| e.to_string())?;

    // Генерируем случайный 12-байтный Nonce / IV
    let mut iv = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut iv);
    let nonce = Nonce::from_slice(&iv);

    let ciphertext = cipher
        .encrypt(nonce, data_json.as_bytes())
        .map_err(|e| format!("Encryption error: {:?}", e))?;

    Ok(EncryptedPayload {
        payload: BASE64_STANDARD.encode(&ciphertext),
        iv: BASE64_STANDARD.encode(&iv),
    })
}

/// Расшифровывает зашифрованный payload обратно в JSON-строку
pub fn decrypt_string(
    encrypted_base64: &str,
    iv_base64: &str,
    password: &str,
) -> Result<String, String> {
    let key_bytes = derive_key(password);
    let cipher = Aes256Gcm::new_from_slice(&key_bytes).map_err(|e| e.to_string())?;

    let iv = BASE64_STANDARD
        .decode(iv_base64)
        .map_err(|e| format!("Invalid IV base64: {}", e))?;
    if iv.len() != 12 {
        return Err("Invalid IV length (must be 12 bytes)".to_string());
    }

    let ciphertext = BASE64_STANDARD
        .decode(encrypted_base64)
        .map_err(|e| format!("Invalid payload base64: {}", e))?;

    let nonce = Nonce::from_slice(&iv);
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "Decryption failed: invalid password or corrupted data".to_string())?;

    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 decode error: {}", e))
}
