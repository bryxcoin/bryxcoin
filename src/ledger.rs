use git2::{
    build::RepoBuilder, Commit, Cred, FetchOptions, ObjectType, PushOptions, RemoteCallbacks,
    Repository, Signature,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    ops::Add,
    path::{Path, PathBuf},
    str::FromStr,
};
use SyncDirec::{FromRemote, ToRemote};

use crate::{settings::Settings, skip_if, BANK_ADDR};

pub fn get_ledger_repo_path() -> PathBuf {
    std::env::current_dir()
        .expect("could not get current directory")
        .join("ledger")
}

enum SyncDirec {
    FromRemote,
    ToRemote,
}

#[derive(Serialize)]
pub struct Tx {
    pub from_addr: String,
    pub to_addr: String,
    pub amt: u32,
}

impl Tx {
    pub fn new(from_addr: &str, to_addr: &str, amt: u32) -> Self {
        Self {
            from_addr: from_addr.to_owned(),
            to_addr: to_addr.to_owned(),
            amt,
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        let parts = s
            .split(|c| c == '|' || c == '-')
            .map(|part| part.trim())
            .collect::<Vec<&str>>();

        if let (from_addr, to_addr, Ok(amt)) = (parts[0], parts[1], parts[2].parse::<u32>()) {
            Some(Self::new(from_addr, to_addr, amt))
        } else {
            None
        }
    }
}

impl std::fmt::Display for Tx {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_fmt(format_args!(
            "{}-{} | {}",
            self.from_addr,
            self.to_addr,
            self.amt.to_string()
        ))
    }
}

pub struct Ledger<'a> {
    pub repo: Repository,
    pub balances: HashMap<String, u32>,
    pub settings: &'a Settings,
}

impl<'a> Ledger<'a> {
    pub fn init(settings: &'a Settings) -> Self {
        let mut builder = RepoBuilder::new();
        let mut cbs = RemoteCallbacks::new();
        let mut opts = FetchOptions::new();

        cbs.credentials(|_, _, _| {
            let creds = Cred::ssh_key(
                "git",
                Some(&PathBuf::from_str(&settings.public_key).unwrap()),
                &PathBuf::from_str(&settings.private_key).unwrap(),
                None,
            )
            .expect("failed to create credentials object");

            Ok(creds)
        });

        opts.remote_callbacks(cbs);
        builder.fetch_options(opts);

        let path = get_ledger_repo_path();
        let repo = builder
            .clone(&settings.ledger_repo, &path)
            .expect("failed to clone repo!");

        Self {
            repo,
            balances: HashMap::new(),
            settings,
        }
    }

    pub fn compute_balances(&mut self) {
        let Self { balances, .. } = self;
        balances.clear();

        let mut ents = fs::read_dir(get_ledger_repo_path())
            .expect("failed to read from ledger directory")
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<PathBuf>, std::io::Error>>()
            .unwrap();

        alphanumeric_sort::sort_path_slice(&mut ents);

        for ent in ents {
            skip_if!(ent.is_dir());

            let tx = Tx::from_str(
                &fs::read_to_string(&ent).expect("could not read from local ledger copy"),
            )
            .expect("polluted/invalid tx file! failed to deseralize into Tx");
            let sender_balance = *balances.get(&tx.from_addr).unwrap_or(&0u32);
            let recv_balance = *balances.get(&tx.to_addr).unwrap_or(&0u32);

            balances.insert(tx.to_addr, recv_balance + tx.amt);

            // psudo-address "bank"; no further logic required
            skip_if!(tx.from_addr == BANK_ADDR);

            if sender_balance < tx.amt {
                panic!(
                    "polluted ledger! when reading tx file: {}, balance of addr: {} is {}, but this tx denotes a withdrawl of {}, making their balance negative!",
                    ent.display(),
                    tx.from_addr,
                    sender_balance,
                    tx.amt
                );
            }

            balances.insert(tx.from_addr, sender_balance - tx.amt);
        }
    }

    fn sync(&mut self, direction: SyncDirec) {
        let mut cbs = RemoteCallbacks::new();

        cbs.credentials(|_, _, _| {
            let creds = Cred::ssh_key(
                "git",
                Some(&PathBuf::from_str(self.settings.public_key.as_str()).unwrap()),
                &PathBuf::from_str(self.settings.private_key.as_str()).unwrap(),
                None,
            )
            .expect("failed to create credentials object");

            Ok(creds)
        });

        let mut remote = self
            .repo
            .find_remote("origin")
            .expect("could not find origin!");

        match direction {
            SyncDirec::FromRemote => {
                let mut opts = FetchOptions::new();
                opts.remote_callbacks(cbs);
                remote
                    .fetch(&["master"], Some(&mut opts), None)
                    .expect("failed to pull from origin/master");
            }
            SyncDirec::ToRemote => {
                let mut opts = PushOptions::new();
                opts.remote_callbacks(cbs);
                remote
                    .push(&["refs/heads/master:refs/heads/master"], Some(&mut opts))
                    .expect("failed to push to origin/master");
            }
        }
    }

    pub fn get_last_commit(&self) -> Result<Commit, git2::Error> {
        self.repo
            .head()?
            .resolve()?
            .peel(ObjectType::Commit)?
            .into_commit()
            .map_err(|_| git2::Error::from_str("Couldn't find commit"))
    }

    pub fn get_last_tx_idx(&self) -> u32 {
        self.get_last_commit()
            .expect("cannot find last commit")
            .message()
            .expect("cannot get message from last commit")
            .trim()
            .parse()
            .expect("could not parse message from last commit to u32")
    }

    pub fn new_tx(&mut self, tx: &Tx) {
        let tx_idx = &self.get_last_tx_idx() + 1;
        let path = get_ledger_repo_path().join(tx_idx.to_string().to_owned().add(".tx"));

        self.sync(FromRemote);

        fs::write(&path, format!("{}", tx)).expect("cannot write tx to disk");
        self.add_and_commit(
            Path::new(&tx_idx.to_string().add(".tx")),
            &tx_idx.to_string(),
        )
        .expect("failed to commit tx to git tree");

        self.compute_balances();
        self.sync(ToRemote);
    }

    pub fn add_and_commit(&self, path: &Path, message: &str) -> Result<git2::Oid, git2::Error> {
        let mut idx = self.repo.index()?;
        idx.add_path(path)?;

        let oid = idx.write_tree()?;
        let signature = Signature::now("Bryxcoin Comitter", "ledger@bryxcoin.org")?;
        let parent_commit = self.get_last_commit()?;
        let tree = self.repo.find_tree(oid)?;

        self.repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            &message,
            &tree,
            &[&parent_commit],
        )
    }
}
