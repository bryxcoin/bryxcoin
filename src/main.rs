use db::{User, DB};
use actix_web::{Responder, HttpResponse, HttpServer, App, web };
use serde::{Deserialize, Serialize};
use std::sync::Arc;

mod db;

#[derive(Deserialize)]
struct UserQueryReqBody {
    first_name: String,
    last_name: String
}

#[derive(Serialize)]
struct Users {
    users: Vec<User>
}

async fn handle_tx(req: web::Json<UserQueryReqBody>, db: web::Data<DB>) -> impl Responder {
    match db.fetch_users(&req.first_name, &req.last_name).await {
        Ok(users) => HttpResponse::Ok().json(Users { users }),
        Err(_) => HttpResponse::InternalServerError().body("Mongodb Query Failed")
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = DB::init().await.expect("Failed to establish a connection with mongodb");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(DB { client: db.client.to_owned() }))
            .route("/health", web::get().to(|| HttpResponse::Ok().body("ok")))
            .route("/tx", web::post().to(handle_tx))
    }).bind(("0.0.0.0", 8080))?
    .run()
    .await
}