use std::str;

use futures::StreamExt;
use mongodb::bson::{ doc, document::Document };
use mongodb::{ options::ClientOptions, Client, Collection };
use serde::{ Serialize, Deserialize };

type MongoResult<T> = std::result::Result<T, mongodb::error::Error>;

const DB_NAME: &str = "bagelbot";
const COLL: &str = "users";

const FIRST_NAME: &str = "first_name";
const LAST_NAME: &str = "last_name";

const BRYXCOIN_ADDRESS: &str = "bryxcoin_address";
const BRYXCOIN_PASSWORD : &str = "bryxcoin_password";

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub first_name: String,
    pub last_name: String,
    pub bryxcoin_address: String,
    pub bryxcoin_password: String
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


        Ok(Self {
            client: Client::with_options(client_options)?,
        })
    }

    pub async fn fetch_by_addr(&self, addr: &str) -> Option<User> {
        self.get_collection()
            .find_one(doc! { "bryxcoin_address": addr }, None).await
            .expect("failed to query users collection")
            .and_then(|doc| { self.doc_to_user(&doc).ok() })
    }

    pub async fn fetch_users(&self, first_name: &str, last_name: &str) -> MongoResult<Vec<User>> {
        let mut cursor = self
            .get_collection()
            .find(
                doc! {
                    "first_name": first_name,
                    "last_name": last_name
                },
                None
            ).await?;

        let mut res: Vec<User> = Vec::new();
        while let Some(doc) = cursor.next().await {
            res.push(self.doc_to_user(&doc?)?);
        }

        Ok(res)
    }

    fn doc_to_user(&self, doc: &Document) -> MongoResult<User> {
        let first_name = doc.get_str(FIRST_NAME).unwrap();
        let last_name = doc.get_str(LAST_NAME).unwrap();
        let bryxcoin_address = doc.get_str(BRYXCOIN_ADDRESS).unwrap();
        let bryxcoin_password = doc.get_str(BRYXCOIN_PASSWORD).unwrap();

        let user = User {
            first_name: first_name.to_owned(),
            last_name: last_name.to_owned(),
            bryxcoin_address: bryxcoin_address.to_owned(),
            bryxcoin_password: bryxcoin_password.to_owned()
        };

        Ok(user)
    }

    fn get_collection(&self) -> Collection {
        self.client.database(DB_NAME).collection(COLL)
    }
}