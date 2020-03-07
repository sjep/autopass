use std::env;
use std::io::{self, stdin, stdout, Write};
use std::str::FromStr;

use clap::{Arg, App, SubCommand, ArgMatches};
use termion::input::TermRead;

use crate::api;
use crate::hash::TextMode;


const PASS_ENV_VAR: &'static str = "PASS_PASS";

fn read_pass_raw(prompt: &str) -> String {
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


fn read_pass(twice: bool) -> Option<String> {
    if !twice {
        if let Ok(s) = env::var(PASS_ENV_VAR) {
            return Some(s);
        }
    }
    let pass1 = read_pass_raw("password: ");
    match twice {
        false => Some(pass1),
        true => {
            let pass2 = read_pass_raw("enter again: ");
            match pass1 == pass2 {
                true => Some(pass1),
                false => None
            }
        }
    }
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


fn new_cmd(matches: &ArgMatches) {
    let name = matches.value_of("name").unwrap();
    println!("Adding '{}' as new service", name);

    let text_mode = match matches.value_of("text_mode").unwrap() {
        "alphanumeric" => TextMode::AlphaNumeric,
        "alphanumericunderscore" => TextMode::AlphaNumericUnderscore,
        "nowhitespace" => TextMode::NoWhiteSpace,
        _ => TextMode::NoWhiteSpace
    };

    let len = match usize::from_str(matches.value_of("length").unwrap()) {
        Err(_) => {
            println!("Length provided not an integer");
            return;
        },
        Ok(l) => l
    };
    let mut valid = true;
    let kvs: Vec<(&str, &str)> = match matches.values_of("kvs") {
        None => vec![],
        Some(v) => v.map(|elem| {
            let eqidx = match elem.find("=") {
                None => {
                    println!("{} must be of the form KEY=VALUE", elem);
                    valid = false;
                    0
                },
                Some(idx) => idx
            };
            (&elem[0..eqidx], &elem[eqidx + 1..elem.len()])
        }).collect()
    };
    if !valid {
        println!("Key value pairs must be of the form KEY=VALUE");
        return;
    }

    let set_password = matches.value_of("set-password");

    let pass = match read_pass(true) {
        None => {
            println!("Passwords don't match");
            return;
        },
        Some(p) => p
    };
    match api::new(name, &pass, &text_mode, len, &kvs, set_password)  {
        Ok(entry) => println!("New password created for service '{}':\n{}", name, entry.get_pass(false).unwrap()),
        Err(s) => println!("Error creating service: {}", s)
    };
}


fn get_cmd(matches: &ArgMatches) {
    let clipboard = matches.is_present("clipboard");
    let name = matches.value_of("name").unwrap();
    let all = matches.is_present("all");
    let pass = read_pass(false).unwrap();
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
    let pass = read_pass(false).unwrap();
    println!("\nServices\n--------");
    for n in api::list(&pass) {
        println!("{}", n);
    }
}


fn setkv_cmd(matches: &ArgMatches) {
    let name = matches.value_of("name").unwrap();
    let pass = read_pass(false).unwrap();
    match fetch_kvs(matches) {
        Err(s) => println!("{}", s),
        Ok(kvs) => {
            match api::set_kvs(name, &pass, &kvs) {
                Err(s) => println!("{}", s),
                _ => {}
            }
        }
    };
}


fn upgrade_cmd(matches: &ArgMatches) {
    let name = matches.value_of("name").unwrap();
    let set_password = matches.value_of("set-password");
    let pass = read_pass(false).unwrap();
    match api::upgrade(name, &pass, set_password) {
        Err(s) => println!("{}", s),
        _ => {}
    };
}


pub fn cli() {
    let app = App::new("Auto-pass")
        .about("Auto-generate and encrypt passwords")
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
                    .arg(arg_set_pass()))
        .subcommand(SubCommand::with_name("get")
                    .about("Get password for service")
                    .arg(arg_name())
                    .arg(Arg::with_name("clipboard")
                         .short("c")
                         .help("Copy password to clipboard"))
                    .arg(Arg::with_name("all")
                         .short("a")
                         .long("all")
                         .help("Print everything about the service")))
        .subcommand(SubCommand::with_name("list")
                    .about("List services unlocked by password"))
        .subcommand(SubCommand::with_name("set-kv")
                    .about("Set key value pairs for a service")
                    .arg(arg_name())
                    .arg(arg_kvs()))
        .subcommand(SubCommand::with_name("upgrade")
                    .about("Upgrade password")
                    .arg(arg_name())
                    .arg(arg_set_pass()))
        .get_matches();

    match app.subcommand() {
        ("new", Some(matches)) => new_cmd(matches),
        ("get", Some(matches)) => get_cmd(matches),
        ("list", Some(matches)) => list_cmd(matches),
        ("set-kv", Some(matches)) => setkv_cmd(matches),
        ("upgrade", Some(matches)) => upgrade_cmd(matches),
        _ => {
            println!("Command not recognized");
        }
    };
}
