use std::fmt;

use clipboard::ClipboardProvider;
use clipboard::osx_clipboard::OSXClipboardContext;

use crate::hash::TextMode;

use serde::{Serialize, Deserialize};

use super::{Serializable, SERVICE_MAGIC};

#[derive(Deserialize, Serialize, Debug)]
pub struct ServiceEntryV2 {
    pub(super) magic: u32,
    pub(super) name: String,
    pub(super) pass: String,
    pub(super) nonce: u8,
    pub(super) kv: Vec<(String, String)>,
    pub(super) tags: Vec<String>,
    pub(super) len: u8,
    pub(super) text_mode: TextMode,
    pub(super) create_time: u64,
    pub(super) modify_time: u64
}

impl ServiceEntryV2 {

    pub fn new<T: AsRef<str>>(
        name: &str,
        pass: &str,
        nonce: u8,
        kvs: &[(T, T)],
        tgs: &[T],
        len: u8,
        text_mode: &TextMode) -> Self
    {
        let mut kv = vec![];
        for (key, val) in kvs {
            kv.push((key.as_ref().to_owned(), val.as_ref().to_owned()));
        }
        kv.sort();
        let mut tags = vec![];
        for tag in tgs {
            tags.push(tag.as_ref().to_owned());
        }
        tags.sort();
        let now = super::now();
        Self {
            magic: SERVICE_MAGIC,
            name: name.to_string(),
            pass: pass.to_string(),
            nonce,
            kv,
            tags,
            len,
            text_mode: text_mode.clone(),
            create_time: now,
            modify_time: now
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_kvs(&self) -> &[(String, String)] {
        &self.kv
    }

    pub fn set_kvs(&mut self, kvs: &[(&str, &str)], reset: bool) {
        if reset {
            self.kv.clear();
        }
        for (key, value) in kvs {
            self.kv.push((key.to_string(), value.to_string()));
        }
        self.kv.sort();
        self.modify_time = super::now();
    }

    pub fn get_tags(&self) -> &[String] {
        &self.tags
    }

    pub fn set_tags<S: AsRef<str>>(&mut self, tags: &[S], reset: bool) {
        if reset {
            self.tags.clear();
        }
        for tag in tags {
            self.tags.push(tag.as_ref().to_string());
        }
        self.tags.sort();
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
        2
    }

}

impl Serializable for ServiceEntryV2 {
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
        self.magic == SERVICE_MAGIC
    }

    fn version(&self) -> u16 {
        Self::version()
    }

    fn spec_type(&self) -> super::SpecType {
        Self::spec_type()
    }
}

impl fmt::Display for ServiceEntryV2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut kvs = String::new();
        for (key, value) in self.kv.iter() {
            kvs = format!("{}  {}: {}\n", kvs, key, value);
        }
        let created = format!("Created: {}", self.created());
        let modified = format!("Modified: {}", self.modified());

        let tags = self.tags.join("\n  ");

        f.write_str(&format!("Name: {}\nPass: {}\n{}\n{}\nKey value pairs:\n{}Tags:\n  {}", self.name, self.pass, created, modified, kvs, tags))
    }
}