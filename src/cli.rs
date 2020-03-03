use std::io::{self, stdin, stdout, Write};
use std::str::FromStr;

use clap::{Arg, App, SubCommand};
use termion::input::TermRead;

use crate::api;
use crate::hash::TextMode;


fn read_pass() -> String {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    stdout.write_all(b"password: ").unwrap();
    stdout.flush().unwrap();

    let pass = stdin.read_passwd(&mut stdout).unwrap().unwrap();
    println!();
    pass
}

pub fn cli() {
    let matches = App::new("Auto-pass")
        .about("Auto-generate and encrypt passwords")
        .subcommand(SubCommand::with_name("new")
                    .about("Create new service")
                    .arg(Arg::with_name("name")
                         .short("n")
                         .long("name")
                         .value_name("NAME")
                         .help("New service name")
                         .takes_value(true)
                         .required(true))
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
                    .arg(Arg::with_name("kvs")
                         .long("key-value")
                         .short("k")
                         .help("Additional key-value pairs to be stored of the form -k KEY=VALUE")
                         .multiple(true)
                         .number_of_values(1)))
        .get_matches();

    if let Some(new_matches) = matches.subcommand_matches("new") {
        let name = new_matches.value_of("name").unwrap();
        println!("Adding {} as new service", name);

        let text_mode = match new_matches.value_of("text_mode").unwrap() {
            "alphanumeric" => TextMode::AlphaNumeric,
            "alphanumericunderscore" => TextMode::AlphaNumericUnderscore,
            "nowhitespace" => TextMode::NoWhiteSpace,
            _ => TextMode::NoWhiteSpace
        };

        let len = match usize::from_str(new_matches.value_of("length").unwrap()) {
            Err(_) => {
                println!("Length provided not an integer");
                return;
            },
            Ok(l) => l
        };
        let mut valid = true;
        let kvs: Vec<(&str, &str)> = match new_matches.values_of("kvs") {
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

        let pass = read_pass();
        match api::new(name, &pass, &text_mode, len, &kvs, None)  {
            Ok(entry) => println!("New password created for service '{}':\n{}", name, entry.get_pass(false).unwrap()),
            Err(s) => println!("Error creating service '{}': {}", name, s)
        };
    }
}
