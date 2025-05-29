use crate::geo_guessr::{Entry, Payload};
use crate::requests::{geo_login, GamesData};
use actix_web::error::{ErrorBadRequest, ErrorInternalServerError};
use actix_web::{post, web, Error, HttpResponse, Responder};
use futures::future::join_all;
use log::error;
use reqwest::Client;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::Deserialize;
use std::collections::HashSet;
use crate::entities::duels_game;
use crate::entities::prelude::DuelsGame;
use crate::requests::insertion_requests::{get_game_data, insert_games_into_db};

const REQUEST_CHUNK_SIZE: usize = 50;

#[derive(Deserialize, Debug)]
struct ImportRecentGamesRequest {
    entries: Vec<Entry>,
}

fn remove_duplicates<T: ActiveModelTrait>(active_models: Vec<T>) -> Vec<T> {
    let mut seen = HashSet::new();
    let mut unique_active_models = Vec::new();

    for model in active_models {
        let key = model.get_primary_key_value().unwrap();
        if seen.insert(key) {
            unique_active_models.push(model);
        }
    }

    unique_active_models
}

async fn insert_games(
    game_ids: Vec<String>,
    db: &DatabaseConnection,
    client: &Client,
    cookies: String,
) -> Result<usize, Error> {
    let existing_games= match DuelsGame::find()
        .filter(duels_game::Column::Id.is_in(game_ids.clone()))
        .all(db)
        .await {
        Ok(games) => games,
        Err(err) => return Err(ErrorInternalServerError(err))
    };
    
    let existing_ids: HashSet<String> = existing_games.into_iter().map(|game| game.id).collect();
    let valid_game_ids: Vec<String> = game_ids.into_iter().filter(|id| !existing_ids.contains(id)).collect();
    
    let mut duels_games = Vec::new();
    let mut rounds = Vec::new();
    let mut guesses = Vec::new();
    let mut locations = Vec::new();
    let mut players = Vec::new();
    let mut comp_teams = Vec::new();
    let mut fun_teams = Vec::new();
    let mut maps = Vec::new();

    let mut futures = Vec::with_capacity(REQUEST_CHUNK_SIZE);
    futures.append(
        &mut valid_game_ids
            .iter()
            .map(|game_id| get_game_data(game_id.as_str(), client, cookies.clone(), db))
            .collect()
    );

    let results = join_all(futures).await;

    for mut game_data in results.into_iter().flatten() {
        duels_games.push(game_data.duels_game);
        rounds.append(&mut game_data.rounds);
        guesses.append(&mut game_data.guesses);
        locations.append(&mut game_data.locations);
        players.append(&mut game_data.players);
        comp_teams.append(&mut game_data.comp_teams);
        fun_teams.append(&mut game_data.fun_teams);
        maps.push(game_data.map);
    }

    guesses = remove_duplicates(guesses);
    locations = remove_duplicates(locations);
    players = remove_duplicates(players);
    comp_teams = remove_duplicates(comp_teams);
    fun_teams = remove_duplicates(fun_teams);
    maps = remove_duplicates(maps);

    let games_data = GamesData {
        duels_games,
        rounds,
        guesses,
        locations,
        players,
        comp_teams,
        fun_teams,
        maps,
    };

    insert_games_into_db(games_data, db).await?;

    Ok(valid_game_ids.len())
}

#[post("/import-games")]
async fn import_recent_games(
    request: web::Json<ImportRecentGamesRequest>,
    db: web::Data<DatabaseConnection>,
) -> Result<impl Responder, Error> {
    if request.entries.is_empty() {
        error!("Game History is empty!");
        return Err(ErrorBadRequest("Game History is empty!"));
    }

    let db = db.get_ref();
    let client = Client::new();
    let cookies = geo_login::get_cookies().await;
    let mut game_ids = Vec::new();

    for entry in request.entries.iter() {
        if let Ok(payloads) = serde_json::from_str::<Vec<Payload>>(&entry.payload) {
            game_ids.extend(
                payloads
                    .into_iter()
                    .filter(|payload| payload.payload.game_mode.as_str() != "LiveChallenge")
                    .map(|payload| payload.payload.game_id)
            );
            
            if game_ids.len() >= REQUEST_CHUNK_SIZE {
                break;
            }
        }
    }
    
    game_ids.truncate(REQUEST_CHUNK_SIZE);

    let inserted_games = insert_games(game_ids, db, &client, cookies).await?;

    Ok(HttpResponse::Ok().json(inserted_games))
}
