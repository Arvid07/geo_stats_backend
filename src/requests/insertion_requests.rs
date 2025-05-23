use crate::entities::guess::ActiveModel as GuessModel;
use crate::entities::location::ActiveModel as LocationModel;
use crate::entities::map::ActiveModel as MapModel;
use crate::entities::prelude::{Guess, Location, Map, Player, SoloGame, SoloRound};
use crate::entities::solo_game::ActiveModel as SoloGameModel;
use crate::entities::solo_round::ActiveModel as SoloRoundModel;
use crate::geo_guessr::MovementOption;
use crate::requests::{geo_login, get_game_data, get_geo_mode, get_player_model, insert_game_into_db, COUNTRY_BOUNDARIES, PRIORITY_COUNTRIES, STATE_BOUNDARIES};
use actix_web::error::{ErrorBadRequest, ErrorInternalServerError};
use actix_web::{post, web, Error, HttpResponse, Responder};
use chrono::{DateTime, TimeDelta, Utc};
use country_boundaries::LatLon;
use log::{error, info};
use reqwest::Client;
use sea_orm::{ActiveValue, DatabaseConnection, DbErr, EntityTrait, TransactionTrait};
use uuid::Uuid;

#[post("/duels-game/{game_id}")]
async fn insert_duels_game(
    path: web::Path<String>,
    db: web::Data<DatabaseConnection>,
) -> Result<impl Responder, Error> {
    let client = Client::new();
    let cookies = geo_login::get_cookies().await;
    let db = db.get_ref();
    let game_id = path.into_inner();

    let game_data = get_game_data(&game_id, &client, cookies, db).await?;
    insert_game_into_db(game_data, db).await?;

    Ok(HttpResponse::Created().body(""))
}

#[post("/solo-game/{game_id}")]
async fn insert_solo_game(
    path: web::Path<String>,
    db: web::Data<DatabaseConnection>,
) -> Result<impl Responder, Error> {
    let client = Client::new();
    let game_id = path.into_inner();

    let game = client
        .get(format!("https://www.geoguessr.com/api/v3/games/{}", game_id))
        .send()
        .await
        .map_err(|err| ErrorInternalServerError(format!("Fetch Game operation failed! Error: {}", err)))?
        .json::<crate::geo_guessr::SoloGame>()
        .await
        .map_err(|_| ErrorBadRequest(format!("Could not find Game with id: {}!", game_id)))?;

    if game.state.as_str() != "finished" {
        return Err(ErrorBadRequest("Game has not finished yet!"));
    }

    let db = db.get_ref();
    let geo_mode = get_geo_mode(&MovementOption {
        forbid_moving: game.forbid_moving,
        forbid_zooming: game.forbid_zooming,
        forbid_rotating: game.forbid_rotating,
    });

    let mut insert_player = None;
    let mut rounds = Vec::with_capacity(game.round as usize);
    let mut guesses = Vec::with_capacity(game.round as usize);
    let mut locations = Vec::with_capacity(game.round as usize);

    match get_player_model(&game.player.id, db, &client).await {
        Ok(player_option) => {
            if let Err(player_model) = player_option {
                insert_player = Some(player_model);
            }
        }
        Err(internal_server_error) => return Err(internal_server_error),
    }

    for (round_number, round) in game.rounds.iter().enumerate() {
        let guess_id = Uuid::new_v4().to_string();
        let round_id = Uuid::new_v4().to_string();

        let subdivision_codes = STATE_BOUNDARIES.ids(LatLon::new(round.lat, round.lng).unwrap());
        let subdivision_code = subdivision_codes.into_iter().next().map(String::from);

        let mut codes = COUNTRY_BOUNDARIES.ids(LatLon::new(game.player.guesses[round_number].lat, game.player.guesses[round_number].lng).unwrap());
        let mut country_code = codes.pop().map(String::from);

        for code in codes {
            if PRIORITY_COUNTRIES.contains(code) {
                country_code = Some(String::from(code));
                break;
            }
        }

        let location = LocationModel {
            id: ActiveValue::Set(round.pano_id.clone()),
            lat: ActiveValue::Set(round.lat),
            lng: ActiveValue::Set(round.lng),
            heading: ActiveValue::Set(round.heading),
            pitch: ActiveValue::Set(round.pitch),
            zoom: ActiveValue::Set(round.zoom),
            country_code: ActiveValue::Set(round.streak_location_code.clone()),
            subdivision_code: ActiveValue::Set(subdivision_code.clone())
        };

        locations.push(location);
        
        let round_start_time: DateTime<Utc> = round.start_time.clone().unwrap().parse().unwrap();
        let guess_time = game.player.guesses[round_number].time;
        
        let guess = GuessModel {
            id: ActiveValue::Set(guess_id.clone()),
            game_id: ActiveValue::Set(game_id.clone()),
            round_id: ActiveValue::Set(round_id.clone()),
            team_id: ActiveValue::Set(game.player.id.clone()),
            lat: ActiveValue::Set(game.player.guesses[round_number].lat),
            lng: ActiveValue::Set(game.player.guesses[round_number].lng),
            score: ActiveValue::Set(game.player.guesses[round_number].round_score_in_points),
            time: ActiveValue::Set(Some(guess_time)),
            date: ActiveValue::Set((round_start_time + TimeDelta::seconds(guess_time as i64)).to_string()),
            distance: ActiveValue::Set(game.player.guesses[round_number].distance_in_meters),
            country_code: ActiveValue::Set(country_code),
            subdivision_code: ActiveValue::Set(subdivision_code),
            round_country_code: ActiveValue::Set(round.streak_location_code.clone()),
            is_teams_best: ActiveValue::Set(true)
        };

        guesses.push(guess);

        let round = SoloRoundModel {
            id: ActiveValue::Set(round_id),
            game_id: ActiveValue::Set(game.token.clone()),
            location_id: ActiveValue::Set(round.pano_id.clone()),
            round_number: ActiveValue::Set(round_number as i32),
        };

        rounds.push(round);
    }

    let solo_game = SoloGameModel {
        id: ActiveValue::Set(game.token.clone()),
        player_id: ActiveValue::Set(game.player.id.clone()),
        geo_mode: ActiveValue::Set(geo_mode.to_string()),
        start_time: ActiveValue::Set(game.rounds[0].start_time.clone().unwrap()),
        map_id: ActiveValue::Set(game.map.clone()),
    };

    let a = geoutils::Location::new(game.bounds.min.lat, game.bounds.min.lng);
    let b = geoutils::Location::new(game.bounds.max.lat, game.bounds.max.lng);
    let distance = a.distance_to(&b).unwrap();
    let distance_meters = distance.meters();

    let map = MapModel {
        id: ActiveValue::Set(game.map.clone()),
        name: ActiveValue::Set(game.map_name.clone()),
        lat1: ActiveValue::Set(game.bounds.min.lat),
        lng1: ActiveValue::Set(game.bounds.min.lng),
        lat2: ActiveValue::Set(game.bounds.max.lat),
        lng2: ActiveValue::Set(game.bounds.max.lng),
        max_distance: ActiveValue::Set(distance_meters as i32),
    };

    match db
        .transaction::<_, _, DbErr>(|txn| {
            Box::pin(async move {
                SoloGame::insert(solo_game).exec(txn).await?;
                SoloRound::insert_many(rounds).exec(txn).await?;
                Guess::insert_many(guesses).exec(txn).await?;
                Map::insert(map).on_conflict_do_nothing().exec(txn).await?;
                Location::insert_many(locations)
                    .on_conflict_do_nothing()
                    .exec(txn)
                    .await?;

                if let Some(player) = insert_player {
                    Player::insert(player).exec(txn).await?;
                }

                Ok(())
            })
        })
        .await
    {
        Ok(()) => info!("All inserts succeeded"),
        Err(err) => {
            error!("Insertion failed, Rolling back: {}", err);

            return if err.to_string().contains("duplicate key value violates unique constraint") {
                Err(ErrorBadRequest(format!("Game with id {} does already exist!", game_id)))
            } else {
                Err(ErrorInternalServerError(err.to_string()))
            };
        }
    }

    Ok(HttpResponse::Created().body(""))
}
