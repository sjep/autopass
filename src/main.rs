mod api;
mod pass;
mod service;
mod hash;


fn main() {
    let res = api::new("hi", "havsi", &hash::TextMode::NoWhiteSpace, 16, vec![], None);
    if let Err(errs) = res {
        println!("{}", errs);
    }

    let res = api::get("hi", "havsi", true);
    println!("{:?}", res);

    for name in api::list("havsi").iter() {
        println!("{}", name);
    }
}
