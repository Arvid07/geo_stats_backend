mod entities;
mod geo_guessr;
mod migrator;
mod requests;
mod login;

use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{App, HttpServer};
use sea_orm::{Database, DbErr};
use crate::login::login_request::{link_account, log_out, user_login, user_signup, verify_email};
use crate::requests::get_requests::get_home_page;
use crate::requests::import_games::import_recent_games;
use crate::requests::insertion_requests::{insert_duels_game, insert_solo_game};

const DATABASE_URL: &str = "***REMOVED***";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();
    
    let db = Database::connect(DATABASE_URL)
        .await
        .unwrap_or_else(|db_err: DbErr| {
            eprintln!("Failed connecting to db: {}", db_err);
            std::process::exit(1);
        });

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .app_data(Data::new(db.clone()))
            .service(insert_duels_game)
            .service(insert_solo_game)
            .service(get_home_page)
            .service(user_login)
            .service(user_signup)
            .service(verify_email)
            .service(link_account)
            .service(log_out)
            .service(import_recent_games)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
