use std::str;
use std::sync::Arc;

use futures::StreamExt;
use mongodb::bson::{doc, document::Document};
use mongodb::{options::ClientOptions, Client, Collection};
use serde::{Serialize, Deserialize};

type MongoResult<T> = std::result::Result<T, mongodb::error::Error>;

const DB_NAME: &str = "bagelbot";
const COLL: &str = "users";

const ID: &str = "_id";
const FIRST_NAME: &str = "first_name";
const LAST_NAME: &str = "last_name";
const SLACK_USER_ID: &str = "slack_user_id";
const SLACK_USER_NAME: &str = "slack_user_name";

const BRYXCOIN_WALLET: &str = "bryxcoin_wallet";
const BRYXCOIN_ADDRESS: &str = "bryxcoin_address";

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub id: String,
    pub first_name: String,
    pub last_name: String,
    pub slack_user_id: String,
    pub slack_user_name: String,
    pub bryxcoin_wallet: String,
    pub bryxcoin_address: String,
}

#[derive(Clone, Debug)]
pub struct DB {
    pub client: Client,
}

impl DB {
    pub async fn init() -> MongoResult<Self> {
        let conn_str = std::env::var("MONGO_CONN_STR").expect("$MONGO_CONN_STR is not set!");
        let mut client_options = ClientOptions::parse(&conn_str).await?;
        
        client_options.app_name = Some("bryxcoin".to_string());

        println!("Connected!");

        Ok(Self {
            client: Client::with_options(client_options)?,
        })
    }

    pub async fn fetch_users(&self, first_name: &str, last_name: &str)-> MongoResult<Vec<User>> {
        let mut cursor = self
            .get_collection()
            .find(
                doc! {
                    "first_name": first_name,
                    "last_name": last_name
                }
                , None)
            .await?;

        let mut res: Vec<User> = Vec::new();
        while let Some(doc) = cursor.next().await {
            res.push(self.doc_to_user(&doc?)?);
        }

        Ok(res)
    }

    fn doc_to_user(&self, doc: &Document) -> MongoResult<User> {
        let id = doc.get_object_id(ID).unwrap();
        let first_name = doc.get_str(FIRST_NAME).unwrap();
        let last_name = doc.get_str(LAST_NAME).unwrap();
        let slack_user_id = doc.get_str(SLACK_USER_ID).unwrap();
        let slack_user_name = doc.get_str(SLACK_USER_NAME).unwrap();
        let bryxcoin_address = doc.get_str(BRYXCOIN_ADDRESS).unwrap();
        let bryxcoin_wallet = doc.get_str(BRYXCOIN_WALLET).unwrap();

        let user = User {
            id: id.to_hex(),
            first_name: first_name.to_owned(),
            last_name: last_name.to_owned(),
            slack_user_id: slack_user_id.to_owned(),
            slack_user_name: slack_user_name.to_owned(),
            bryxcoin_address: bryxcoin_address.to_owned(),
            bryxcoin_wallet: bryxcoin_wallet.to_owned()
        };

        Ok(user)
    }

    fn get_collection(&self) -> Collection {
        self.client.database(DB_NAME).collection(COLL)
    }
}