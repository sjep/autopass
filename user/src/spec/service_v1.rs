use std::collections::HashMap;
use std::fmt;

use clipboard::ClipboardProvider;
use clipboard::osx_clipboard::OSXClipboardContext;

use crate::hash::TextMode;

use serde::{Serialize, Deserialize};

use super::Serializable;

#[derive(Deserialize, Serialize, Debug)]
pub struct ServiceEntryV1 {
    pub(super) pad: u16,
    pub(super) name: String,
    pub(super) pass: String,
    pub(super) nonce: u8,
    pub(super) kv: HashMap<String, String>,
    pub(super) len: u8,
    pub(super) text_mode: TextMode,
    pub(super) create_time: u64,
    pub(super) modify_time: u64
}

impl ServiceEntryV1 {

    pub fn new<T: AsRef<str>>(
        name: &str,
        pass: &str,
        nonce: u8,
        kvs: &[(T, T)],
        len: u8,
        text_mode: &TextMode) -> Self
    {
        let mut kv: HashMap<String, String> = HashMap::new();
        for (key, val) in kvs {
            kv.insert(key.as_ref().to_owned(), val.as_ref().to_owned());
        }
        let now = super::now();
        Self {
            pad: 0,
            name: name.to_string(),
            pass: pass.to_string(),
            nonce,
            kv,
            len,
            text_mode: text_mode.clone(),
            create_time: now,
            modify_time: now
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
        self.modify_time = super::now();
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
        self.modify_time = super::now();
    }

    pub fn to_string(&self) -> String {
        format!("{}: {:?}", self.name, self.kv)
    }

    pub fn created(&self) -> String {
        super::timestamp_as_string(self.create_time)
    }

    pub fn modified(&self) -> String {
        super::timestamp_as_string(self.modify_time)
    }

    pub fn spec_type() -> super::SpecType {
        super::SpecType::Service
    }

    pub fn version() -> u16 {
        1
    }

}

impl Serializable for ServiceEntryV1 {
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

    fn version(&self) -> u16 {
        Self::version()
    }

    fn spec_type(&self) -> super::SpecType {
        Self::spec_type()
    }
}

impl fmt::Display for ServiceEntryV1 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut kvs = String::new();
        for (key, value) in self.kv.iter() {
            kvs = format!("{}  {}: {}\n", kvs, key, value);
        }
        let created = format!("Created: {}", self.created());
        let modified = format!("Modified: {}", self.modified());

        f.write_str(&format!("Name: {}\nPass: {}\n{}\n{}\nKey value pairs:\n{}", self.name, self.pass, created, modified, kvs))
    }
}

