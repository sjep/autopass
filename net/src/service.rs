use std::{net::IpAddr, path::Path, sync::{Arc, Mutex}};

use log::info;
use serde::{Deserialize, Serialize};

use crate::{discovery::PeerList, keypair::KeyPair, trust::{Identity, IdentityList}, ErrorIdentity};



pub struct IdentityService {
    keypair: KeyPair,
    peerlist: Arc<Mutex<PeerList>>,
    trustlist: IdentityList
}

impl IdentityService {
    pub fn new(name: &str, certdir: &Path) -> Result<Self, ErrorIdentity> {
        std::fs::create_dir_all(certdir)?;
        let keypair = if KeyPair::exists(certdir) {
            KeyPair::load(certdir).map(|mut kp| {
                let days_left = kp.cert().days_left()?;
                if days_left < 3 {
                    kp.refresh_cert()?;
                }
                Ok(kp)
            })?
        } else {
            KeyPair::new(name).map(|kp| {
                kp.save(certdir).and(Ok(kp))
            })?
        }?;

        let trustlist = if IdentityList::exists(certdir) {
            IdentityList::load(certdir)?
        } else {
            IdentityList::new()
        };

        let peerlist = PeerList::new(vec![], keypair.cert().to_bytes()?)?;
        Ok(IdentityService { keypair, peerlist, trustlist })
    }

    pub fn list_available(&self) -> Result<Vec<(IpAddr, Identity)>, ErrorIdentity> {
        let current = self.peerlist.lock().unwrap().list()?;
        let mut list = vec![];
        for (ip, cert) in current {
            let pubkey = match cert.extract_pubkey() {
                Ok(pk) => pk,
                Err(e) => {
                    info!("Error extracting public key for IP {}: {}", ip, e);
                    continue
                }
            };
            
            cert.extract_name()
                .map(|nint| nint.as_ref().to_owned())
                .map(|name| {
                    let id = Identity::new(name, pubkey);
                    list.push((ip, id))
                });
        }
        Ok(list)
    }
}