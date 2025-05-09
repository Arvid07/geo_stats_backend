use crate::geo_guessr::{Entry, Payload, PayloadGameInfo};
use actix_web::error::ErrorBadRequest;
use actix_web::{post, web, Error, HttpResponse, Responder};
use sea_orm::DatabaseConnection;
use serde::Deserialize;

const REQUEST_CHUNK_SIZE: usize = 20;

#[derive(Deserialize, Debug)]
struct ImportRecentGamesRequest {
    entries: Vec<Entry>,
}

async fn insert_games(user_id: &str, game_ids: Vec<String>, db: &DatabaseConnection, ) -> Result<(), Error> {
    Ok(())
}

#[post("/import-games")]
pub async fn import_recent_games(
    request: web::Json<ImportRecentGamesRequest>,
    db: web::Data<DatabaseConnection>
) -> Result<impl Responder, Error> {
    if (request.entries.is_empty()) {
        return Err(ErrorBadRequest("Game History is empty!"));
    }

    let db = db.get_ref();
    let user_id = request.entries[0].user.id.clone();

    let mut game_ids = Vec::with_capacity(REQUEST_CHUNK_SIZE);
    let mut inserted_games = 0;

    for entry in request.entries.iter() {
        if let Ok(payloads) = serde_json::from_str::<Vec<Payload>>(&entry.payload) {
            for payload in payloads {
                inserted_games += 1;
                game_ids.push(payload.payload.game_id);

                if inserted_games % REQUEST_CHUNK_SIZE == 0 {
                    insert_games(&user_id, game_ids, db).await?;
                    game_ids = Vec::with_capacity(REQUEST_CHUNK_SIZE);
                }
            }
        }
    }

    Ok(HttpResponse::Ok())
}
