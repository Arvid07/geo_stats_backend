use crate::entities::player::Model as PlayerModel;
use crate::entities::prelude::{CompTeam, DuelsGame};
use crate::entities::{comp_team, duels_game, duels_round, guess, location};
use crate::geo_guessr::TeamGameMode;
use crate::login::get_player_from_session;
use actix_web::error::{ErrorInternalServerError, ErrorUnauthorized};
use actix_web::{get, web, Error, HttpRequest, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use sea_orm::QueryFilter;
use sea_orm::{ColumnTrait, LoaderTrait};
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SingleGuess {
    time: Option<i32>,
    points: i32,
    lat: f64,
    lon: f64,
    country_code: Option<String>,
    subdivision_code: Option<String>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatsGuess {
    game_start_time: i64,
    round_subdivision_code: Option<String>,
    player_guess: Option<SingleGuess>,
    enemy_guess: Option<SingleGuess>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TeamStatsGuess {
    game_start_time: i64,
    round_subdivision_code: Option<String>,
    team_guesses: Vec<SingleGuess>,
    enemy_team_guesses: Vec<SingleGuess>
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

fn get_team_duels_guess(guesses: Vec<guess::Model>, team_ids: &HashSet<String>, date: DateTime<Utc>, location: location::Model) -> TeamStatsGuess {
    let mut team_guesses = Vec::new();
    let mut enemy_team_guesses = Vec::new();

    for guess in guesses {
        let single_guess = SingleGuess {
            time: guess.time,
            points: guess.score,
            lat: guess.lat,
            lon: guess.lng,
            country_code: guess.country_code,
            subdivision_code: guess.subdivision_code,
        };

        if team_ids.contains(&guess.team_id) {
            team_guesses.push(single_guess);
        } else {
            enemy_team_guesses.push(single_guess);
        }
    }

    TeamStatsGuess {
        game_start_time: date.timestamp_millis(),
        round_subdivision_code: location.subdivision_code,
        team_guesses,
        enemy_team_guesses
    }
}

fn get_duels_guess(guesses: Vec<guess::Model>, team_ids: &HashSet<String>, date: DateTime<Utc>, location: location::Model) -> StatsGuess {
    let mut player_guess = None;
    let mut enemy_guess = None;

    for guess in guesses {
        let single_guess = SingleGuess {
            time: guess.time,
            points: guess.score,
            lat: guess.lat,
            lon: guess.lng,
            country_code: guess.country_code,
            subdivision_code: guess.subdivision_code,
        };

        if team_ids.contains(&guess.team_id) {
            player_guess = Some(single_guess);
        } else {
            enemy_guess = Some(single_guess);
        }
    }

    StatsGuess {
        game_start_time: date.timestamp_millis(),
        round_subdivision_code: location.subdivision_code,
        player_guess,
        enemy_guess
    }
}

#[get("/country/{country_code}")]
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
    let country_code = path.into_inner().to_ascii_uppercase();
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
        .map_err(ErrorInternalServerError)?;
    
    let games_rounds = games
        .load_many(
            duels_round::Entity::find().filter(duels_round::Column::RoundCountryCode.eq(&country_code)),
            db
        )
        .await
        .map_err(ErrorInternalServerError)?;

    let game_values: HashMap<String, (String, String)> = games.into_iter().map(|game| (game.id, (game.team_game_mode, game.start_time))).collect();

    let mut duels = Vec::new();
    let mut duels_ranked = Vec::new();
    let mut team_duels = Vec::new();
    let mut team_duels_ranked = Vec::new();
    let mut team_fun = Vec::new();
    
    let rounds: Vec<duels_round::Model> = games_rounds.into_iter().flatten().collect();
    let rounds_guesses = rounds.load_many(guess::Entity, db).await.map_err(ErrorInternalServerError)?;
    let locations = rounds.load_one(location::Entity, db).await.map_err(ErrorInternalServerError)?;
    
    for ((round, guesses), location) in rounds.into_iter().zip(rounds_guesses).zip(locations) {
        if let Some(location) = location {
            let (team_game_mode_str, game_start_time) = game_values.get(&round.game_id).unwrap();
            let date: DateTime<Utc> = game_start_time.parse().unwrap();
            
            match TeamGameMode::from_str(team_game_mode_str).unwrap() {
                TeamGameMode::Duels => duels.push(get_duels_guess(guesses, &team_ids, date, location)),
                TeamGameMode::DuelsRanked => duels_ranked.push(get_duels_guess(guesses, &team_ids, date, location)),
                TeamGameMode::TeamDuels => team_duels.push(get_team_duels_guess(guesses, &team_ids, date, location)),
                TeamGameMode::TeamDuelsRanked => team_duels_ranked.push(get_team_duels_guess(guesses, &team_ids, date, location)),
                TeamGameMode::TeamFun => team_fun.push(get_team_duels_guess(guesses, &team_ids, date, location))
            }
        }
    }
    
    duels.sort_unstable_by(|a, b| b.game_start_time.cmp(&a.game_start_time));
    duels_ranked.sort_unstable_by(|a, b| b.game_start_time.cmp(&a.game_start_time));
    team_duels.sort_unstable_by(|a, b| b.game_start_time.cmp(&a.game_start_time));
    team_duels_ranked.sort_unstable_by(|a, b| b.game_start_time.cmp(&a.game_start_time));
    team_fun.sort_unstable_by(|a, b| b.game_start_time.cmp(&a.game_start_time));
    
    let stats = Stats {
        duels,
        duels_ranked,
        team_duels,
        team_duels_ranked,
        team_fun
    };
    
    let response = CountryStatsResponse {
        player,
        stats: Some(stats)
    };
    
    Ok(HttpResponse::Ok().json(response))
}