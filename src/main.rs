use std::sync::{ Arc, Mutex };
use lazy_static::lazy_static;
use ledger::Ledger;
use db::DB;
use ledger::get_ledger_repo_path;
use actix_web::{ HttpResponse, HttpServer, App, web::{ self, Data } };

use crate::settings::Settings;

mod db;
mod ledger;
mod http;
mod macros;
mod settings;

const BANK_ADDR: &str = "0000000000000000000000000000000000000000000000000000000000000000";

// lazy_static! {
//     static ref SETTINGS: Settings
//     static ref HASHMAP: HashMap<u32, &'static str> = {
//         let mut m = HashMap::new();
//         m.insert(0, "foo");
//         m.insert(1, "bar");
//         m.insert(2, "baz");
//         m
//     };
//     static ref COUNT: usize = HASHMAP.len();
//     static ref NUMBER: u32 = times_two(21);
// }

lazy_static! {
    static ref SETTINGS: Settings = {
        Settings::new().expect("Failed to load bryxcoin.toml config file!")
    };
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = DB::init(&SETTINGS.mongo_connection_string).await.expect("failed to establish a connection with mongodb");

    let mut ledger = Ledger::init(&SETTINGS);
    ledger.compute_balances();


    println!("current balances:");

    for (k, v) in &ledger.balances {
        println!("{}: {} bxcn", k, v);
    }

    println!("---------");

    let data = Arc::new(
        Mutex::new(http::AppData {
            ledger,
            db,
            settings: &SETTINGS
        })
    );

    println!("ledger_repo: {}", &SETTINGS.ledger_repo);
    println!("mongo: {}", &SETTINGS.mongo_connection_string);
    println!("user_collection: {}", &SETTINGS.mongo_user_collection);
    println!("user_database: {}", &SETTINGS.mongo_user_database);

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(data.clone()))
            .route(
                "/health",
                web::get().to(|| HttpResponse::Ok().body("ok"))
            )
            .route("/tx", web::post().to(http::handle_tx))
            .route("/users", web::get().to(http::handle_users))
            .route("/ledger", web::get().to(http::get_txs) )
    })
        .bind(("0.0.0.0", SETTINGS.port))?
        .run().await
        .and_then(|_| {
            std::fs::remove_dir_all(get_ledger_repo_path()).expect("failed to cleanup pwd/ledger!");
            println!("[Done]");

            Ok(())
        })
}
