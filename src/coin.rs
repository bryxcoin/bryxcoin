use std::{ ops::{ Index, Add }, fs, path::{ PathBuf, Path } };

use git2::{ Repository, RemoteCallbacks, Cred, FetchOptions, build::RepoBuilder, Remote, PushOptions };

use crate::{utils::{ get_ledger_repo_path, find_last_commit, add_and_commit, push_to_remote }, db::User};

const REMOTE: &str = "git@github.com:bryxcoin/ledger.git";

enum SyncDirec {
    FromRemote,
    ToRemote,
}

pub struct Tx {
    pub from_addr: String,
    pub to_addr: String,
    pub amt: u32,
}

impl Tx {
    pub fn from_str(s: &str) -> Self {
        let iter: Vec<&str> = s
            .split(|c| { c == '|' || c == '-' })
            .map(|part| part.trim())
            .collect();

        let from_addr = (*iter.index(0)).to_owned();
        let to_addr = (*iter.index(1)).to_owned();
        let amt: u32 = (*iter.index(2)).to_owned().parse().expect("failed to parse tx amount");

        Tx { from_addr, to_addr, amt }
    }

    pub fn write_to_file(&self, path: PathBuf) -> Result<(), std::io::Error> {
        fs::write(path, format!("{}", &self))
    }
}

impl std::fmt::Display for Tx {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_fmt(format_args!("{}-{} | {}", self.from_addr, self.to_addr, self.amt.to_string()))
    }
}

pub struct Ledger {
    pub repo: Repository,
}

impl Ledger {
    pub fn init() -> Self {
        let mut builder = RepoBuilder::new();
        let mut cbs = RemoteCallbacks::new();
        let mut opts = FetchOptions::new();

        cbs.credentials(|_, _, _| {
            let creds = Cred::ssh_key(
                "git",
                Some(Path::new("/Users/tyler/.ssh/id_ed25519.pub")),
                Path::new(&format!("{}/.ssh/id_ed25519", std::env::var("HOME").unwrap())),
                None
            ).expect("failed to create credentials object");

            Ok(creds)
        });

        opts.remote_callbacks(cbs);
        builder.fetch_options(opts);

        let path = get_ledger_repo_path();
        let repo = builder.clone(REMOTE, &path).expect("failed to clone repo!");

        Ledger { repo }
    }

    fn sync(&self, direction: SyncDirec) {
        let mut cbs = RemoteCallbacks::new();

        cbs.credentials(|_, _, _| {
            let creds = Cred::ssh_key(
                "git",
                Some(Path::new("/Users/tyler/.ssh/id_ed25519.pub")),
                Path::new(&format!("{}/.ssh/id_ed25519", std::env::var("HOME").unwrap())),
                None
            ).expect("failed to create credentials object");

            Ok(creds)
        });

        let mut remote = self.repo.find_remote("origin").expect("could not find origin!");

        match direction {
            SyncDirec::FromRemote => {
                let mut opts = FetchOptions::new();
                opts.remote_callbacks(cbs);
                remote.fetch(&["master"], Some(&mut opts), None).expect("failed to pull from origin/master");
            },
            SyncDirec::ToRemote => {
                let mut opts = PushOptions::new();
                opts.remote_callbacks(cbs);
                remote.push(&["refs/heads/master:refs/heads/master"], Some(&mut opts)).expect("failed to push to origin/master");
            }
        };
    }

    pub fn get_last_tx_idx(&self) -> u32 {
        find_last_commit(&self.repo)
            .expect("cannot find last commit")
            .message()
            .expect("cannot get message from last commit")
            .trim()
            .parse()
            .expect("could not parse message from last commit to u32")
    }

    pub fn new_tx(&self, tx: &Tx) -> Result<(), Box<dyn std::error::Error>> {
        let idx = &self.get_last_tx_idx() + 1;
        let path = get_ledger_repo_path().join(idx.to_string().add(".tx"));

        fs::write(&path, format!("{}", tx)).expect("cannot write tx to disk");
        add_and_commit(&self.repo, Path::new(&idx.to_string().add(".tx")), &idx.to_string()).expect(
            "failed to commit tx to git tree"
        );

        self.sync(SyncDirec::ToRemote);
        Ok(())
    }
}