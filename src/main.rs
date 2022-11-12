use std::sync::{ Arc, Mutex };

use coin::{ Ledger, Tx };
use utils::{ get_ledger_repo_path };
use db::{ User, DB };
use actix_web::{ Responder, HttpResponse, HttpServer, App, web };
use serde::{ Deserialize, Serialize };

mod db;
mod coin;
mod utils;

#[derive(Deserialize)]
struct UserQueryReqBody {
    from_addr: String,
    to_addr: String,
    amt: u32,
    secret: String,
}

struct AppCtx {
    db: DB,
    ledger: Ledger,
}

#[derive(Serialize)]
struct RequestFailure<'a> {
    justification: &'a str,
}

#[derive(Serialize)]
struct Users {
    users: Vec<User>,
}

async fn handle_tx(req: web::Json<UserQueryReqBody>, data: web::Data<Arc<Mutex<AppCtx>>>) -> impl Responder {
    let ctx = data.lock().expect("failed to lock app data mutex");

    if !ctx.db.addr_exists(&req.to_addr).await {
        return HttpResponse::BadRequest().json(RequestFailure {
            justification: "no recieving user could be indexed with provided address",
        });
    }

    match ctx.db.fetch_by_addr(&req.from_addr).await {
        None =>
            HttpResponse::BadRequest().json(RequestFailure {
                justification: "no sender user could be indexed with provided address",
            }),
        Some(sender) => {
            if sender.bryxcoin_password != req.secret {
                return HttpResponse::BadRequest().json(RequestFailure {
                    justification: "invalid secret for sender indexed with provided address",
                });
            }

            match
                ctx.ledger.new_tx(
                    &(Tx {
                        amt: req.amt,
                        from_addr: req.from_addr.to_owned(),
                        to_addr: req.to_addr.to_owned(),
                    })
                )
            {
                Ok(_) => HttpResponse::Created().body("ok"),
                Err(err) =>
                    HttpResponse::BadGateway().json(RequestFailure {
                        justification: &format!("{}", err),
                    }),
            }
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let db = DB::init().await.expect("Failed to establish a connection with mongodb");
    let ledger = Ledger::init();

    let data = Arc::new(Mutex::new(AppCtx { ledger, db }));

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(data.clone()))
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