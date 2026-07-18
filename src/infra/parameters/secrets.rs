use aes_gcm::{
    Aes256Gcm, Key,
    aead::{Aead, KeyInit, Nonce},
};
use anyhow::{Context, Result, anyhow};
use std::path::Path;
use tokio::fs::read;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MasterKey([u8; 32]);

pub struct EncryptedValue {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub wrapped_dek: Vec<u8>,
    pub dek_nonce: Vec<u8>,
}

fn random_nonce() -> Result<Nonce<Aes256Gcm>> {
    let mut bytes = [0u8; 12];
    getrandom::fill(&mut bytes).context("failed to generate nonce")?;
    let nonce: Nonce<Aes256Gcm> = bytes
        .as_slice()
        .try_into()
        .expect("nonce is exactly NonceSize bytes");
    Ok(nonce)
}

fn nonce_from_bytes(bytes: &[u8]) -> Result<Nonce<Aes256Gcm>> {
    bytes
        .try_into()
        .map_err(|_| anyhow!("invalid nonce length"))
}

impl MasterKey {
    pub async fn load(path: &Path) -> Result<Self> {
        let bytes = Zeroizing::new(
            read(path)
                .await
                .with_context(|| format!("failed to read master key from {}", path.display()))?,
        );
        let key: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("master key must be 32 bytes, got {}", bytes.len()))?;

        Ok(Self(key))
    }

    fn cipher(&self) -> Aes256Gcm {
        let key: Key<Aes256Gcm> = self.0.into();
        Aes256Gcm::new(&key)
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<EncryptedValue> {
        let mut dek = Zeroizing::new([0u8; 32]);
        getrandom::fill(dek.as_mut_slice()).context("failed to generate dek")?;
        let dek_key: Key<Aes256Gcm> = (*dek).into();

        let data_cipher = Aes256Gcm::new(&dek_key);
        let nonce = random_nonce()?;
        let ciphertext = data_cipher
            .encrypt(&nonce, plaintext.as_bytes())
            .map_err(|_| anyhow!("failed to encrypt value"))?;

        let dek_nonce = random_nonce()?;
        let wrapped_dek = self
            .cipher()
            .encrypt(&dek_nonce, dek.as_slice())
            .map_err(|_| anyhow!("failed to wrap dek"))?;

        Ok(EncryptedValue {
            ciphertext,
            nonce: nonce.to_vec(),
            wrapped_dek,
            dek_nonce: dek_nonce.to_vec(),
        })
    }

    pub fn decrypt(&self, enc: &EncryptedValue) -> Result<Zeroizing<String>> {
        let dek_nonce = nonce_from_bytes(&enc.dek_nonce)?;
        let dek = Zeroizing::new(
            self.cipher()
                .decrypt(&dek_nonce, enc.wrapped_dek.as_slice())
                .map_err(|_| anyhow!("failed to unwrap dek"))?,
        );

        let dek_key: Key<Aes256Gcm> = dek
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("invalid dek length"))?;
        let data_cipher = Aes256Gcm::new(&dek_key);
        let nonce = nonce_from_bytes(&enc.nonce)?;
        let plaintext_bytes = Zeroizing::new(
            data_cipher
                .decrypt(&nonce, enc.ciphertext.as_slice())
                .map_err(|_| anyhow!("failed to decrypt value"))?,
        );

        Ok(Zeroizing::new(
            String::from_utf8(plaintext_bytes.to_vec())
                .map_err(|_| anyhow!("decrypted value was not valid utf-8"))?,
        ))
    }
}
