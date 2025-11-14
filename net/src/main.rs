use std::{fs::File, path::Path};


use openssl::{error::ErrorStack, x509::X509};
use simplelog::{CombinedLogger, Config, SimpleLogger};


mod discovery;
mod keypair;
mod api;
mod service;
mod trust;


use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErrorIdentity {
    #[error("Filesystem error")]
    IO(#[from] std::io::Error),
    #[error("SSL error")]
    SSL(#[from] ErrorStack),
    #[error("Serialization error")]
    Bincode(#[from] bincode::Error)
}

struct IpLogger {}

impl discovery::IpEvent for IpLogger {
    fn ip_added(&mut self, addr: std::net::IpAddr, certbytes: &[u8]) {
        println!("New IP discovered: {}", addr);
        let cert = X509::from_pem(&certbytes).unwrap();
        println!("{:?}", cert.issuer_name())
    }

    fn ip_removed(&mut self, addr: std::net::IpAddr) {
        println!("Ip became inactive: {}", addr);
    }
}

pub fn create_or_truncate(p: &Path) -> Result<File, ErrorIdentity> {
    let mut oo = std::fs::OpenOptions::new();
    Ok(if std::fs::exists(p)? {
        let f = oo.write(true).open(&p)?;
        f.set_len(0)?;
        f
    } else {
        std::fs::File::create(&p)?
    })
}

fn main() -> Result<(), ErrorIdentity> {
    CombinedLogger::init(vec![
        SimpleLogger::new(simplelog::LevelFilter::Info, Config::default())
    ]).unwrap();

    //let is = IdentityService::new("Adam Schwab", Path::new("/tmp"))?;

    //loop {
    //    std::thread::sleep(Duration::from_secs(1));
    //}
    Ok(())
}
