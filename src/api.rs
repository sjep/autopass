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
    #[error("Wrong type, wanted {0:?} but got {1:?}")]
    WrongType(SpecType, SpecType),
    #[error("Wrong version, wanted {0} but got {1}")]
    WrongVersion(u16, u16)
}

type ServiceType = crate::spec::service_v1::ServiceEntryV1;
type EncryptorType = crate::spec::encryptor::Encrypt;


pub fn exists(pass: &str, name: &str) -> bool {
    EncryptorType::full_path(pass, name).exists()
}

pub fn create_key(pass: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(pass.as_bytes());
    hasher.finalize().to_vec()
}

pub fn generate_pass(name: &str,
                     pass: &str,
                     nonce: u8,
                     len: u8,
                     text_mode: &TextMode) -> String {
    let mut digest = Sha256::new();
    digest.update(name.as_bytes());
    let h1 = digest.finalize();
    let h2 = create_key(pass);

    let mut digest = Sha256::new();
    digest.update(std::slice::from_ref(&nonce));
    digest.update(&h1);
    digest.update(&h2);
    let pwbin = digest.finalize();
    bin_to_str(&pwbin, text_mode, len)
}

fn load_entry(name: &str, pass: &str) -> Result<ServiceType, APError> {
    if !exists(pass, name) {
        return Err(APError::NotExist(name.to_owned()));
    }

    let filename = EncryptorType::full_path(pass, name);
    let mut file = File::open(filename)?;
    let key = create_key(pass);

    load::<ServiceType, EncryptorType>(&mut file, &key)
}

pub fn new<T: AsRef<str>>(
    name: &str,
    pass: &str,
    text_mode: &TextMode,
    len: u8,
    kvs: &[(T, T)],
    service_pass: Option<&str>) -> Result<ServiceType, APError>
{

    if exists(pass, name) {
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
    let h2 = create_key(pass);
    let path = base_path();
    std::fs::create_dir_all(path)?;
    let full_path = EncryptorType::full_path(pass, entry.get_name());
    let mut file = File::create(full_path)?;
    save::<ServiceType, EncryptorType>(&mut file, &h2, &entry);
    Ok(entry)
}

pub fn get(name: &str,
           pass: &str,
           clipboard: bool) -> Result<Option<String>, APError> {
    let entry = load_entry(&name, &pass)?;
    Ok(match entry.get_pass(clipboard) {
        Some(pass) => Some(pass.to_string()),
        None => None
    })
}

pub fn get_all(name: &str,
               pass: &str) -> Result<ServiceType, APError> {
    load_entry(name, pass)
}

pub fn set_kvs(name: &str,
               pass: &str,
               kvs: &[(&str, &str)],
               reset: bool) -> Result<(), APError> {
    let mut entry = load_entry(&name, &pass)?;
    entry.set_kvs(kvs, reset);
    let full_path = EncryptorType::full_path(pass, entry.get_name());
    let mut file = File::create(full_path)?;
    save::<ServiceType, EncryptorType>(&mut file, &create_key(pass), &entry);
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

pub fn list(pass: &str) -> Vec<String> {
    let dir = base_path();
    if !dir.exists() {
        return vec![];
    }
    let mut names: Vec<String> = vec![];
    for fbuf in read_dir(dir).unwrap() {
        let filename = fbuf.unwrap();
        if filename.file_type().unwrap().is_dir() {
            continue;
        }
        let mut file = File::open(filename.path()).unwrap();
        let key = create_key(pass);
        if let Ok(entry) = load::<ServiceType, EncryptorType>(&mut file, &key) {
            names.push(entry.get_name().to_string());
        }
    }
    names.sort();
    names
}

pub fn upgrade(name: &str,
               pass: &str,
               service_pass: Option<&str>) -> Result<(String, String), APError> {
    match load_entry(&name, &pass) {
        Ok(mut entry) => {

            let new_pass = match service_pass {
                Some(s) => s.to_string(),
                None => {
                    let nonce = entry.uptick();
                    generate_pass(name, pass, nonce, entry.get_len(),  entry.get_text_mode())
                }
            };
            let old_pass = entry.get_pass(false).unwrap().to_string();
            entry.set_pass(&new_pass);
            let full_path = EncryptorType::full_path(pass, entry.get_name());
            let mut file = File::create(full_path).unwrap();
            save::<ServiceType, EncryptorType>(&mut file, &create_key(pass), &entry);
            Ok((old_pass, new_pass))
        },
        Err(s) => {
            Err(s)
        }
    }
}

pub fn delete(name: &str, pass: &str) -> Result<(), String> {
   match remove_file(EncryptorType::full_path(pass, name)) {
       Ok(_) => Ok(()),
       Err(e) => Err(e.to_string())
   }
}