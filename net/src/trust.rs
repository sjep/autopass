use std::{io::{Read, Write}, path::Path};

use serde::{Deserialize, Serialize};

use crate::{create_or_truncate, ErrorIdentity};

#[derive(Debug, Serialize, Deserialize)]
pub struct IdentityList(Vec<Identity>);

impl IdentityList {
    const IDLIST_FILE: &'static str = "trusted_certs";

    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn exists(certdir: &Path) -> bool {
        let p = certdir.join(Self::IDLIST_FILE);
        std::fs::exists(p).unwrap_or(false)
    }

    pub fn save(&self, certdir: &Path) -> Result<(), ErrorIdentity> {
        let trustpath = certdir.join(Self::IDLIST_FILE);
        let buf = bincode::serialize(self)?;
        let mut file = create_or_truncate(&trustpath)?;
        file.write_all(&buf)?;
        Ok(())
    }

    pub fn load(certdir: &Path) -> Result<Self, ErrorIdentity> {
        let trustpath = certdir.join(Self::IDLIST_FILE);
        let mut buf = vec![];
        let mut file = std::fs::File::open(trustpath)?;
        file.read_to_end(&mut buf)?;

        let idlist: Self = bincode::deserialize(&buf)?;
        Ok(idlist)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Identity {
    name: String,
    pubkey: Vec<u8>
}

impl Identity {
    pub fn new(name: String, pubkey: Vec<u8>) -> Self {
        Self {
            name,
            pubkey
        }
    }
}