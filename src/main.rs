use crypto::digest::Digest;
use crypto::sha3::Sha3;

use clipboard::ClipboardProvider;
use clipboard::osx_clipboard::OSXClipboardContext;

mod api;
mod pass;
mod service;
mod hash;

fn test1() {
    let pass = pass::gen_pass("Hello, world!");
    let mut clipboard = OSXClipboardContext::new().unwrap();
    clipboard.set_contents(pass.to_string());
}
    
fn test2() {

    let mut hasher = Box::new(Sha3::sha3_256());
    hasher.input_str("abc");
    println!("{}", hasher.output_bytes());
    let hex = hasher.result_str();
    println!("{}", hex.len());
}


fn main() {
    let res = api::new("hi", "havsi", &hash::TextMode::NoWhiteSpace, 16, vec![], None);
    if let Err(errs) = res {
        println!("{}", errs);
    }

    let res = api::get("hi", "havsi", true);
    println!("{:?}", res);

    let res = api::list("havsi");
}
