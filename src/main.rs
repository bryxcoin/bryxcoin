use coin::Ledger;
use utils::{get_ledger_repo_path};
use db::{ User, DB };
use actix_web::{ Responder, HttpResponse, HttpServer, App, web };
use serde::{ Deserialize, Serialize };

mod db;
mod coin;
mod utils;

#[derive(Deserialize)]
struct UserQueryReqBody {
    first_name: String,
    last_name: String,
}

#[derive(Serialize)]
struct Users {
    users: Vec<User>,
}

async fn handle_tx(req: web::Json<UserQueryReqBody>, db: web::Data<DB>) -> impl Responder {
    match db.fetch_users(&req.first_name, &req.last_name).await {
        Ok(users) => HttpResponse::Ok().json(Users { users }),
        Err(_) => HttpResponse::InternalServerError().body("Mongodb Query Failed"),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = DB::init().await.expect("Failed to establish a connection with mongodb");

    crate::coin::Tx::from_str("tyler - holewinski | 1000");
    let ledger = Ledger::init().expect("failed to clone ledger");

    println!(
        "{}",
        crate::utils::find_last_commit(&ledger.repo).expect("no commit").message().expect("no body")
    );

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(DB { client: db.client.to_owned() }))
            .route(
                "/health",
                web::get().to(|| HttpResponse::Ok().body("ok"))
            )
            .route("/tx", web::post().to(handle_tx))
    })
        .bind(("0.0.0.0", 8080))?
        .run().await
        .and_then(|_| {
            std::fs::remove_dir_all(get_ledger_repo_path()).expect("failed to cleanup pwd/ledger!");
            println!("[Done]");

            Ok(())
        })
}