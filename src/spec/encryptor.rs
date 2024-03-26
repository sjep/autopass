use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key
};
use serde::{Deserialize, Serialize};

use super::{Encryptor, SpecType};


#[derive(Serialize, Deserialize)]
pub struct Encrypt {
    spec_type: SpecType,
    version: u16,
    nonce: [u8; 12],
    ciphertext: Vec<u8>
}

impl Encryptor for Encrypt {
    fn encrypt<T: super::Serializable>(key: &[u8], obj: &T) -> Self {
        let bin = obj.to_binary();

        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(&key);
        let n = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&n, bin.as_slice()).unwrap();
        let mut nonce: [u8; 12] = [0; 12];
        nonce.copy_from_slice(n.as_slice());
        Self {
            spec_type: T::spec_type(),
            version: T::version(),
            nonce,
            ciphertext
        }
    }

    fn decrypt<T: super::Serializable>(&self, key: &[u8]) -> Option<T> {
        let key = Key::<Aes256Gcm>::from_slice(key);
        let cipher = Aes256Gcm::new(&key);

        let nonce = Nonce::from_slice(&self.nonce);
        let plaintext = cipher.decrypt(&nonce, self.ciphertext.as_ref()).ok()?;
        
        T::from_binary(&plaintext)
    }
}