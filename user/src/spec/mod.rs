use std::{fs::{read_dir, File}, io::{Read, Write}, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};
use time::{format_description, OffsetDateTime, UtcOffset};

use crate::api::APError;

pub mod service_v1;
pub mod service_v2;
pub mod identity_v1;
pub mod identity_v2;
pub mod encryptor;

pub const PASS_PATH: &'static str = ".pass";
pub const PASS_BASE_ENVVAR: &'static str = "AP_BASEDIR";
const IDENTITY_MAGIC: u32 = 0xfedb1234;
const SERVICE_MAGIC: u32 = 0x83596235;
const IDENTITY_FNAME: &str = ".apid";

pub type EncryptorType = crate::spec::encryptor::Encrypt;
pub type IdentityType = identity_v2::IdentityV2;
pub type ServiceType = service_v2::ServiceEntryV2;

pub fn base_path() -> PathBuf {
    if let Ok(basepath) = std::env::var(PASS_BASE_ENVVAR) {
        return basepath.into();
    }
    Path::join(&dirs::home_dir().unwrap(), Path::new(PASS_PATH))
}

pub fn identity_path<P: AsRef<Path>>(basedir: P) -> PathBuf {
    Path::join(basedir.as_ref(), IDENTITY_FNAME)
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone, PartialEq)]
pub enum SpecType {
    Service,
    Identity
}

pub trait Serializable: Sized {
    fn name(&self) -> &str;

    fn to_binary(&self) -> Vec<u8>;

    fn from_binary(bin: &[u8]) -> Option<Self>;

    fn sanity_check(&self) -> bool;

    fn version(&self) -> u16;

    fn spec_type(&self) -> SpecType;
}

pub type APKey = [u8; 32];

pub trait Encryptor: Serialize + for <'a> Deserialize<'a> {
    fn encrypt<T: Serializable>(key: &[u8], obj: &T) -> Self;

    fn decrypt<T: Serializable>(&self, key: &[u8]) -> Option<T>;

    fn encrypt_version() -> u16;

    fn genkey(pass: &str) -> APKey;

    fn filename(key: &[u8], name: &str) -> String;

    fn full_path(key: &[u8], name: &str) -> PathBuf {
        Path::join(&base_path(), Path::new(&Self::filename(key, name)))
    }
}

fn now() -> u64 {
    OffsetDateTime::now_utc().unix_timestamp() as u64
}

fn timestamp_as_string(ts: u64) -> String {
    let dtutc = OffsetDateTime::from_unix_timestamp(ts as i64)
        .unwrap();
    let utc = match UtcOffset::current_local_offset() {
        Ok(offset) => {
            dtutc.to_offset(offset);
            false
        }
        Err(_e) => {
            true
        }
    };
    let mut dtstr = dtutc.format(&format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap())
        .unwrap();
    if utc {
        dtstr = format!("{} UTC", dtstr);
    }
    dtstr
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Header {
    pub spec_type: SpecType,
    pub spec_version: u16,
    pub encrypt_version: u16
}

const HEADER_SIZE: usize = 8;

impl Header {
    fn create<T: Serializable, E: Encryptor>(entry: &T) -> Self {
        Self {
            spec_type: entry.spec_type(),
            spec_version: entry.version(),
            encrypt_version: E::encrypt_version()
        }
    }
}

pub fn save<T: Serializable>(file: &mut File, key: &[u8], service: &T) -> Result<(), APError> {
    assert!(file.metadata()?.len() == 0);
    let encrypted = EncryptorType::encrypt(key, service);
    let header = Header::create::<T, EncryptorType>(service);
    let headerdata = bincode::serialize(&header)?;
    assert!(headerdata.len() == HEADER_SIZE);
    let data = bincode::serialize(&encrypted)?;
    file.write_all(&headerdata).unwrap();
    file.write_all(&data).unwrap();
    Ok(())
}

pub fn load_header(file: &mut File) -> Result<Header, APError> {
    let mut data = [0u8; 8];
    file.read_exact(&mut data).unwrap();
    let header = bincode::deserialize::<Header>(&data[0..HEADER_SIZE])?;
    Ok(header)
}

pub fn load<T: Serializable, E: Encryptor>(file: &mut File, key: &[u8]) -> Result<T, APError> {
    let mut data = vec![];
    file.read_to_end(&mut data).unwrap();
    let header = bincode::deserialize::<Header>(&data[0..HEADER_SIZE])?;
    if header.encrypt_version != E::encrypt_version() {
        return Err(APError::WrongEncryptVersion(E::encrypt_version(), header.encrypt_version));
    }

    let encoder = bincode::deserialize::<EncryptorType>(&data[HEADER_SIZE..data.len()])?;
    match encoder.decrypt::<T>(key) {
        Some(entry) => {
            match entry.sanity_check() {
                true => Ok(entry),
                false => Err(APError::PasswordIncorrect)
            }
        },
        None => Err(APError::Decryption)
    }
}

pub fn list<P: AsRef<Path>>(basedir: P, by_spec: Option<SpecType>, by_version: Option<u16>) -> Result<Vec<PathBuf>, APError> {
    let dir = basedir.as_ref();
    let mut data = [0u8; HEADER_SIZE];
    let mut entries = vec![];

    if !dir.exists() {
        return Ok(entries);
    }
    for fbuf in read_dir(dir)? {
        let filename = fbuf?;
        if filename.file_type()?.is_dir() {
            continue;
        }
        if filename.path().starts_with(".") {
            continue;
        }
        let mut file = File::open(filename.path())?;
        file.read_exact(&mut data)?;
        let header = bincode::deserialize::<Header>(&data[0..HEADER_SIZE])?;
        if let Some(spec) = by_spec {
            if spec != header.spec_type {
                continue;
            }
        }
        if let Some(version) = by_version {
            if version != header.spec_version {
                continue;
            }
        }
        entries.push(filename.path());
    }
    Ok(entries)
}

impl From<self::identity_v1::IdentityV1> for self::identity_v2::IdentityV2 {
    fn from(value: self::identity_v1::IdentityV1) -> Self {
        let mut kv = vec![];
        for (k, v) in value.kv {
            kv.push((k, v));
        }
        kv.sort();
        Self {
            magic: value.magic,
            name: value.name,
            key: value.key,
            kv,
            create_time: value.create_time,
            modify_time: value.modify_time,
        }
    }
}

impl From<self::service_v1::ServiceEntryV1> for self::service_v2::ServiceEntryV2 {
    fn from(value: self::service_v1::ServiceEntryV1) -> Self {
        let mut kv = vec![];
        for (k, v) in value.kv {
            kv.push((k, v));
        }
        kv.sort();
        Self {
            magic: SERVICE_MAGIC,
            name: value.name,
            pass: value.pass,
            nonce: value.nonce,
            kv,
            tags: Vec::new(),
            len: value.len,
            text_mode: value.text_mode,
            create_time: value.create_time,
            modify_time: value.modify_time,
        }
    }
}