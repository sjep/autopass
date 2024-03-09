use std::collections::HashMap;
use std::fmt;

use clipboard::ClipboardProvider;
use clipboard::osx_clipboard::OSXClipboardContext;

use crate::hash::TextMode;

use serde::{Serialize, Deserialize};

use super::Serializable;


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

impl Serializable for ServiceEntry {
    fn to_binary(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    fn from_binary(bin: &[u8]) -> Option<Self> {
        match bincode::deserialize(bin) {
            Ok(entry) => Some(entry),
            Err(_) => None
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn sanity_check(&self) -> bool {
        self.pad == 0
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

