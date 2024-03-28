use std::{fs::File, io::{Read, Write}, path::{Path, PathBuf}};

use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

use crate::{api::APError, hash::{bin_to_str, TextMode}};

pub mod service_v1;
pub mod encryptor;

pub const PASS_PATH: &'static str = ".pass";
pub const PASS_BASE_ENVVAR: &'static str = "AP_BASEDIR";


pub fn filename(name: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(name.as_bytes());
    let res = hasher.finalize();
    bin_to_str(&res, &TextMode::AlphaNumeric, 32)
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

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
pub enum SpecType {
    Service
}

pub trait Serializable: Sized {
    fn name(&self) -> &str;

    fn to_binary(&self) -> Vec<u8>;

    fn from_binary(bin: &[u8]) -> Option<Self>;

    fn sanity_check(&self) -> bool;

    fn version() -> u16;

    fn spec_type() -> SpecType;
}

pub trait Encryptor: Serialize + for <'a> Deserialize<'a> {
    fn encrypt<T: Serializable>(key: &[u8], obj: &T) -> Self;

    fn decrypt<T: Serializable>(&self, key: &[u8]) -> Option<T>;

    fn get_spec_type(&self) -> SpecType;

    fn get_version(&self) -> u16;
}

pub fn save<T: Serializable, E: Encryptor>(file: &mut File, key: &[u8], service: &T) {
    let encrypted = E::encrypt(key, service);
    let data = bincode::serialize(&encrypted).unwrap();
    file.write_all(&data).unwrap();
}

/*
pub fn get_spec_version(file: &mut File) -> Result<(SpecType, u16), APError> {
    let mut buffer = vec![];
    file.read_to_end(&mut buffer)?;
    let encoded = bincode::deserialize(&buffer)?;
}
*/

pub fn load<T: Serializable, E: Encryptor>(file: &mut File, key: &[u8]) -> Result<T, APError> {
    let mut buffer = vec![];
    file.read_to_end(&mut buffer)?;
    let encoded: E = bincode::deserialize(&buffer)?;
    match encoded.decrypt::<T>(key) {
        Some(entry) => {
            match entry.sanity_check() {
                true => Ok(entry),
                false => Err(APError::PasswordIncorrect)
            }
        },
        None => Err(APError::Decryption)
    }
}