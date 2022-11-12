use std::{ ops::Add, fs, path::{ PathBuf, Path } };
use git2::{
    Repository,
    RemoteCallbacks,
    Cred,
    FetchOptions,
    build::RepoBuilder,
    PushOptions,
    ObjectType,
    Commit,
    Signature,
};
use serde::Serialize;
use SyncDirec::{FromRemote, ToRemote};

pub fn get_ledger_repo_path() -> PathBuf {
    std::env::current_dir().expect("could not get current directory").join("ledger")
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
        Tx { from_addr: from_addr.to_owned(), to_addr: to_addr.to_owned(), amt }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        let parts = s
            .split(|c| { c == '|' || c == '-' })
            .map(|part| part.trim())
            .collect::<Vec<&str>>();

        if let ([from_addr, to_addr], Ok(amt)) = (&parts[0..1], parts[2].parse::<u32>()) {
            Some(Tx::new(from_addr, *to_addr, amt))
        } else {
            None
        }
    }

    pub fn write_to_file(&self, path: PathBuf) -> Result<(), std::io::Error> {
        fs::write(path, format!("{}", &self))
    }
}

impl std::fmt::Display for Tx {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_fmt(
            format_args!("{}-{} | {}", self.from_addr, self.to_addr, self.amt.to_string())
        )
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
        let repo = builder.clone(crate::REMOTE, &path).expect("failed to clone repo!");

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

    pub fn new_tx(&self, tx: &Tx) {
        let tx_idx = &self.get_last_tx_idx() + 1;
        let path = get_ledger_repo_path().join(tx_idx.to_string().add(".tx"));

        self.sync(FromRemote);

        fs::write(&path, format!("{}", tx)).expect("cannot write tx to disk");
        self.add_and_commit(Path::new(&tx_idx.to_string().add(".tx")), &tx_idx.to_string()).expect(
            "failed to commit tx to git tree"
        );

        self.sync(ToRemote);
    }

    pub fn add_and_commit(&self, path: &Path, message: &str) -> Result<git2::Oid, git2::Error> {
        let mut idx = self.repo.index()?;
        idx.add_path(path)?;

        let oid = idx.write_tree()?;
        let signature = Signature::now("Bryxcoin Comitter", "ledger@bryxcoin.org")?;
        let parent_commit = self.get_last_commit()?;
        let tree = self.repo.find_tree(oid)?;

        self.repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[&parent_commit])
    }
}