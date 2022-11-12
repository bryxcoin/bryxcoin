use chrono::format::format;
use git2::{ Oid, Signature, Repository, Commit, ObjectType, RemoteCallbacks, Cred, PushOptions };
use std::path::{ Path, PathBuf };

pub fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
    let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
    obj.into_commit().map_err(|_| git2::Error::from_str("Couldn't find commit"))
}

pub fn add_and_commit(repo: &Repository, path: &Path, message: &str) -> Result<Oid, git2::Error> {
    let mut idx = repo.index()?;
    idx.add_path(path)?;
    let oid = idx.write_tree()?;
    let signature = Signature::now("Bryxcoin Comitter", "ledger@bryxcoin.org")?;
    let parent_commit = find_last_commit(&repo)?;
    let tree = repo.find_tree(oid)?;
    repo.commit(
        Some("HEAD"), //  point HEAD to our new commit
        &signature, // author
        &signature, // committer
        message, // commit message
        &tree, // tree
        &[&parent_commit]
    ) // parents
}

pub fn push_to_remote(repo: &Repository, url: &str) -> Result<(), git2::Error> {
    let mut remote = match repo.find_remote("origin") {
        Ok(r) => r,
        Err(_) => repo.remote("origin", url)?,
    };

    println!("trying: {}/.ssh/id_rsa", std::env::var("HOME").unwrap());

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_, _, _| {
        let creds = Cred::ssh_key(
            "git",
            Some(Path::new("/Users/tyler/.ssh/id_rsa.pub")),
            Path::new(&format!("{}/.ssh/id_rsa", std::env::var("HOME").unwrap())),
            Some("honeyTSH207980")
        ).expect("failed to create credentials object");

        Ok(creds)
    });


    remote.connect_auth(git2::Direction::Push, Some(callbacks), None)?;
    remote.push(&["refs/heads/master:refs/heads/master"], None)
}

pub fn get_ledger_repo_path() -> PathBuf {
    std::env::current_dir().expect("could not get current directory").join("ledger")
}