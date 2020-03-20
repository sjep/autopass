use std::collections::HashMap;
use std::fmt;
use std::{fs, fs::File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use clipboard::ClipboardProvider;
use clipboard::osx_clipboard::OSXClipboardContext;

use crate::hash::{HashAlg, get_digest, bin_to_str, TextMode};

use crypto::blockmodes::NoPadding;
use crypto::aes::{cbc_decryptor, cbc_encryptor, KeySize};
use crypto::buffer::{BufferResult, RefReadBuffer, RefWriteBuffer};

use rand::{RngCore, SeedableRng};
use rand::rngs::StdRng;

use serde::{Serialize, Deserialize};


pub const PASS_PATH: &str = ".pass";


fn filename(name: &str) -> String {
    let mut digest = get_digest(HashAlg::SHA256);
    let mut fbin = vec![0; digest.output_bytes()];
    digest.input(name.as_bytes());
    digest.result(&mut fbin);
    digest.reset();
    bin_to_str(&fbin, &TextMode::AlphaNumeric, 32)
}


pub fn full_path(name: &str) -> PathBuf {
    let base_dir = Path::join(&dirs::home_dir().unwrap(), Path::new(PASS_PATH));
    Path::join(&base_dir, Path::new(&filename(name)))
}


#[derive(Deserialize, Serialize, Debug)]
pub struct ServiceEncrypted {
    iv: [u8; 16],
    cyphertext: Vec<u8>
}


#[derive(Deserialize, Serialize, Debug)]
pub struct ServiceEntry {
    pad: u16,
    name: String,
    pass: String,
    nonce: u8,
    kv: HashMap<String, String>,
    len: u8,
    text_mode: TextMode
}

impl ServiceEntry {

    pub fn new(name: &str,
               pass: &str,
               nonce: u8,
               kvs: &[(&str, &str)],
               len: u8,
               text_mode: &TextMode) -> Self {
        let mut kv: HashMap<String, String> = HashMap::new();
        for (key, val) in kvs {
            kv.insert(key.to_string(), val.to_string());
        }
        ServiceEntry{
            pad: 0,
            name: name.to_string(),
            pass: pass.to_string(),
            nonce,
            kv,
            len,
            text_mode: text_mode.clone()
        }
    }

    fn encrypt(&self, key: &[u8]) -> ServiceEncrypted {
        let mut bin = self.to_binary();

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
        ServiceEncrypted{iv, cyphertext}
    }

    fn decrypt(service_encrypted: &ServiceEncrypted, key: &[u8]) -> Option<Self> {
        let iv = &service_encrypted.iv;
        let cyphertext = &service_encrypted.cyphertext;
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

        Self::from_binary(&plaintext)
    }

    pub fn save(&self, key: &[u8]) {
        let path = Path::new(PASS_PATH);
        if !path.exists() {
            fs::create_dir_all(path).unwrap();
        }

        let encrypted = self.encrypt(key);
        let data = bincode::serialize(&encrypted).unwrap();
        let full_path = full_path(&self.name);
        let mut file = File::create(full_path).unwrap();
        file.write_all(&data).unwrap();
    }

    pub fn load(file: &mut File, key: &[u8]) -> Result<Self, &'static str> {
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).unwrap();
        let service_encrypted: ServiceEncrypted = bincode::deserialize(&buffer).unwrap();
        match Self::decrypt(&service_encrypted, key) {
            Some(entry) => {
                match entry.pad == 0 {
                    true => Ok(entry),
                    false => Err("Wrong password")
                }
            },
            None => Err("Wrong password")
        }
    }

    fn to_binary(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    fn from_binary(bin: &[u8]) -> Option<Self> {
        match bincode::deserialize(bin) {
            Ok(entry) => Some(entry),
            Err(_) => None
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_kvs(&self) -> &HashMap<String, String> {
        &self.kv
    }

    pub fn set_kvs(&mut self, kvs: &[(&str, &str)], reset: bool) {
        if reset {
            self.kv.clear();
        }
        for (key, value) in kvs.iter() {
            self.kv.insert(key.to_string(), value.to_string());
        }
    }

    pub fn get_pass(&self, clipboard: bool) -> Option<&str> {
        match clipboard {
            true => {
                let mut clipboard = OSXClipboardContext::new().unwrap();
                clipboard.set_contents(self.pass.to_string()).unwrap();
                None
            },
            false => {
                Some(&self.pass)
            }
        }
    }

    pub fn uptick(&mut self) -> u8 {
        self.nonce += 1;
        self.nonce
    }

    pub fn get_text_mode(&self) -> &TextMode {
        &self.text_mode
    }

    pub fn get_len(&self) -> u8 {
        self.len
    }

    pub fn set_pass(&mut self, pass: &str) {
        self.pass = pass.to_string();
    }

    pub fn to_string(&self) -> String {
        format!("{}: {:?}", self.name, self.kv)
    }
}

impl fmt::Display for ServiceEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut kvs = String::new();
        for (key, value) in self.kv.iter() {
            kvs = format!("{}{}: {}\n", kvs, key, value);
        }
        f.write_str(
            &format!("Name: {}\nPass: {}\nKey value pairs:\n{}", self.name, self.pass, kvs))
    }
}

