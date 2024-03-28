use std::{fs::File, path::Path};

use thiserror::Error;

use crate::{api::{self, APError}, spec::{self, base_path, filename, load, save, Encryptor, Serializable}};


#[derive(Error, Debug)]
pub enum APUpgradeError {
    #[error("Internal AP error: {0}")]
    APError(#[from] APError),
    #[error("In Progress")]
    InProgress
}

pub fn list<P: AsRef<Path>, E: Encryptor, T: Serializable>(path: P, key: &[u8]) -> Vec<String> {
    let path = path.as_ref();
    if !path.exists() {
        return vec![];
    }
    let mut names: Vec<String> = vec![];
    for fbuf in std::fs::read_dir(path).unwrap() {
        let filename = fbuf.unwrap();
        if filename.file_type().unwrap().is_dir() {
            continue;
        }
        let mut file = File::open(filename.path()).unwrap();
        if let Ok(entry) = spec::load::<T, E>(&mut file, &key) {
            names.push(entry.name().to_string());
        }
    }
    names.sort();
    names
}

pub fn upgrade_encryptor<O: Encryptor, N: Encryptor, T: Serializable>(pass: &str) -> Result<(), APUpgradeError> {
    let key = api::create_key(pass);
    let legacy_dir = base_path().join("legacy");
    std::fs::create_dir_all(&legacy_dir).unwrap();
    let inprogress = legacy_dir.read_dir().unwrap().next().is_some();
    if inprogress {
        return Err(APUpgradeError::InProgress);
    }
    for objname in &list::<_, O, T>(base_path(), &key) {
        let objfilename = filename(objname);
        let newobjpath = base_path().join(&objfilename);
        let oldobjpath = legacy_dir.clone().join(&objfilename);
        std::fs::rename(&newobjpath, &oldobjpath).unwrap();

        let mut oldfile = File::open(&oldobjpath).unwrap();
        let entry = load::<T, O>(&mut oldfile, &key)?;
        let mut newfile = File::create(&newobjpath).unwrap();
        save::<T, N>(&mut newfile, &key, &entry);
        std::fs::remove_file(&oldobjpath).unwrap();
        println!("Saved entry {}", entry.name());
    }

    let done = legacy_dir.read_dir().unwrap().next().is_none();
    if done {
        std::fs::remove_dir(&legacy_dir).unwrap();
    }
    Ok(())
}