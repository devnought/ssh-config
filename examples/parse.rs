use clap::Clap;
use std::{fs, path::PathBuf};

#[derive(Clap, Debug)]
struct Opts {
    pub input: PathBuf,
}

fn main() {
    let opt = Opts::parse();
    let data = fs::read_to_string(&opt.input).unwrap_or_else(|_| {
        println!("Could not open file '{}'", &opt.input.display());
        std::process::exit(1);
    });

    for host in ssh_config::parse(&data) {
        println!("{:#?}", host);
    }
}
