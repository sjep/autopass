use std::path::Path;
use std::fs::{File, read_dir, remove_file};

use crate::service::{ServiceEntry, full_path, base_path};
use crate::hash::{HashAlg, get_digest, bin_to_str, TextMode};


pub fn exists(name: &str) -> bool {
    full_path(name).exists()
}


fn create_key(pass: &str) -> Vec<u8> {
    let mut digest = get_digest(HashAlg::SHA256);
    let mut key = vec![0; digest.output_bytes()];
    digest.input(pass.as_bytes());
    digest.result(&mut key);
    key
}

fn generate_pass(name: &str,
                 pass: &str,
                 nonce: u8,
                 len: u8,
                 text_mode: &TextMode) -> String {
    let mut digest = get_digest(HashAlg::SHA256);
    let mut h1 = vec![0; digest.output_bytes()];
    digest.input(name.as_bytes());
    digest.result(&mut h1);
    digest.reset();
    let h2 = create_key(pass);
    let mut pwbin = vec![0; digest.output_bytes()];
    digest.input(std::slice::from_ref(&nonce));
    digest.input(&h1);
    digest.input(&h2);
    digest.result(&mut pwbin);
    digest.reset();
    bin_to_str(&pwbin, text_mode, len)
}

fn load_entry(name: &str, pass: &str) -> Result<ServiceEntry, &'static str> {
    if !exists(name) {
        return Err("Service doesn't exist");
    }

    let filename = full_path(name);
    let mut file = match File::open(filename) {
        Err(_) => return Err("Error opening file"),
        Ok(f) => f
    };
    let key = create_key(pass);

    ServiceEntry::load(&mut file, &key)
}

pub fn new(name: &str,
           pass: &str,
           text_mode: &TextMode,
           len: u8,
           kvs: &[(&str, &str)],
           service_pass: Option<&str>) -> Result<ServiceEntry, String> {

    if exists(name) {
        return Err(format!("Service '{}' already exists", name))
    }

    let password = match service_pass {
        None => generate_pass(name, pass, 0u8, len, text_mode),
        Some(s) => s.to_string()
    };

    let entry = ServiceEntry::new(
        name,
        &password,
        0u8,
        kvs,
        len,
        text_mode
    );
    let h2 = create_key(pass);
    entry.save(&h2);
    Ok(entry)
}

pub fn get(name: &str,
           pass: &str,
           clipboard: bool) -> Result<Option<String>, &'static str> {
    let entry = match load_entry(&name, &pass) {
        Ok(entry) => entry,
        Err(s) => return Err(s)
    };
    Ok(match entry.get_pass(clipboard) {
        Some(pass) => Some(pass.to_string()),
        None => None
    })
}

pub fn get_all(name: &str,
               pass: &str) -> Result<ServiceEntry, &'static str> {
    load_entry(name, pass)
}

pub fn set_kvs(name: &str,
               pass: &str,
               kvs: &[(&str, &str)],
               reset: bool) -> Result<(), &'static str> {
    match load_entry(&name, &pass) {
        Ok(mut entry) => {
            entry.set_kvs(kvs, reset);
            entry.save(&create_key(pass));
            Ok(())
        },
        Err(s) => Err(s)
    }
}

pub fn list(pass: &str) -> Vec<String> {
    let dir = base_path();
    if !dir.exists() {
        return vec![];
    }
    let mut names: Vec<String> = vec![];
    for fbuf in read_dir(dir).unwrap() {
        let filename = fbuf.unwrap().path();
        let mut file = File::open(filename).unwrap();
        let key = create_key(pass);
        if let Ok(entry) = ServiceEntry::load(&mut file, &key) {
            names.push(entry.get_name().to_string());
        }
    }
    names
}

pub fn upgrade(name: &str,
               pass: &str,
               service_pass: Option<&str>) -> Result<(String, String), &'static str> {
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
            entry.save(&create_key(pass));
            Ok((old_pass, new_pass))
        },
        Err(s) => {
            Err(s)
        }
    }
}

pub fn delete(name: &str) -> Result<(), String> {
   match remove_file(full_path(name)) {
       Ok(_) => Ok(()),
       Err(e) => Err(e.to_string())
   }
}
