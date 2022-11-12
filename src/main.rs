use std::sync::{ Arc, Mutex };
use ledger::Ledger;
use db::DB;
use ledger::get_ledger_repo_path;
use actix_web::{ HttpResponse, HttpServer, App, web::{ self, Data } };

mod db;
mod ledger;
mod http;

const REMOTE: &str = "git@github.com:bryxcoin/ledger.git";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let data = Arc::new(
        Mutex::new(http::AppData {
            ledger: Ledger::init(),
            db: DB::init().await.expect("failed to establish connection with mongodb"),
        })
    );

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(data.clone()))
            .route(
                "/health",
                web::get().to(|| HttpResponse::Ok().body("ok"))
            )
            .route("/tx", web::post().to(http::handle_tx))
            .route("/addr", web::get().to(http::handle_addr))
    })
        .bind(("0.0.0.0", 8080))?
        .run().await
        .and_then(|_| {
            std::fs::remove_dir_all(get_ledger_repo_path()).expect("failed to cleanup pwd/ledger!");
            println!("[Done]");

            Ok(())
        })
}