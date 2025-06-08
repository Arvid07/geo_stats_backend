use crate::entities::player::Model as PlayerModel;
use crate::entities::prelude::{CompTeam, DuelsGame, Guess};
use crate::entities::{comp_team, duels_game, duels_round, guess, location};
use crate::geo_guessr::TeamGameMode;
use crate::login::get_player_from_session;
use actix_web::error::{ErrorInternalServerError, ErrorUnauthorized};
use actix_web::{get, web, Error, HttpRequest, HttpResponse, Responder};
use sea_orm::QueryFilter;
use sea_orm::{ColumnTrait, LoaderTrait};
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::Serialize;
use std::collections::HashSet;
use std::str::FromStr;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SingleGuess {
    time: u64,
    points: u64,
    lat: f64,
    lon: f64,
    country_code: String,
    subdivision_code: String
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsGuess {
    round_start_time: i64,
    player_guess: Option<SingleGuess>,
    enemy_guess: Option<SingleGuess>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TeamStatsGuess {
    round_start_time: i64,
    round_subdivision_code: String,
    player_guess: Vec<SingleGuess>,
    enemy_guess: Vec<SingleGuess>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Stats {
    // solo: Vec<Guess>,
    duels: Vec<StatsGuess>,
    duels_ranked: Vec<StatsGuess>,
    team_duels: Vec<TeamStatsGuess>,
    team_duels_ranked: Vec<TeamStatsGuess>,
    team_fun: Vec<TeamStatsGuess>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CountryStatsResponse {
    player: PlayerModel,
    stats: Option<Stats>
}

#[get("country/{country_code}")]
pub async fn get_country_stats(
    db: web::Data<DatabaseConnection>,
    path: web::Path<String>,
    http_request: HttpRequest
) -> Result<impl Responder, Error> {
    let session_id = match http_request.cookie("sessionId") {
        Some(cookie) => {
            String::from(cookie.value())
        },
        None => return Err(ErrorUnauthorized("Missing `sessionId` cookie!"))
    };

    let db = db.get_ref();
    let country_code = path.into_inner();
    let player = get_player_from_session(&session_id, db).await?;
    let mut team_ids: HashSet<String> = [player.id.clone()].into_iter().collect();

    match CompTeam::find()
        .filter(comp_team::Column::PlayerId1.eq(&player.id).or(comp_team::Column::PlayerId2.eq(&player.id)))
        .all(db)
        .await
    {
        Ok(teams) => team_ids.extend(teams.into_iter().map(|team| team.team_id)),
        Err(err) => return Err(ErrorInternalServerError(err))
    };

    let games = DuelsGame::find()
        .filter(duels_game::Column::TeamId1.is_in(&team_ids).or(duels_game::Column::TeamId2.is_in(&team_ids)))
        .all(db)
        .await
        .unwrap();
    
    let games_rounds = games
        .load_many(duels_round::Entity, db)
        .await
        .unwrap();
    
    let rounds: Vec<duels_round::Model> = games_rounds.clone().into_iter().flatten().collect();
    let rounds_guesses = rounds.load_many(guess::Entity, db).await.unwrap();
    let locations = rounds.load_one(location::Entity, db).await.unwrap();
    

    Ok(HttpResponse::Ok().body(""))
}