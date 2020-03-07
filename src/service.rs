use std::collections::HashMap;
use std::fmt;
use std::{fs, fs::File};
use std::io::prelude::*;
use std::path::{Path, PathBuf};

use serde::{Serialize, Deserialize};
use crypto::aessafe::{AesSafe256Encryptor, AesSafe256Decryptor};
use crypto::symmetriccipher::{BlockDecryptor, BlockEncryptor};
use clipboard::ClipboardProvider;
use clipboard::osx_clipboard::OSXClipboardContext;

use crate::hash::{HashAlg, get_digest, bin_to_str, TextMode};


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


pub fn create_key(pass: &str) -> Vec<u8> {
    let mut digest = get_digest(HashAlg::SHA256);
    let mut key = vec![0; digest.output_bytes()];
    digest.input(pass.as_bytes());
    digest.result(&mut key);
    key
}


#[derive(Deserialize, Serialize, Debug)]
pub struct ServiceEntry {
    name: String,
    pass: String,
    nonce: u8,
    kv: HashMap<String, String>,
    len: usize,
    text_mode: TextMode
}

impl ServiceEntry {

    pub fn new(name: &str,
               pass: &str,
               nonce: u8,
               kvs: &[(&str, &str)],
               len: usize,
               text_mode: &TextMode) -> Self {
        let mut kv: HashMap<String, String> = HashMap::new();
        for (key, val) in kvs {
            kv.insert(key.to_string(), val.to_string());
        }
        ServiceEntry{
            name: name.to_string(),
            pass: pass.to_string(),
            nonce,
            kv,
            len,
            text_mode: text_mode.clone()
        }
    }

    fn encrypt(&self, key: &[u8]) -> Vec<u8> {
        let mut bin = self.to_binary();

        let encrypt = AesSafe256Encryptor::new(key);
        let blocksize = encrypt.block_size();
        let leftover = blocksize - bin.len() % blocksize;
        for _ in 0..leftover {
            bin.push(0);
        }
        let blocks = bin.len() / blocksize;
        let mut out = vec![0; blocks * blocksize];
        for i in 0..blocks {
            let inp: &[u8] = &bin[i * blocksize..(i + 1) * blocksize];
            let outp: &mut [u8] = &mut out[i * blocksize..(i + 1) * blocksize];
            encrypt.encrypt_block(inp, outp);
        }
        out
    }

    fn decrypt(encrypted: &[u8], key: &[u8]) -> Option<Self> {
        let decrypt = AesSafe256Decryptor::new(key);
        let blocksize = decrypt.block_size();
        let blocks = encrypted.len() / blocksize;

        let mut out = vec![0; blocks * blocksize];
        for i in 0..blocks {
            let inp: &[u8] = &encrypted[i * blocksize..(i + 1) * blocksize];
            let outp: &mut [u8] = &mut out[i * blocksize..(i + 1) * blocksize];
            decrypt.decrypt_block(inp, outp);
        }

        Self::from_binary(&out)
    }

    pub fn save(&self, key: &[u8]) {
        let path = Path::new(PASS_PATH);
        if !path.exists() {
            fs::create_dir_all(path).unwrap();
        }

        let encrypted = self.encrypt(key);
        let full_path = full_path(&self.name);
        let mut file = File::create(full_path).unwrap();
        file.write_all(&encrypted).unwrap();
    }

    pub fn load(file: &mut File, key: &[u8]) -> Result<Self, &'static str> {
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).unwrap();
        match Self::decrypt(&buffer, key) {
            Some(entry) => Ok(entry),
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

    pub fn set_kvs(&mut self, kvs: &[(&str, &str)]) {
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

    pub fn get_len(&self) -> usize {
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

