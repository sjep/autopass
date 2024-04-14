use std::{collections::HashMap, fmt, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

use super::{APKey, Serializable, SpecType};

const IDENTITY_MAGIC: u32 = 0xfedb1234;
const IDENTITY_FNAME: &str = ".apid";

pub fn identity_path<P: AsRef<Path>>(basedir: P) -> PathBuf {
    Path::join(basedir.as_ref(), IDENTITY_FNAME)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct IdentityV1 {
    magic: u32,
    name: String,
    key: APKey,
    kv: HashMap<String, String>,
    create_time: u64,
    modify_time: u64
}

impl IdentityV1 {
    pub fn new<T: AsRef<str>>(name: &str, key: &APKey, kvs: &[(T, T)]) -> Self {
        let now = super::now();
        let mut kv: HashMap<String, String> = HashMap::new();
        for (key, val) in kvs {
            kv.insert(key.as_ref().to_owned(), val.as_ref().to_owned());
        }
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

    pub fn created(&self) -> String {
        super::timestamp_as_string(self.create_time)
    }

    pub fn modified(&self) -> String {
        super::timestamp_as_string(self.modify_time)
    }
}

impl Serializable for IdentityV1 {
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
        1
    }

    fn spec_type(&self) -> SpecType {
        SpecType::Identity
    }
}

impl fmt::Display for IdentityV1 {
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