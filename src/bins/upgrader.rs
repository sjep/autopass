use pass::spec::{encryptor::Encrypt, service_v1::ServiceEntryV1};


type Current = ServiceEntryV1;
type OldEncryptor = Encrypt;
type NewEncryptor = Encrypt;


fn main() {
    let pwd = pass::cli::read_pass();
    if let Err(e) = pass::upgrade::upgrade_encryptor::<OldEncryptor, NewEncryptor, Current>(&pwd) {
        eprintln!("Error upgrading: {}", e);
    }
}