use std::fs::File;

use thiserror::Error;

use crate::{api::{self, APError}, spec::{base_path, load, save, Encryptor, Serializable}};


#[derive(Error, Debug)]
pub enum APUpgradeError {
    #[error("Internal AP error: {0}")]
    APError(#[from] APError),
    #[error("In Progress")]
    InProgress
}

fn try_open<O: Encryptor, T: Serializable>(objname: &str, key: &[u8]) -> bool {
    let path = O::full_path(key, objname);
    File::open(&path).ok().map(|mut f| {
        load::<T, O>(&mut f, &key).ok()
    }).flatten().is_some()
}

pub fn upgrade_encryptor<O: Encryptor, N: Encryptor, T: Serializable>(pass: &str) -> Result<(), APUpgradeError> {
    let legacy_dir = base_path().join("legacy");
    std::fs::create_dir_all(&legacy_dir).unwrap();
    let inprogress = legacy_dir.read_dir().unwrap().next().is_some();
    if inprogress {
        return Err(APUpgradeError::InProgress);
    }
    for objname in &api::list(pass)? {
        if try_open::<N, T>(objname, &N::key(pass)) {
            println!("Skipping {}", objname);
            continue;
        }
        let newkey = N::key(pass);
        let oldkey = O::key(pass);
        let newfilename = N::filename(&newkey, objname);
        let oldfilename = O::filename(&oldkey, objname);
        let newobjpath = base_path().join(&newfilename);
        let oldobjpath = legacy_dir.clone().join(&oldfilename);
        std::fs::rename(&base_path().join(oldfilename), &oldobjpath).unwrap();

        let mut oldfile = File::open(&oldobjpath).unwrap();
        let entry = load::<T, O>(&mut oldfile, &oldkey)?;
        let mut newfile = File::create(&newobjpath).unwrap();
        save::<T>(&mut newfile, &newkey, &entry)?;
        std::fs::remove_file(&oldobjpath).unwrap();
        println!("Saved entry {}", entry.name());
    }

    let done = legacy_dir.read_dir().unwrap().next().is_none();
    if done {
        std::fs::remove_dir(&legacy_dir).unwrap();
    }
    Ok(())
}