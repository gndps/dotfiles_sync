use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{bail, Context, Result};
use bip39::{Language, Mnemonic};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;
use std::fs;
use std::path::{Path, PathBuf};

const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;
const PBKDF2_ITERATIONS: u32 = 100_000;
// Key stored in HOME directory for security - NEVER in repo!
const ENCRYPTION_KEY_FILE: &str = ".dotfiles.encryption.key";
// Marker file in repo to indicate encryption is used
const ENCRYPTION_MARKER_FILE: &str = ".dotfiles.encryption.enabled";

pub struct FileEncryptor;

impl FileEncryptor {
    /// Get the path to the encryption key file in HOME directory (NOT repo!)
    pub fn get_key_file_path() -> Result<PathBuf> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        Ok(home.join(ENCRYPTION_KEY_FILE))
    }

    /// Get the path to the encryption marker file in the repository
    fn get_marker_file_path(repo_path: &Path) -> PathBuf {
        repo_path.join(ENCRYPTION_MARKER_FILE)
    }

    /// Check if encryption is enabled in the repository
    pub fn is_encryption_setup(repo_path: &Path) -> bool {
        Self::get_marker_file_path(repo_path).exists()
    }

    /// Check if encryption key exists in home directory
    pub fn has_local_key() -> bool {
        Self::get_key_file_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    /// Generate a new BIP39 mnemonic (12 words)
    pub fn generate_mnemonic() -> Result<Mnemonic> {
        let mut entropy = [0u8; 16]; // 16 bytes = 128 bits = 12 words
        OsRng.fill_bytes(&mut entropy);
        
        Mnemonic::from_entropy_in(Language::English, &entropy)
            .map_err(|e| anyhow::anyhow!("Failed to generate mnemonic: {}", e))
    }

    /// Derive encryption key from mnemonic seed phrase
    pub fn derive_key_from_mnemonic(mnemonic: &Mnemonic) -> [u8; KEY_SIZE] {
        let seed = mnemonic.to_seed("");
        let mut key = [0u8; KEY_SIZE];
        pbkdf2_hmac::<Sha256>(&seed[..32], b"dotfiles-encryption", PBKDF2_ITERATIONS, &mut key);
        key
    }

    /// Save encryption key to HOME directory (NEVER to repo!)
    pub fn save_key_to_home(key: &[u8; KEY_SIZE]) -> Result<()> {
        let key_path = Self::get_key_file_path()?;
        let encoded = base64::encode(key);
        fs::write(&key_path, encoded)
            .context("Failed to write encryption key to home directory")?;
        Ok(())
    }

    /// Create marker file in repo to indicate encryption is used
    pub fn create_encryption_marker(repo_path: &Path) -> Result<()> {
        let marker_path = Self::get_marker_file_path(repo_path);
        fs::write(&marker_path, "This repository uses BIP39 seed phrase encryption.\nThe encryption key is stored in your home directory, NOT in this repo.\nYou will need your 12-word seed phrase to decrypt files on a new machine.")
            .context("Failed to create encryption marker file")?;
        Ok(())
    }

    /// Load encryption key from HOME directory
    pub fn load_key_from_home() -> Result<[u8; KEY_SIZE]> {
        let key_path = Self::get_key_file_path()?;
        
        if !key_path.exists() {
            bail!("Encryption key not found in home directory. You need to enter your seed phrase.");
        }

        let encoded = fs::read_to_string(&key_path)
            .context("Failed to read encryption key file")?;
        
        let decoded = base64::decode(encoded.trim())
            .context("Failed to decode encryption key")?;
        
        if decoded.len() != KEY_SIZE {
            bail!("Invalid encryption key size");
        }

        let mut key = [0u8; KEY_SIZE];
        key.copy_from_slice(&decoded);
        Ok(key)
    }

    /// Encrypt a file using the provided key
    pub fn encrypt_file(source: &Path, dest: &Path, key: &[u8; KEY_SIZE]) -> Result<()> {
        let content = fs::read(source).context("Failed to read source file")?;
        let encrypted = Self::encrypt_data(&content, key)?;
        
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(dest, encrypted).context("Failed to write encrypted file")?;
        Ok(())
    }

    /// Decrypt a file using the provided key
    pub fn decrypt_file(source: &Path, dest: &Path, key: &[u8; KEY_SIZE]) -> Result<()> {
        let encrypted = fs::read(source).context("Failed to read encrypted file")?;
        let decrypted = Self::decrypt_data(&encrypted, key)?;
        
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        
        fs::write(dest, decrypted).context("Failed to write decrypted file")?;
        Ok(())
    }

    /// Encrypt data using the provided key
    pub fn encrypt_data(data: &[u8], key: &[u8; KEY_SIZE]) -> Result<Vec<u8>> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| anyhow::anyhow!("Failed to create cipher: {}", e))?;
        
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;
        
        let mut result = Vec::new();
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }

    /// Decrypt data using the provided key
    pub fn decrypt_data(data: &[u8], key: &[u8; KEY_SIZE]) -> Result<Vec<u8>> {
        if data.len() < NONCE_SIZE {
            bail!("Invalid encrypted data: too short");
        }

        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| anyhow::anyhow!("Failed to create cipher: {}", e))?;
        
        let nonce = Nonce::from_slice(&data[..NONCE_SIZE]);
        let ciphertext = &data[NONCE_SIZE..];
        
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;
        
        Ok(plaintext)
    }

    /// Display the seed phrase to the user with prominent warnings
    pub fn display_seed_phrase(mnemonic: &Mnemonic) {
        use colored::Colorize;
        
        println!();
        println!("{}", "═══════════════════════════════════════════════════════════════".yellow().bold());
        println!("{}", "                  🔐 ENCRYPTION SEED PHRASE                   ".yellow().bold());
        println!("{}", "═══════════════════════════════════════════════════════════════".yellow().bold());
        println!();
        println!("{}", "⚠️  CRITICAL: SAVE THIS SEED PHRASE NOW! ⚠️".red().bold());
        println!();
        println!("   {}", "This is your 12-word BIP39 seed phrase:".bold());
        println!();
        
        let words: Vec<&str> = mnemonic.word_iter().collect();
        for (i, word) in words.iter().enumerate() {
            print!("   {:2}. {:12}", i + 1, word.green().bold());
            if (i + 1) % 3 == 0 {
                println!();
            }
        }
        println!();
        println!();
        println!("{}", "⚠️  IMPORTANT SECURITY NOTICE:".yellow().bold());
        println!("   • {}", "You will NOT see this seed phrase again".bold());
        println!("   • {}", "Write it down on paper (NOT digitally)".bold());
        println!("   • {}", "Keep it in a safe place".bold());
        println!("   • {}", "You need this to decrypt files on new machines".bold());
        println!("   • {}", "Anyone with this phrase can decrypt your files".bold());
        println!();
        println!("{}", "═══════════════════════════════════════════════════════════════".yellow().bold());
        println!();
    }

    /// Prompt user to enter their seed phrase for decryption
    pub fn prompt_for_seed_phrase() -> Result<Mnemonic> {
        use colored::Colorize;
        
        println!();
        println!("{}", "🔐 Enter your 12-word seed phrase to decrypt files:".bold());
        println!("   (Enter all 12 words separated by spaces)");
        println!();
        print!("   Seed phrase: ");
        
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)
            .context("Failed to read seed phrase")?;
        
        let mnemonic = Mnemonic::parse_in(Language::English, input.trim())
            .map_err(|e| anyhow::anyhow!("Invalid seed phrase: {}", e))?;
        
        if mnemonic.word_count() != 12 {
            bail!("Seed phrase must be exactly 12 words");
        }
        
        Ok(mnemonic)
    }
}
