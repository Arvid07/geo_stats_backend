mod entities;
mod geo_guessr;
mod migrator;
mod requests;
mod login;

use crate::login::login_request::{link_account, log_out, user_login, user_signup, verify_email};
use crate::requests::general_stats_requests::get_general_stats;
use crate::requests::import_games::import_recent_games;
use crate::requests::insertion_requests::{insert_duels_game, insert_solo_game};
use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{App, HttpServer};
use dotenv::dotenv;
use sea_orm::{Database, DbErr};
use std::env;
use crate::requests::country_stats_request::get_country_stats;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();
    
    dotenv().ok();
    
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in a .env file!");
    
    let db = Database::connect(database_url)
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
            .service(get_general_stats)
            .service(user_login)
            .service(user_signup)
            .service(verify_email)
            .service(link_account)
            .service(log_out)
            .service(import_recent_games)
            .service(get_country_stats)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
