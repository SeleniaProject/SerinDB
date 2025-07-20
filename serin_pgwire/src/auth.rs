//! Authentication utilities (MD5 & SCRAM) and configuration loader.

use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use serde::Deserialize;
use sha2::Sha256;
use pbkdf2::pbkdf2_hmac;
use base64::{engine::general_purpose, Engine as _};
use rand::{RngCore, rngs::OsRng};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    pub users: HashMap<String, String>, // username -> plaintext password (demo)
}

impl AuthConfig {
    pub fn load(path: &str) -> anyhow::Result<Arc<Self>> {
        let content = fs::read_to_string(path)?;
        let config: AuthConfig = serde_yaml::from_str(&content)?;
        Ok(Arc::new(config))
    }

    pub fn password(&self, user: &str) -> Option<&str> { self.users.get(user).map(|s| s.as_str()) }
}

// === MD5 ===
fn md5_hex(data: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

pub fn verify_md5_password(stored_pwd: &str, user: &str, client_resp: &str, salt: &[u8; 4]) -> bool {
    let mut inner = Vec::new();
    inner.extend_from_slice(stored_pwd.as_bytes());
    inner.extend_from_slice(user.as_bytes());
    let hash1 = md5_hex(&inner);
    let mut outer = Vec::new();
    outer.extend_from_slice(hash1.as_bytes());
    outer.extend_from_slice(salt);
    let hash2 = md5_hex(&outer);
    let expected = format!("md5{}", hash2);
    expected == client_resp
}

// === SCRAM (simplified) ===
/// Generate server signature for SCRAM.
pub fn scram_server_key(password: &str, salt: &[u8], iterations: u32) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(password.as_bytes()).unwrap();
    mac.update(salt);
    let mut ui = mac.finalize().into_bytes();
    let mut output = ui.clone();
    for _ in 1..iterations {
        let mut mac = HmacSha256::new_from_slice(password.as_bytes()).unwrap();
        mac.update(&ui);
        ui = mac.finalize().into_bytes();
        for (o, u) in output.iter_mut().zip(ui.iter()) { *o ^= u; }
    }
    output.to_vec()
}

pub struct ScramCred {
    pub salted_password: Vec<u8>,
    pub salt: Vec<u8>,
    pub iterations: u32,
}

pub fn build_scram_credentials(password: &str, iterations: u32) -> ScramCred {
    let mut salt = vec![0u8; 16];
    OsRng.fill_bytes(&mut salt);
    let salted = derive_salted_password(password, &salt, iterations);
    ScramCred { salted_password: salted, salt, iterations }
}

pub fn derive_salted_password(password: &str, salt: &[u8], iterations: u32) -> Vec<u8> {
    let mut out = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, iterations, &mut out);
    out.to_vec()
}

pub fn client_key(salted: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(salted).unwrap();
    mac.update(b"Client Key");
    mac.finalize().into_bytes().to_vec()
}

pub fn stored_key(client_key: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(client_key);
    hasher.finalize().to_vec()
}

pub fn server_key(salted: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(salted).unwrap();
    mac.update(b"Server Key");
    mac.finalize().into_bytes().to_vec()
}

pub fn base64_encode(data: &[u8]) -> String { general_purpose::STANDARD.encode(data) } 

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_invalid_password() {
        let salt = [1u8, 2, 3, 4];
        let ok = verify_md5_password("secret", "alice", "md5deadbeef", &salt);
        assert!(!ok);
    }
} 