use std::{fs::File, io::Write, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

use crate::api::APError;

pub mod service_v1;
pub mod encryptor;

pub const PASS_PATH: &'static str = ".pass";
pub const PASS_BASE_ENVVAR: &'static str = "AP_BASEDIR";

type EncryptorType = crate::spec::encryptor::Encrypt;

pub fn base_path() -> PathBuf {
    if let Ok(basepath) = std::env::var(PASS_BASE_ENVVAR) {
        return basepath.into();
    }
    Path::join(&dirs::home_dir().unwrap(), Path::new(PASS_PATH))
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone, PartialEq)]
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

    fn encrypt_version() -> u16;

    fn key(pass: &str) -> Vec<u8>;

    fn filename(key: &[u8], name: &str) -> String;

    fn full_path(key: &[u8], name: &str) -> PathBuf {
        Path::join(&base_path(), Path::new(&Self::filename(key, name)))
    }
}

pub fn save<T: Serializable>(file: &mut File, key: &[u8], service: &T) -> Result<(), APError> {
    let encrypted = EncryptorType::encrypt(key, service);
    let data = bincode::serialize(&(T::spec_type(), T::version(), EncryptorType::encrypt_version(), &encrypted)).unwrap();
    file.write_all(&data)?;
    Ok(())
}

pub fn get_spec_version(file: &mut File) -> Result<(SpecType, u16, u16), APError> {
    let (spec, version, encrypt_version) = bincode::deserialize_from(file)?;
    Ok((spec, version, encrypt_version))
}

pub fn load<T: Serializable, E: Encryptor>(file: &mut File, key: &[u8]) -> Result<T, APError> {

    let (spec, version, encrypt_version, encoded): (SpecType, u16, u16, E) = bincode::deserialize_from(file)?;
    if spec != T::spec_type() {
        return Err(APError::WrongType(T::spec_type(), spec));
    }
    if version != T::version() {
        return Err(APError::WrongVersion(T::version(), version));
    }
    if encrypt_version != E::encrypt_version() {
        return Err(APError::WrongEncryptVersion(E::encrypt_version(), encrypt_version));
    }

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