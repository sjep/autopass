use std::collections::HashMap;
use std::path::Path;
use std::fs::{File, read_dir};

use crate::service::{ServiceEntry, full_path, PASS_PATH};
use crate::hash::{HashAlg, get_digest, bin_to_str, TextMode};


fn create_key(pass: &str) -> Vec<u8> {
    let mut digest = get_digest(HashAlg::SHA256);
    let mut key = vec![0; digest.output_bytes()];
    digest.input(pass.as_bytes());
    digest.result(&mut key);
    key
}


pub fn new(name: &str,
           pass: &str,
           text_mode: &TextMode,
           len: usize,
           kvs: Vec<(&str, &str)>,
           service_pass: Option<&str>) -> Result<(), String> {

    if full_path(name).exists() {
        return Err(format!("Service '{}' already exists", name))
    }

    let nonce: u8 = 0;
    let mut digest = get_digest(HashAlg::SHA256);
    let mut h1 = vec![0; digest.output_bytes()];
    let h2 = create_key(pass);
    digest.input(name.as_bytes());
    digest.result(&mut h1);
    digest.reset();
    let password = match service_pass {
        Some(s) => s.to_string(),
        None => {
            let mut pwbin = vec![0; digest.output_bytes()];
            digest.input(std::slice::from_ref(&nonce));
            digest.input(&h1);
            digest.input(&h2);
            digest.result(&mut pwbin);
            digest.reset();
            bin_to_str(&pwbin, text_mode, len)
        }
    };

    let entry = ServiceEntry::new(
        name,
        &password,
        nonce,
        kvs,
        len,
        text_mode
    );
    entry.save(&h2);
    Ok(())
}

pub fn get(name: &str,
           pass: &str,
           clipboard: bool) -> Result<Option<String>, &'static str> {
    let filename = full_path(name);
    if !filename.exists() {
        return Err("Service doesn't exist");
    }

    let mut file = match File::open(filename) {
        Err(_) => return Err("Error opening file"),
        Ok(f) => f
    };
    let key = create_key(pass);

    let entry = match ServiceEntry::load(&mut file, &key) {
        Ok(entry) => entry,
        Err(s) => return Err(s)
    };
    Ok(match entry.get_pass(clipboard) {
        Some(pass) => Some(pass.to_string()),
        None => None
    })
}

pub fn list(pass: &str) -> Vec<String> {
    let dir = Path::new(PASS_PATH);
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
