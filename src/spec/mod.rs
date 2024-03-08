use serde::{Serialize, Deserialize};

pub mod service;

#[derive(Deserialize, Serialize, Debug)]
pub struct ServiceEncrypted {
    iv: [u8; 16],
    cyphertext: Vec<u8>
}