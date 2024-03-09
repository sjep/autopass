use std::{fs::File, io::{Read, Write}, path::{Path, PathBuf}};

use crypto::{aes::{cbc_decryptor, cbc_encryptor, KeySize}, blockmodes::NoPadding, buffer::{BufferResult, RefReadBuffer, RefWriteBuffer}};
use rand::{rngs::StdRng, RngCore, SeedableRng};
use serde::{Serialize, Deserialize};

use crate::hash::{bin_to_str, get_digest, HashAlg, TextMode};

pub mod service;

pub const PASS_PATH: &'static str = ".pass";
pub const PASS_BASE_ENVVAR: &'static str = "AP_BASEDIR";


fn filename(name: &str) -> String {
    let mut digest = get_digest(HashAlg::SHA256);
    let mut fbin = vec![0; digest.output_bytes()];
    digest.input(name.as_bytes());
    digest.result(&mut fbin);
    digest.reset();
    bin_to_str(&fbin, &TextMode::AlphaNumeric, 32)
}

pub fn base_path() -> PathBuf {
    if let Ok(basepath) = std::env::var(PASS_BASE_ENVVAR) {
        return basepath.into();
    }
    Path::join(&dirs::home_dir().unwrap(), Path::new(PASS_PATH))
}

pub fn full_path(name: &str) -> PathBuf {
    Path::join(&base_path(), Path::new(&filename(name)))
}

pub trait Serializable: Sized {
    fn name(&self) -> &str;

    fn to_binary(&self) -> Vec<u8>;

    fn from_binary(bin: &[u8]) -> Option<Self>;

    fn sanity_check(&self) -> bool;
}

#[derive(Deserialize, Serialize, Debug)]
pub struct ServiceEncrypted {
    iv: [u8; 16],
    cyphertext: Vec<u8>
}

impl ServiceEncrypted {

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
        Self{iv, cyphertext}
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

pub fn save<T: Serializable>(key: &[u8], service: &T) {
    let path = base_path();
    if !path.exists() {
        std::fs::create_dir_all(path).unwrap();
    }

    let encrypted = ServiceEncrypted::encrypt(key, service);
    let data = bincode::serialize(&encrypted).unwrap();
    let full_path = full_path(service.name());
    let mut file = File::create(full_path).unwrap();
    file.write_all(&data).unwrap();
}

pub fn load<T: Serializable>(file: &mut File, key: &[u8]) -> Result<T, &'static str> {
    let mut buffer = vec![];
    file.read_to_end(&mut buffer).unwrap();
    let service_encrypted: ServiceEncrypted = bincode::deserialize(&buffer).unwrap();
    match service_encrypted.decrypt::<T>(key) {
        Some(entry) => {
            match entry.sanity_check() {
                true => Ok(entry),
                false => Err("Wrong password")
            }
        },
        None => Err("Wrong password")
    }
}