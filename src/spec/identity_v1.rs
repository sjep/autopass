use std::{collections::HashMap, path::{Path, PathBuf}};

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