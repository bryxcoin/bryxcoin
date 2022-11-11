use std::{ops::Index, fs};

use git2::Repository;

use crate::utils::get_ledger_repo_path;

pub struct Tx {
    from: String,
    to: String,
    amt: u32,
}

impl Tx {
    pub fn from_str(s: &str) -> Self {
        let iter: Vec<&str> = s
            .split(|c| { c == '|' || c == '-' })
            .map(|part| part.trim())
            .collect();

        let from = (*iter.index(0)).to_owned();
        let to = (*iter.index(1)).to_owned();
        let amt: u32 = (*iter.index(2)).to_owned().parse().expect("failed to parse tx amount");

        Tx { from, to, amt }
    }
}

impl std::fmt::Display for Tx {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_fmt(format_args!("{}\t-\t{}\t| {}", self.from, self.to, self.amt.to_string()))
    }
}

pub struct Ledger {
    pub repo: Repository,
}

impl Ledger {
    pub fn init() -> Result<Self, git2::Error> {
        let path = get_ledger_repo_path();
        let url = "https://github.com/bryxcoin/ledger.git";

        println!("{}", path.as_os_str().to_str().unwrap());

        match Repository::clone(url, path) {
            Ok(repo) => Ok(Ledger { repo }),
            Err(err) => Err(err),
        }
    }

    pub fn new_tx(&self, tx: &Tx) {
        std::fs::remove_dir_all(get_ledger_repo_path());
        println!("[Done]");
    }
}