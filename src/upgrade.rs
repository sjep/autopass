use std::{fs::File, io::Seek, path::PathBuf};

use thiserror::Error;

use crate::{api::{self, APError}, spec::{base_path, identity_v1::IdentityV1, identity_v2::IdentityV2, load, load_header, save, service_v1::ServiceEntryV1, service_v2::ServiceEntryV2, Encryptor, EncryptorType, Serializable, SpecType}};


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
        if try_open::<N, T>(objname, &N::genkey(pass)) {
            println!("Skipping {}", objname);
            continue;
        }
        let newkey = N::genkey(pass);
        let oldkey = O::genkey(pass);
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

fn upgrade_spec<E: Encryptor, O: Serializable, N: Serializable + From<O>>(file: &mut File, key: &[u8]) -> Result<(), APError> {
    let old = load::<O, E>(file, key)?;
    let new = N::from(old);
    file.set_len(0)?;
    file.seek(std::io::SeekFrom::Start(0))?;
    save(file, key, &new)?;
    Ok(())
}

pub fn check_upgrade<E: Encryptor>(filename: &PathBuf, key: &[u8]) -> Result<(), APError> {
    
    let header = {
        let mut file = File::open(&filename)?;
        load_header(&mut file)?
    };

    let mut file = File::options()
        .read(true)
        .write(true)
        .open(&filename)?;

    match header.spec_type {
        SpecType::Service => match header.spec_version {
            1 => upgrade_spec::<EncryptorType, ServiceEntryV1, ServiceEntryV2>(&mut file, key),
            2 => Ok(()),
            _ => Err(APError::VersionTooOld)
        }
        SpecType::Identity => match header.spec_version {
            1 => upgrade_spec::<EncryptorType, IdentityV1, IdentityV2>(&mut file, key),
            2 => Ok(()),
            _ => Err(APError::VersionTooOld)
        }
    }
}