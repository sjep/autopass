use std::io::{stdin, stdout, Write};
use std::str::FromStr;

use clap::{Arg, App, SubCommand, ArgMatches};
use termion::input::TermRead;

use crate::api;
use crate::hash::TextMode;
use crate::spec::Serializable;


pub fn read_pass_raw(prompt: &str) -> String {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    stdout.write_all(prompt.as_bytes()).unwrap();
    stdout.flush().unwrap();

    let pass = stdin.read_passwd(&mut stdout).unwrap().unwrap();
    println!();
    pass
}

pub fn read_pass() -> String {
    read_pass_raw("password: ")
}

fn arg_name() -> Arg<'static, 'static> {
    Arg::with_name("name")
        .short("n")
        .long("name")
        .value_name("NAME")
        .help("Service name")
        .takes_value(true)
        .required(true)
}

fn arg_ident() -> Arg<'static, 'static> {
    Arg::with_name("identity")
        .short("i")
        .long("identity")
        .value_name("NAME")
        .help("Identifier for use in syncing with other users")
        .takes_value(true)
        .required(true)
}


fn arg_kvs() -> Arg<'static, 'static> {
    Arg::with_name("kvs")
         .long("key-value")
         .short("k")
         .help("Key-value pairs to be stored of the form -k KEY=VALUE")
         .multiple(true)
         .number_of_values(1)
}


fn arg_set_pass() -> Arg<'static, 'static> {
    Arg::with_name("set-password")
                   .long("set-password")
                   .value_name("PASS")
                   .help("Set a password instead of generating one")
                   .takes_value(true)
}

fn fetch_kvs<'a>(matches: &'a ArgMatches) -> Result<Vec<(&'a str, &'a str)>, &'static str> {
    let mut valid = true;
    let kvs: Vec<(&str, &str)> = match matches.values_of("kvs") {
        None => vec![],
        Some(v) => v.map(|elem| {
            let eqidx = match elem.find("=") {
                None => {
                    valid = false;
                    0
                },
                Some(idx) => idx
            };
            (&elem[0..eqidx], &elem[eqidx + 1..elem.len()])
        }).collect()
    };
    match valid {
        true => Ok(kvs),
        false => Err("Key value pairs must be of the form KEY=VALUE")
    }
}

fn init_cmd(matches: &ArgMatches) {
    let pwd = read_pass_raw("password: ");
    let pwdconfirm = read_pass_raw("re-enter password: ");
    if pwd != pwdconfirm {
        println!("Passwords don't match");
        return;
    }
    let name = matches.value_of("name").unwrap();
    let kvs: Vec<(&str, &str)> = match fetch_kvs(&matches) {
        Ok(k) => k,
        Err(s) => {
            println!("{}", s);
            return;
        }
    };
    match api::init(name, &pwd, &kvs) {
        Ok(res) => println!("Initialized ap with identity {}", res.name()),
        Err(e) => println!("Error initializing ap: {}", e)
    }
}

fn new_cmd(matches: &ArgMatches) {
    let pass = read_pass();

    let name = matches.value_of("name").unwrap();
    println!("Adding '{}' as new service", name);
    if api::exists(&pass, name) {
        println!("{} already exists", name);
        return;
    }

    let text_mode = match matches.value_of("text_mode").unwrap() {
        "alphanumeric" => TextMode::AlphaNumeric,
        "alphanumericunderscore" => TextMode::AlphaNumericUnderscore,
        "nowhitespace" => TextMode::NoWhiteSpace,
        _ => TextMode::NoWhiteSpace
    };

    let len: u8 = match usize::from_str(matches.value_of("length").unwrap()) {
        Err(_) => {
            println!("Length provided not an integer");
            return;
        },
        Ok(l) => {
            match l > 256 {
                true => {
                    println!("Max length allowed is 256");
                    return;
                },
                false => l as u8
            }
        }
    };
    let kvs: Vec<(&str, &str)> = match fetch_kvs(&matches) {
        Ok(k) => k,
        Err(s) => {
            println!("{}", s);
            return;
        }
    };

    let tags = match matches.values_of("kvs") {
        None => vec![],
        Some(vs) => vs.collect::<Vec<&str>>()
    };

    let set_password = matches.value_of("set-password");

    match api::new(name, &pass, &text_mode, len, &kvs, &tags, set_password)  {
        Ok(entry) => println!("New password created for service '{}':\n{}", name, entry.get_pass(false).unwrap()),
        Err(s) => println!("Error creating service: {}", s)
    };
}

fn get_id_cmd(_matches: &ArgMatches) {
    let pass = read_pass();

    match api::get_id(&pass) {
        Ok(id) => println!("{}", id),
        Err(s) => println!("Error getting id info: {}", s)
    }
}

fn get_cmd(matches: &ArgMatches) {
    let pass = read_pass();
    let name = matches.value_of("name").unwrap();
    if !api::exists(&pass, name) {
        println!("{} does not exist", name);
        return;
    }

    let clipboard = matches.is_present("clipboard");

    let all = matches.is_present("all");
    
    match all {
        false => {
            match api::get(name, &pass, clipboard) {
                Ok(opts) => match opts {
                    Some(p) => println!("{}", p),
                    None => println!("Copied to clipboard")
                },
                Err(s) => println!("Error getting service: {}", s)
            }
        },
        true => {
            match api::get_all(name, &pass) {
                Ok(entry) => println!("{}", entry),
                Err(s) => println!("Error getting service: {}", s)
            }
        }
    }
}

fn list_cmd(matches: &ArgMatches) {
    let pass = read_pass();
    if !matches.is_present("simple") {
        println!("\nServices\n--------");
    }
    match api::list(&pass) {
        Ok(items) => {
            for n in items {
                println!("{}", n);
            }
        }
        Err(e) => {
            println!("Error listing services: {}", e);
        }
    }
}

fn setkv_cmd(matches: &ArgMatches) {
    let pass = read_pass();
    let name = matches.value_of("name").unwrap();
    if !api::exists(&pass, name) {
        println!("{} does not exist", name);
        return;
    }
    let reset = matches.is_present("reset");
    match fetch_kvs(matches) {
        Err(s) => println!("{}", s),
        Ok(kvs) => {
            match api::set_kvs(name, &pass, &kvs, reset) {
                Err(s) => println!("{}", s),
                _ => {}
            }
        }
    };
}

fn setkv_id_cmd(matches: &ArgMatches) {
    let pass = read_pass();
    let reset = matches.is_present("reset");
    match fetch_kvs(matches) {
        Err(s) => println!("{}", s),
        Ok(kvs) => {
            match api::set_kvs_id(&pass, &kvs, reset) {
                Err(s) => println!("{}", s),
                _ => {}
            }
        }
    };
}

fn upgrade_cmd(matches: &ArgMatches) {
    let pass = read_pass();
    let name = matches.value_of("name").unwrap();
    if !api::exists(&pass, name) {
        println!("{} does not exist", name);
        return;
    }
    let set_password = matches.value_of("set-password");
    let pass = read_pass();
    match api::upgrade(name, &pass, set_password) {
        Err(s) => println!("{}", s),
        Ok((old_pass, new_pass)) => {
            println!("Old pass: {}\nNew pass: {}", old_pass, new_pass);
        }
    };
}

fn delete_cmd(matches: &ArgMatches) {
    let pass = read_pass();
    let name = matches.value_of("name").unwrap();
    if !api::exists(&pass, name) {
        println!("{} does not exist", name);
        return;
    }
    api::delete(&pass, name).unwrap();
    println!("Service {} deleted.", name);
}

pub fn cli() {
    let app = App::new("Auto-pass")
        .about("Auto-generate and encrypt passwords")
        .subcommand(SubCommand::with_name("init")
                    .about("Initialize new ap with master password")
                    .arg(arg_ident())
                    .arg(arg_kvs()))
        .subcommand(SubCommand::with_name("new")
                    .about("Create new service")
                    .arg(arg_name())
                    .arg(Arg::with_name("text_mode")
                         .long("text_mode")
                         .help("New password text mode")
                         .default_value("default")
                         .possible_values(&["default", "alphanumeric", "alphanumericunderscore", "nowhitespace"]))
                    .arg(Arg::with_name("length")
                         .long("length")
                         .short("l")
                         .value_name("LEN")
                         .help("New password's length")
                         .default_value("16"))
                    .arg(arg_kvs())
                    .arg(Arg::with_name("tags")
                         .short("t")
                         .help("Tags for this service")
                         .multiple(true)
                         .number_of_values(1))
                    .arg(arg_set_pass())
                    .display_order(10))
        .subcommand(SubCommand::with_name("get")
                    .about("Get password for service")
                    .arg(arg_name())
                    .arg(Arg::with_name("clipboard")
                         .short("c")
                         .help("Copy password to clipboard"))
                    .arg(Arg::with_name("all")
                         .short("a")
                         .long("all")
                         .help("Print everything about the service"))
                    .display_order(20))
        .subcommand(SubCommand::with_name("get-id")
                    .about("Get generic information not associated with a particular service"))
        .subcommand(SubCommand::with_name("list")
                    .about("List services unlocked by password")
                    .display_order(0)
                    .arg(Arg::with_name("simple")
                        .short("s")
                        .help("Simple output")))
        .subcommand(SubCommand::with_name("set-kv")
                    .about("Set key value pairs for a service")
                    .arg(arg_name())
                    .arg(arg_kvs())
                    .arg(Arg::with_name("reset")
                         .short("r")
                         .long("reset")
                         .takes_value(false)
                         .help("Clear all existing values"))
                    .display_order(50))
        .subcommand(SubCommand::with_name("set-kv-id")
                    .about("Set generic key value pairs not associated with a particular service")
                    .arg(arg_kvs())
                    .arg(Arg::with_name("reset")
                         .short("r")
                         .long("reset")
                         .takes_value(false)
                         .help("Clear all existing values"))
                    .display_order(50))
        .subcommand(SubCommand::with_name("upgrade")
                    .about("Upgrade password")
                    .arg(arg_name())
                    .arg(arg_set_pass())
                    .display_order(50))
        .subcommand(SubCommand::with_name("delete")
                    .about("Delete an existing service")
                    .arg(arg_name())
                    .display_order(50))
        .get_matches();

    match app.subcommand() {
        ("init", Some(matches)) => init_cmd(matches),
        ("new", Some(matches)) => new_cmd(matches),
        ("get", Some(matches)) => get_cmd(matches),
        ("get-id", Some(matches)) => get_id_cmd(matches),
        ("list", Some(matches)) => list_cmd(matches),
        ("set-kv", Some(matches)) => setkv_cmd(matches),
        ("set-kv-id", Some(matches)) => setkv_id_cmd(matches),
        ("upgrade", Some(matches)) => upgrade_cmd(matches),
        ("delete", Some(matches)) => delete_cmd(matches),
        _ => {
            println!("{}", app.usage());
        }
    };
}
