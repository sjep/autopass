use std::fs::{File, read_dir, remove_file};

use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::spec::{base_path, load, save, Encryptor, SpecType};
use crate::hash::{bin_to_str, TextMode};



#[derive(Error, Debug)]
pub enum APError {
    #[error("Io Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] Box<bincode::ErrorKind>),
    #[error("Entry {0} already exists")]
    Exists(String),
    #[error("Entry {0} doesn't exist")]
    NotExist(String),
    #[error("Error during decryption")]
    Decryption,
    #[error("Password incorrect")]
    PasswordIncorrect,
    #[error("Wrong encryptionversion, wanted {0} but got {1}")]
    WrongEncryptVersion(u16, u16),
}

type ServiceType = crate::spec::service_v1::ServiceEntryV1;
type EncryptorType = crate::spec::encryptor::Encrypt;


fn exists_int(key: &[u8], name: &str) -> bool {
    EncryptorType::full_path(key, name).exists()
}

pub fn exists(pass: &str, name: &str) -> bool {
    let key = EncryptorType::key(pass);
    exists_int(&key, name)
}

pub fn generate_pass(name: &str,
                     pass: &str,
                     nonce: u8,
                     len: u8,
                     text_mode: &TextMode) -> String {
    let mut digest = Sha256::new();
    digest.update(name.as_bytes());
    let h1 = digest.finalize();
    let h2 = EncryptorType::key(pass);

    let mut digest = Sha256::new();
    digest.update(std::slice::from_ref(&nonce));
    digest.update(&h1);
    digest.update(&h2);
    let pwbin = digest.finalize();
    bin_to_str(&pwbin, text_mode, len)
}

fn load_entry(name: &str, pass: &str) -> Result<(ServiceType, Vec<u8>), APError> {
    let key = EncryptorType::key(pass);
    if !exists_int(&key, name) {
        return Err(APError::NotExist(name.to_owned()));
    }

    let filename = EncryptorType::full_path(&key, name);
    let mut file = File::open(filename)?;

    Ok((load::<ServiceType, EncryptorType>(&mut file, &key)?, key))
}

pub fn new<T: AsRef<str>>(
    name: &str,
    pass: &str,
    text_mode: &TextMode,
    len: u8,
    kvs: &[(T, T)],
    service_pass: Option<&str>) -> Result<ServiceType, APError>
{
    let key = EncryptorType::key(pass);

    if exists_int(&key, name) {
        return Err(APError::Exists(name.to_owned()))
    }

    let password = match service_pass {
        None => generate_pass(name, pass, 0u8, len, text_mode),
        Some(s) => s.to_string()
    };

    let entry = ServiceType::new(
        name,
        &password,
        0u8,
        kvs,
        len,
        text_mode
    );
    let path = base_path();
    std::fs::create_dir_all(path)?;
    let full_path = EncryptorType::full_path(&key, entry.get_name());
    let mut file = File::create(full_path)?;
    save::<ServiceType>(&mut file, &key, &entry)?;
    Ok(entry)
}

pub fn get(name: &str,
           pass: &str,
           clipboard: bool) -> Result<Option<String>, APError> {
    let (entry, _key) = load_entry(&name, &pass)?;
    Ok(match entry.get_pass(clipboard) {
        Some(pass) => Some(pass.to_string()),
        None => None
    })
}

pub fn get_all(name: &str,
               pass: &str) -> Result<ServiceType, APError> {
    load_entry(name, pass).map(|(entry, _k)| entry)
}

pub fn set_kvs(name: &str,
               pass: &str,
               kvs: &[(&str, &str)],
               reset: bool) -> Result<(), APError> {
    let (mut entry, key) = load_entry(&name, &pass)?;
    entry.set_kvs(kvs, reset);
    let full_path = EncryptorType::full_path(&key, entry.get_name());
    let mut file = File::create(full_path)?;
    save::<ServiceType>(&mut file, &EncryptorType::key(pass), &entry)?;
    Ok(())
}

pub fn empty() -> Result<bool, APError> {
    let dir = base_path();
    if !dir.exists() {
        return Ok(true);
    }
    Ok(read_dir(dir)?
        .nth(0)
        .is_none())
}

pub fn list(pass: &str) -> Result<Vec<String>, APError> {
    let dir = base_path();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut names: Vec<String> = vec![];
    for filename in &crate::spec::list(&dir, Some(SpecType::Service), None)? {
        let mut file = File::open(filename)?;
        let key = EncryptorType::key(pass);
        let entry = load::<ServiceType, EncryptorType>(&mut file, &key)?;
        names.push(entry.get_name().to_string());
    }
    names.sort();
    Ok(names)
}

pub fn upgrade(name: &str,
               pass: &str,
               service_pass: Option<&str>) -> Result<(String, String), APError> {
    match load_entry(&name, &pass) {
        Ok((mut entry, key)) => {

            let new_pass = match service_pass {
                Some(s) => s.to_string(),
                None => {
                    let nonce = entry.uptick();
                    generate_pass(name, pass, nonce, entry.get_len(),  entry.get_text_mode())
                }
            };
            let old_pass = entry.get_pass(false).unwrap().to_string();
            entry.set_pass(&new_pass);
            let full_path = EncryptorType::full_path(&key, entry.get_name());
            let mut file = File::create(full_path)?;
            save::<ServiceType>(&mut file, &key, &entry)?;
            Ok((old_pass, new_pass))
        },
        Err(s) => {
            Err(s)
        }
    }
}

pub fn delete(name: &str, pass: &str) -> Result<(), String> {
    let key = EncryptorType::key(pass);
    match remove_file(EncryptorType::full_path(&key, name)) {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string())
    }
}