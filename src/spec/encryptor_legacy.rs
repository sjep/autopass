use crypto::{aes::{cbc_decryptor, cbc_encryptor, KeySize}, blockmodes::NoPadding, buffer::{BufferResult, RefReadBuffer, RefWriteBuffer}};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use serde::{Deserialize, Serialize};

use super::{Encryptor, Serializable, SpecType};


#[derive(Deserialize, Serialize, Debug)]
pub struct PassData {
    spec_type: SpecType,
    version: u16,
    iv: [u8; 16],
    cyphertext: Vec<u8>
}

impl Encryptor for PassData {
    fn encrypt<T: Serializable>(key: &[u8], service: &T) -> Self {
        let mut bin = service.to_binary();

        let mut gen: StdRng = StdRng::from_seed([5u8; 32]);
        let mut iv: [u8; 16] = [0; 16];
        gen.fill_bytes(&mut iv);

        let blocksize = 16;

        let leftover = blocksize - bin.len() % blocksize;
        for _ in 0..leftover {
            bin.push(0);
        }

        let mut cyphertext = vec![0; bin.len()];

        let mut encryptor = cbc_encryptor(KeySize::KeySize256, &key, &iv, NoPadding);
        match encryptor.encrypt(&mut RefReadBuffer::new(&bin),
                                &mut RefWriteBuffer::new(&mut cyphertext),
                                true) {
            Ok(buf_res) => if let BufferResult::BufferOverflow = buf_res {
                assert!(false, "Buffer incorrect size. Encrypt aborted");
            },
            Err(e) => {
                assert!(false, "Encrypt Error: {:?}", e);
            }
        }
        Self{spec_type: T::spec_type(), version: T::version(), iv, cyphertext}
    }

    fn decrypt<T: Serializable>(&self, key: &[u8]) -> Option<T> {
        let iv = &self.iv;
        let cyphertext = &self.cyphertext;
        let mut decryptor = cbc_decryptor(KeySize::KeySize256, &key, iv, NoPadding);
        let mut plaintext = vec![0; cyphertext.len()];
        match decryptor.decrypt(&mut RefReadBuffer::new(cyphertext),
                                &mut RefWriteBuffer::new(&mut plaintext),
                                true) {
            Ok(buf_res) => if let BufferResult::BufferOverflow = buf_res {
                assert!(false, "Buffer incorrect size. Decrypt aborted");
            },
            Err(e) => {
                assert!(false, "Decrypt Error: {:?}", e);
            }
        }

        T::from_binary(&plaintext)
    }

}