use std::io::{Read, Write};
use std::path::Path;

use openssl::asn1::Asn1Time;
use openssl::ec::{EcGroup, EcKey};
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use openssl::x509::{X509Name, X509};

use crate::{create_or_truncate, ErrorIdentity};

pub struct Cert {
    x509: X509
}

impl Cert {
    pub fn from_x509(x509: X509) -> Self {
        Self{x509}
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ErrorIdentity> {
        Ok(Self{x509: X509::from_pem(bytes)?})
    }

    pub fn extract_name(&self) -> Option<impl AsRef<str>> {
        self.x509.subject_name().entries_by_nid(Nid::ORGANIZATIONNAME).last().map(|entry| {
            entry.data().as_utf8().unwrap()
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ErrorIdentity> {
        Ok(self.x509.to_pem()?)
    }

    pub fn extract_pubkey(&self) -> Result<Vec<u8>, ErrorIdentity> {
        let pkey = self.x509.public_key()?;
       Ok(pkey.public_key_to_pem()?)
    }

    pub fn days_left(&self) -> Result<i32, ErrorIdentity> {
        let time = self.x509.not_after();
        let now = Asn1Time::days_from_now(0)?;
        let td = time.diff(&now)?;
        Ok(td.days)
    }
}


pub struct KeyPair {
    key: EcKey<Private>,
    cert: Cert
}

impl KeyPair {

    const KEY_FILE: &'static str = "key.pem";
    const CERT_FILE: &'static str = "cert.crt";

    pub fn new(name: &str) -> Result<Self, ErrorIdentity> {
        let ecg = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
        let eck = EcKey::generate(&ecg)?;
        
        let pkey = PKey::from_ec_key(eck.clone())?;
        let mut certbuilder = openssl::x509::X509Builder::new()?;
        certbuilder.set_pubkey(&pkey)?;
        let mut namebuilder = X509Name::builder()?;
        namebuilder.append_entry_by_nid(Nid::ORGANIZATIONNAME, name)?;
        let expire = Asn1Time::days_from_now(10)?;
        certbuilder.set_not_after(&expire)?;
        let notbefore = Asn1Time::days_from_now(0)?;
        certbuilder.set_not_before(&notbefore)?;
        certbuilder.set_subject_name(&namebuilder.build())?;
        certbuilder.sign(&pkey, MessageDigest::sha256())?;
    
        let cert = Cert::from_x509(certbuilder.build());
        
        Ok(Self {
            key: eck,
            cert
        })
    }

    pub fn refresh_cert(&mut self) -> Result<(), ErrorIdentity> {
        let pkey = PKey::from_ec_key(self.key.clone())?;
        let mut certbuilder = openssl::x509::X509Builder::new()?;
        certbuilder.set_pubkey(&pkey)?;
        let mut namebuilder = X509Name::builder()?;
        let name = self.cert.extract_name().unwrap();
        namebuilder.append_entry_by_nid(Nid::ORGANIZATIONNAME, name.as_ref())?;
        let expire = Asn1Time::days_from_now(10)?;
        certbuilder.set_not_after(&expire)?;
        let notbefore = Asn1Time::days_from_now(0)?;
        certbuilder.set_not_before(&notbefore)?;
        certbuilder.set_subject_name(&namebuilder.build())?;
        certbuilder.sign(&pkey, MessageDigest::sha256())?;
    
        self.cert = Cert::from_x509(certbuilder.build());
        Ok(())
    }

    pub fn print(&self) -> Result<(), ErrorIdentity> {
        let pembytes = self.key.private_key_to_pem()?;
        println!("Key:\n{}", String::from_utf8(pembytes).unwrap());
        let certpembytes = self.cert.to_bytes()?;
        println!("Cert:\n{}", String::from_utf8(certpembytes).unwrap());
        Ok(())
    }

    pub fn save(&self, certdir: &Path) -> Result<(), ErrorIdentity> {
        let mut oo = std::fs::OpenOptions::new();
        let keypath = certdir.join(Self::KEY_FILE);
        let mut keyfile = create_or_truncate(&keypath)?;
        keyfile.write_all(&self.key.private_key_to_pem()?)?;

        let certpath = certdir.join(Self::CERT_FILE);

        let mut certfile = if std::fs::exists(&certpath)? {
            let f = oo.write(true).open(&certpath)?;
            f.set_len(0)?;
            f
        } else {
            std::fs::File::create(&certpath)?
        };
        certfile.write_all(&self.cert.to_bytes()?)?;
        Ok(())
    }

    pub fn exists(certdir: &Path) -> bool {
        let keypath = certdir.join(Self::KEY_FILE);
        std::fs::exists(keypath).unwrap_or(false)
    }

    pub fn load(certdir: &Path) -> Result<Self, ErrorIdentity> {
        let keypath = certdir.join(Self::KEY_FILE);
        let mut keyfile = std::fs::File::open(keypath)?;
        let mut pembytes: Vec<u8> = Vec::new();
        keyfile.read_to_end(&mut pembytes)?;

        let key = EcKey::private_key_from_pem(&pembytes)?;

        let certpath = certdir.join(Self::CERT_FILE);
        let mut certfile = std::fs::File::open(certpath)?;
        let mut certbytes: Vec<u8> = Vec::new();
        certfile.read_to_end(&mut certbytes)?;
        let cert = Cert::from_x509(X509::from_pem(&certbytes)?);
        return Ok(Self{key, cert})
    }

    pub fn cert(&self) -> &Cert {
        &self.cert
    }
}