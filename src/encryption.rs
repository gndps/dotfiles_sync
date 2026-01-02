use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use rand::RngCore;
use std::fs;
use std::path::Path;

const NONCE_SIZE: usize = 12;

pub struct FileEncryptor;

impl FileEncryptor {
    pub fn encrypt_file(source: &Path, dest: &Path, password: &str) -> Result<()> {
        let content = fs::read(source).context("Failed to read source file")?;
        
        let encrypted = Self::encrypt_data(&content, password)?;
        
        fs::write(dest, encrypted).context("Failed to write encrypted file")?;
        
        Ok(())
    }

    pub fn decrypt_file(source: &Path, dest: &Path, password: &str) -> Result<()> {
        let encrypted = fs::read(source).context("Failed to read encrypted file")?;
        
        let decrypted = Self::decrypt_data(&encrypted, password)?;
        
        fs::write(dest, decrypted).context("Failed to write decrypted file")?;
        
        Ok(())
    }

    pub fn encrypt_data(data: &[u8], password: &str) -> Result<Vec<u8>> {
        let salt = SaltString::generate(&mut OsRng);
        
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
        
        let hash = password_hash.hash.context("No hash generated")?;
        let key_bytes = hash.as_bytes();
        let key = &key_bytes[..32];
        
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| anyhow::anyhow!("Failed to create cipher: {}", e))?;
        
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
        
        let mut result = Vec::new();
        result.extend_from_slice(salt.as_str().as_bytes());
        result.push(0);
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }

    pub fn decrypt_data(data: &[u8], password: &str) -> Result<Vec<u8>> {
        let null_pos = data
            .iter()
            .position(|&b| b == 0)
            .context("Invalid encrypted data format")?;
        
        let salt_str = std::str::from_utf8(&data[..null_pos])
            .context("Invalid salt format")?;
        let salt = SaltString::from_b64(salt_str)
            .map_err(|e| anyhow::anyhow!("Failed to parse salt: {}", e))?;
        
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
        
        let hash = password_hash.hash.context("No hash generated")?;
        let key_bytes = hash.as_bytes();
        let key = &key_bytes[..32];
        
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| anyhow::anyhow!("Failed to create cipher: {}", e))?;
        
        let nonce_start = null_pos + 1;
        let nonce_end = nonce_start + NONCE_SIZE;
        let nonce = Nonce::from_slice(&data[nonce_start..nonce_end]);
        
        let ciphertext = &data[nonce_end..];
        
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;
        
        Ok(plaintext)
    }

    pub fn prompt_password(confirm: bool) -> Result<String> {
        let password = rpassword::prompt_password("Enter password: ")
            .context("Failed to read password")?;
        
        if confirm {
            let confirm_password = rpassword::prompt_password("Confirm password: ")
                .context("Failed to read confirmation password")?;
            
            if password != confirm_password {
                anyhow::bail!("Passwords do not match");
            }
        }
        
        if password.is_empty() {
            anyhow::bail!("Password cannot be empty");
        }
        
        Ok(password)
    }
}
