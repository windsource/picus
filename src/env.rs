use std::{env, process};

pub fn read_env_or_exit(name: &str) -> String {
    let res = env::var(name);
    if res.is_err() {
        eprintln!("Error: Environment variable {} not found!", name);
        process::exit(1);
    }
    res.unwrap()
}

pub fn read_env_or_default(name: &str, default: String) -> String {
    env::var(name).unwrap_or(default)
}