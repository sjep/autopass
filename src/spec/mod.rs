use std::{fs::File, io::{Read, Write}, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

use crate::hash::{bin_to_str, get_digest, HashAlg, TextMode};

pub mod service_v1;
pub mod encryptor;
pub mod encryptor_legacy;

pub const PASS_PATH: &'static str = ".pass";
pub const PASS_BASE_ENVVAR: &'static str = "AP_BASEDIR";


pub fn filename(name: &str) -> String {
    let mut digest = get_digest(HashAlg::SHA256);
    let mut fbin = vec![0; digest.output_bytes()];
    digest.input(name.as_bytes());
    digest.result(&mut fbin);
    digest.reset();
    bin_to_str(&fbin, &TextMode::AlphaNumeric, 32)
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

#[derive(Deserialize, Serialize, Debug)]
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
}

pub fn save<T: Serializable, E: Encryptor>(file: &mut File, key: &[u8], service: &T) {
    let encrypted = E::encrypt(key, service);
    let data = bincode::serialize(&encrypted).unwrap();
    let full_path = full_path(service.name());
    let mut file = File::create(full_path).unwrap();
    file.write_all(&data).unwrap();
}

pub fn load<T: Serializable, E: Encryptor>(file: &mut File, key: &[u8]) -> Result<T, &'static str> {
    let mut buffer = vec![];
    file.read_to_end(&mut buffer).unwrap();
    let encoded: E = bincode::deserialize(&buffer).unwrap();
    match encoded.decrypt::<T>(key) {
        Some(entry) => {
            match entry.sanity_check() {
                true => Ok(entry),
                false => Err("Wrong password")
            }
        },
        None => Err("Wrong password")
    }
}