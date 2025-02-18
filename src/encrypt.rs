use std::error::Error;
use std::fmt;
use chacha20poly1305::{
    aead::{Aead, KeyInit, AeadCore},
    ChaCha20Poly1305, Key, Nonce,
};
use rand::rngs::OsRng;
use rand_core::RngCore;
use argon2::{Argon2, Algorithm, Version, Params};

#[derive(Debug)]
pub enum EncryptError {
    KeyDerivationError(String),
    EncryptionError(String),
    DecryptionError(String),
}

impl fmt::Display for EncryptError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EncryptError::KeyDerivationError(msg) => write!(f, "Key derivation error: {}", msg),
            EncryptError::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            EncryptError::DecryptionError(msg) => write!(f, "Decryption error: {}", msg),
        }
    }
}

impl Error for EncryptError {}

impl From<Box<dyn Error>> for EncryptError {
    fn from(error: Box<dyn Error>) -> Self {
        EncryptError::KeyDerivationError(error.to_string())
    }
}

pub struct Encryptor {
    argon2: Argon2<'static>,
}

impl Encryptor {
    pub fn new() -> Self {
        let argon2 = Argon2::new(
            Algorithm::Argon2id,        // Argon2id 
            Version::V0x13,             // latest version
            Params::new(
                65536,          // Memory size in KB (64MB = 65536KB)
                3,              // Number of iterations (time)
                4,              // Degree of parallelism (threads)
                Some(32),   // Output length (32 bytes)
            ).unwrap(),
        );

        Self { argon2 }
    }

    fn derive_key(&self, password: &[u8], salt: &[u8]) -> Result<[u8; 32], EncryptError> {
        let mut key = [0u8; 32];
        
        self.argon2
            .hash_password_into(password, salt, &mut key)
            .map_err(|e| EncryptError::KeyDerivationError(format!("Failed to derive key: {}", e)))?;

        Ok(key)
    }

    pub fn encrypt(&self, password: &[u8], data: &[u8]) -> Result<Vec<u8>, EncryptError> {
        // Generate a random salt
        let mut salt = [0u8; 32];
        OsRng.fill_bytes(&mut salt);

        let key = self.derive_key(password, &salt)?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));
        
        // Generate a random nonce
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

        let encrypted_data = cipher
            .encrypt(&nonce, data)
            .map_err(|e| EncryptError::EncryptionError(format!("Encryption failed: {}", e)))?;

        // Combine salt + nonce + encrypted data
        let mut result = Vec::with_capacity(salt.len() + nonce.len() + encrypted_data.len());
        result.extend_from_slice(&salt);
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&encrypted_data);

        Ok(result)
    }

    pub fn decrypt(&self, password: &[u8], encrypted_data: &[u8]) -> Result<Vec<u8>, EncryptError> {
        if encrypted_data.len() < 44 {
            return Err(EncryptError::DecryptionError("Invalid encrypted data length".to_string()));
        }

        // Extract salt, nonce and encrypted data
        let salt = &encrypted_data[..32];
        let nonce = &encrypted_data[32..44];
        let ciphertext = &encrypted_data[44..];

        let key = self.derive_key(password, salt)?;
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key));

        cipher
            .decrypt(Nonce::from_slice(nonce), ciphertext)
            .map_err(|e| EncryptError::DecryptionError(format!("Decryption failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let encryptor = Encryptor::new();
        let password = b"test_password";
        let original_data = b"Hello, this is a test message!";

        // Encrypt the data
        let encrypted = encryptor.encrypt(password, original_data).unwrap();
        
        // Decrypt the data
        let decrypted = encryptor.decrypt(password, &encrypted).unwrap();

        // Compare original and decrypted data
        assert_eq!(original_data.to_vec(), decrypted);
        println!("Original data length: {}", original_data.len());
        println!("Decrypted data length: {}", decrypted.len());
        println!("Original data: {:?}", original_data);
        println!("Decrypted data: {:?}", decrypted);
    }

    #[test]
    fn test_keypair_encryption() {
        let encryptor = Encryptor::new();
        let password = b"test_password";
        
        // Sample keypair data (64 bytes, just for testing)
        let keypair_data: Vec<u8> = (0..64).collect();
        
        // Encrypt the keypair
        let encrypted = encryptor.encrypt(password, &keypair_data).unwrap();
        
        // Decrypt the keypair
        let decrypted = encryptor.decrypt(password, &encrypted).unwrap();

        // Compare original and decrypted keypair
        assert_eq!(keypair_data, decrypted);
        println!("Original keypair length: {}", keypair_data.len());
        println!("Decrypted keypair length: {}", decrypted.len());
        println!("Original keypair: {:?}", keypair_data);
        println!("Decrypted keypair: {:?}", decrypted);
    }
}