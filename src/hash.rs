use crypto::digest::Digest;
use crypto::sha3::Sha3;
use crypto::sha2::Sha256;
use crypto::md5::Md5;

use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub enum HashAlg {
    MD5,
    SHA256,
    SHA3
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum TextMode {
    AlphaNumeric,
    AlphaNumericUnderscore,
    NoWhiteSpace
}

fn set_alphanumeric(map: &mut Vec<u8>) {
    for i in 48..(48 + 10) {
        map.push(i);
    }
    for i in 65..(65 + 26) {
        map.push(i);
    }
    for i in 97..(97 + 26) {
        map.push(i);
    }
}

fn set_alphanumeric_underscore(map: &mut Vec<u8>) {
    set_alphanumeric(map);
    map.push(95);
}

fn set_no_whitespace(map: &mut Vec<u8>) {
    for i in 48..127 {
        map.push(i);
    }
}

pub fn bin_to_str(data: &[u8], text_mode: &TextMode, len: u8) -> String {
    assert!(len as usize <= data.len());
    let mut map: Vec<u8> = vec![];
    match text_mode {
        TextMode::AlphaNumeric => {
            set_alphanumeric(&mut map);
        },
        TextMode::AlphaNumericUnderscore => {
            set_alphanumeric_underscore(&mut map);
        },
        TextMode::NoWhiteSpace => {
            set_no_whitespace(&mut map);
        }
    }

    let count = map.len() as f64;
    let mut res: Vec<u8> = vec![];
    for i in data[0..(len as usize)].iter() {
        res.push(map[((*i as f64 / 256.0) * count) as usize]);
    }
    std::str::from_utf8(&res).unwrap().to_string()
}

pub fn get_digest(hash_alg: HashAlg) -> Box<dyn Digest> {
    match hash_alg {
        HashAlg::MD5 => Box::new(Md5::new()),
        HashAlg::SHA256 => Box::new(Sha256::new()),
        HashAlg::SHA3 => Box::new(Sha3::sha3_256())
    }
}
