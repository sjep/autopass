use std::fmt;

use serde::{Deserialize, Serialize};

use super::{APKey, Serializable, SpecType, IDENTITY_MAGIC};

#[derive(Deserialize, Serialize, Debug)]
pub struct IdentityV2 {
    pub(super) magic: u32,
    pub(super) name: String,
    pub(super) key: APKey,
    pub(super) kv: Vec<(String, String)>,
    pub(super) create_time: u64,
    pub(super) modify_time: u64
}

impl IdentityV2 {
    pub fn new<T: AsRef<str>>(name: &str, key: &APKey, kvs: &[(T, T)]) -> Self {
        let now = super::now();
        let mut kv = vec![];
        for (key, val) in kvs {
            kv.push((key.as_ref().to_owned(), val.as_ref().to_owned()));
        }
        kv.sort();
        Self {
            magic: IDENTITY_MAGIC,
            name: name.to_owned(),
            key: key.to_owned(),
            kv,
            create_time: now,
            modify_time: now
        }
    }

    pub fn key(&self) -> APKey {
        self.key.clone()
    }

    pub fn get_kvs(&self) -> &[(String, String)] {
        &self.kv
    }

    pub fn set_kvs(&mut self, kvs: &[(&str, &str)], reset: bool) {
        if reset {
            self.kv.clear();
        }
        for (key, value) in kvs.iter() {
            self.kv.push((key.to_string(), value.to_string()));
        }
        self.kv.sort();
        self.modify_time = super::now();
    }

    pub fn created(&self) -> String {
        super::timestamp_as_string(self.create_time)
    }

    pub fn modified(&self) -> String {
        super::timestamp_as_string(self.modify_time)
    }

    pub fn version() -> u16 {
        2
    }

    pub fn spec_type() -> SpecType {
        SpecType::Identity
    }
}

impl Serializable for IdentityV2 {
    fn name(&self) -> &str {
        &self.name
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

    fn sanity_check(&self) -> bool {
        self.magic == IDENTITY_MAGIC
    }

    fn version(&self) -> u16 {
        Self::version()
    }

    fn spec_type(&self) -> SpecType {
        Self::spec_type()
    }
}

impl fmt::Display for IdentityV2 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut kvs = String::new();
        for (key, value) in self.kv.iter() {
            kvs = format!("{}  {}: {}\n", kvs, key, value);
        }
        let created = format!("Created: {}", self.created());
        let modified = format!("Modified: {}", self.modified());

        f.write_str(&format!("Name: {}\n{}\n{}\nKey value pairs:\n{}", self.name, created, modified, kvs))
    }
}