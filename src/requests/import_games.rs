use crate::entities::prelude::{CompTeam, DuelsGame, DuelsRound, FunTeam, Guess, Player};
use crate::geo_guessr::{Entry, Payload};
use crate::requests::geo_login;
use crate::requests::insertion_requests::{get_game_data_if_not_exists};
use actix_web::error::{ErrorBadRequest, ErrorInternalServerError};
use actix_web::{post, web, Error, HttpResponse, Responder};
use futures::future::join_all;
use log::{error, info};
use reqwest::Client;
use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, TransactionTrait};
use serde::Deserialize;
use std::collections::HashSet;

const REQUEST_CHUNK_SIZE: usize = 60;

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
    _user_id: &str,
    game_ids: Vec<String>,
    db: &DatabaseConnection,
    client: &Client,
    cookies: String,
) -> Result<(), Error> {
    let mut duels_games = Vec::new();
    let mut rounds = Vec::new();
    let mut guesses = Vec::new();
    let mut locations = Vec::new();
    let mut insert_players = Vec::new();
    let mut update_players = Vec::new();
    let mut insert_comp_teams = Vec::new();
    let mut update_comp_teams = Vec::new();
    let mut insert_fun_teams = Vec::new();
    let mut maps = Vec::new();

    while !game_ids.is_empty() {
        let mut futures = Vec::with_capacity(REQUEST_CHUNK_SIZE);
        futures.append(
            &mut game_ids
                .iter()
                .take(REQUEST_CHUNK_SIZE)
                .map(|game_id| get_game_data_if_not_exists(game_id.as_str(), client, cookies.clone(), db))
                .collect()
        );

        let results = join_all(futures).await;

        for result in results {
            match result {
                Ok(mut game_data) => {
                    duels_games.push(game_data.duels_game);
                    rounds.append(&mut game_data.rounds);
                    guesses.append(&mut game_data.guesses);
                    locations.append(&mut game_data.locations);
                    insert_players.append(&mut game_data.insert_players);
                    update_players.append(&mut game_data.update_players);
                    insert_comp_teams.append(&mut game_data.insert_comp_teams);
                    update_comp_teams.append(&mut game_data.update_comp_teams);
                    insert_fun_teams.append(&mut game_data.insert_fun_teams);
                    maps.push(game_data.map);
                }
                Err(err) => {
                    error!("{}", err);
                }
            }
        }

        if true {
            break;
        }
    }

    guesses = remove_duplicates(guesses);
    locations = remove_duplicates(locations);
    insert_players = remove_duplicates(insert_players);
    update_players = remove_duplicates(update_players);
    insert_comp_teams = remove_duplicates(insert_comp_teams);
    update_comp_teams = remove_duplicates(update_comp_teams);
    insert_fun_teams = remove_duplicates(insert_fun_teams);
    maps = remove_duplicates(maps);

    match db
        .transaction::<_, _, DbErr>(|txn| {
            Box::pin(async move {
                if !duels_games.is_empty() {
                    DuelsGame::insert_many(duels_games).exec(txn).await?;
                }

                if !guesses.is_empty() {
                    Guess::insert_many(guesses).exec(txn).await?;
                }
                if !rounds.is_empty() {
                    DuelsRound::insert_many(rounds).exec(txn).await?;
                }
                if !insert_players.is_empty() {
                    Player::insert_many(insert_players).exec(txn).await?;
                }
                for player in update_players {
                    player.update(txn).await?;
                }
                if !insert_comp_teams.is_empty() {
                    CompTeam::insert_many(insert_comp_teams).exec(txn).await?;
                }
                for team in update_comp_teams {
                    team.update(txn).await?;
                }
                if !insert_fun_teams.is_empty() {
                    FunTeam::insert_many(insert_fun_teams).exec(txn).await?;
                }

                Ok(())
            })
        })
        .await
    {
        Ok(()) => info!("All inserts succeeded"),
        Err(err) => {
            error!("Insertion failed, Rolling back: {}", err);
            return Err(ErrorInternalServerError(err.to_string()));
        }
    }

    for location in locations {
        let _ = location.insert(db).await;
    }

    for map in maps {
        let _ = map.insert(db).await;
    }

    Ok(())
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
    let user_id = request.entries[0].user.id.clone();

    let mut game_ids = Vec::new();

    for entry in request.entries.iter() {
        if let Ok(payloads) = serde_json::from_str::<Vec<Payload>>(&entry.payload) {
            game_ids.extend(
                payloads
                    .into_iter()
                    .filter(|payload| payload.payload.game_mode.as_str() != "LiveChallenge")
                    .map(|payload| payload.payload.game_id)
            );
        }
    }

    insert_games(&user_id, game_ids, db, &client, cookies).await?;

    Ok(HttpResponse::Ok())
}
