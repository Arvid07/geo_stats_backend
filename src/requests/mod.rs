use std::cmp::max;
use std::collections::HashSet;
use std::fs::File;
use crate::entities::duels_game::ActiveModel as DuelsGameModel;
use crate::entities::duels_round::ActiveModel as DuelsRoundModel;
use crate::entities::guess::ActiveModel as GuessModel;
use crate::entities::location::ActiveModel as LocationModel;
use crate::entities::player::ActiveModel as PlayerModel;
use crate::entities::comp_team::ActiveModel as CompTeamModel;
use crate::entities::fun_team::ActiveModel as FunTeamModel;
use crate::entities::map::ActiveModel as MapModel;
use actix_web::Error;
use actix_web::error::{ErrorBadRequest, ErrorInternalServerError, ErrorNotFound};
use chrono::{DateTime, Utc};
use country_boundaries::{CountryBoundaries, LatLon};
use lazy_static::lazy_static;
use log::{debug, error, info};
use reqwest::Client;
use reqwest::header::COOKIE;
use sea_orm::{sea_query, ActiveValue, DatabaseConnection, DbErr, EntityTrait, TransactionTrait};
use uuid::Uuid;
use crate::entities::{comp_team, map, player};
use crate::entities::prelude::{CompTeam, DuelsGame, DuelsRound, FunTeam, Guess, Location, Map, Player};
use crate::geo_guessr::{GameModeRatings, GeoMode, MovementOption, PlayerRankedSystemProgress, RankedTeam, RankedTeamDuelsProgress, TeamGameMode, User};
use crate::geo_guessr::GeoMode::{Moving, NoMove, NoMovingZooming, NoPanning, NoPanningMoving, NoPanningZooming, NoZooming, NMPZ};

pub mod insertion_requests;
pub mod get_requests;
pub mod geo_login;
pub mod import_games;

lazy_static! {
    static ref COUNTRY_BOUNDARIES: CountryBoundaries = CountryBoundaries::from_reader(
        File::open("world.ser")
        .expect("failed to open world.ser")
    ).expect("failed to load country boundaries");
    
    static ref STATE_BOUNDARIES: CountryBoundaries = CountryBoundaries::from_reader(
        File::open("states.ser")
        .expect("failed to open states.ser")
    ).expect("failed to load country boundaries");
    
    static ref PRIORITY_COUNTRIES: HashSet<String> = [
        String::from("CU"), 
        String::from("DO"),
        String::from("PR"), 
        String::from("VI"), 
        String::from("GU"), 
        String::from("MP"),
        String::from("HK"),
        String::from("CX"), 
        String::from("ST"), 
        String::from("SJ")
    ].into_iter().collect();
}

pub struct GamesData {
    pub duels_games: Vec<DuelsGameModel>,
    pub rounds: Vec<DuelsRoundModel>,
    pub guesses: Vec<GuessModel>,
    pub locations: Vec<LocationModel>,
    pub players: Vec<PlayerModel>,
    pub comp_teams: Vec<CompTeamModel>,
    pub fun_teams: Vec<FunTeamModel>,
    pub maps: Vec<MapModel>
}

pub struct GameData {
    pub duels_game: DuelsGameModel,
    pub rounds: Vec<DuelsRoundModel>,
    pub guesses: Vec<GuessModel>,
    pub locations: Vec<LocationModel>,
    pub players: Vec<PlayerModel>,
    pub comp_teams: Vec<CompTeamModel>,
    pub fun_teams: Vec<FunTeamModel>,
    pub map: MapModel
}

pub async fn create_new_comp_team(
    team_id: &str,
    player_id1: &String,
    player_id2: &String,
    team_progress: &RankedTeamDuelsProgress,
    client: &Client
) -> Result<comp_team::ActiveModel, Error> {
    let ranked_team_request_url = format!(
        "https://www.geoguessr.com/api/v4/ranked-team-duels/teams/?userId={}&userId={}",
        player_id1, player_id2
    );

    let team_response = client
        .get(ranked_team_request_url)
        .send()
        .await
        .map_err(|_| ErrorInternalServerError("Fetch Comp Team operation failed!"))?
        .json::<RankedTeam>()
        .await
        .map_err(|_| ErrorInternalServerError("Could not pass Json to Ranked Team"))?;

    let team = comp_team::ActiveModel {
        team_id: ActiveValue::Set(String::from(team_id)),
        player_id1: ActiveValue::Set(player_id1.clone()),
        player_id2: ActiveValue::Set(player_id2.clone()),
        name: ActiveValue::Set(team_response.team_name),
        rating: ActiveValue::Set(team_progress.rating_after)
    };

    Ok(team)
}

async fn create_fun_team_if_not_exists(
    team_id: String,
    player_ids: Vec<String>,
    db: &DatabaseConnection
) -> Result<Option<crate::entities::fun_team::ActiveModel>, Error> {
    let team_result = FunTeam::find_by_id(team_id.clone()).one(db).await;

    if let Ok(team_option) = team_result {
        if team_option.is_some() {
            return Ok(None);
        }

        let team = crate::entities::fun_team::ActiveModel {
            team_id: ActiveValue::Set(team_id),
            player_ids: ActiveValue::Set(player_ids)
        };

        Ok(Some(team))
    } else {
        Err(ErrorInternalServerError("Database operation get_fun_team failed!"))
    }
}

async fn insert_fun_team_duels_game_model(
    game: &crate::geo_guessr::DuelsGame,
    game_mode: &TeamGameMode,
    geo_mode: &GeoMode,
    db: &DatabaseConnection
) -> Result<(crate::entities::duels_game::ActiveModel, Vec<crate::entities::fun_team::ActiveModel>), Error> {
    let team_id1 = game.teams[0].id.clone();
    let team_id2 = game.teams[1].id.clone();
    let mut teams = Vec::new();

    if let Some(team) = create_fun_team_if_not_exists(
        team_id1.clone(),
        game.teams[0]
            .players
            .iter()
            .map(|player| player.player_id.clone())
            .collect(),
        db
    )
        .await? {
        teams.push(team);
    }

    if let Some(team) = create_fun_team_if_not_exists(
        team_id2.clone(),
        game.teams[1]
            .players
            .iter()
            .map(|player| player.player_id.clone())
            .collect(),
        db
    )
        .await? {
        teams.push(team);
    }

    let game_model =
        get_team_duels_game_model(game, game_mode, geo_mode, team_id1, team_id2, None, None);

    Ok((game_model, teams))
}

async fn insert_comp_team_duels_game_model(
    game: &crate::geo_guessr::DuelsGame,
    game_mode: &TeamGameMode,
    geo_mode: &GeoMode,
    client: &Client,
) -> Result<(crate::entities::duels_game::ActiveModel, Vec<comp_team::ActiveModel>), Error> {
    let team_id1 = game.teams[0].id.clone();
    let team_id2 = game.teams[1].id.clone();
    let mut teams = Vec::new();

    teams.push(
        create_new_comp_team(
            &team_id1,
            &game.teams[0].players[0].player_id,
            &game.teams[0].players[1].player_id,
            game.teams[0].players[0]
                .progress_change
                .as_ref()
                .unwrap()
                .ranked_team_duels_progress
                .as_ref()
                .unwrap(),
            client
        )
            .await?
    );

    teams.push(
        create_new_comp_team(
            &team_id2,
            &game.teams[1].players[0].player_id,
            &game.teams[1].players[1].player_id,
            game.teams[1].players[0]
                .progress_change
                .as_ref()
                .unwrap()
                .ranked_team_duels_progress
                .as_ref()
                .unwrap(),
            client
        )
            .await?
    );

    let mut rating_before_team1 = None;
    let mut rating_before_team2 = None;

    if let (Some(progress1), Some(progress2)) = (
        &game.teams[1].players[0].progress_change,
        &game.teams[0].players[0].progress_change
    ) {
        if let (Some(ranked_team_progress1), Some(ranked_team_progress2)) = (
            &progress1.ranked_team_duels_progress,
            &progress2.ranked_team_duels_progress
        ) {
            rating_before_team1 = ranked_team_progress1.rating_before;
            rating_before_team2 = ranked_team_progress2.rating_before;
        }
    }

    let game_model = get_team_duels_game_model(
        game,
        game_mode,
        geo_mode,
        team_id1,
        team_id2,
        rating_before_team1,
        rating_before_team2
    );

    Ok((game_model, teams))
}

fn get_team_duels_game_model(
    game: &crate::geo_guessr::DuelsGame,
    game_mode: &TeamGameMode,
    geo_mode: &GeoMode,
    team_id1: String,
    team_id2: String,
    rating_before_team1: Option<i32>,
    rating_before_team2: Option<i32>,
) -> crate::entities::duels_game::ActiveModel {
    crate::entities::duels_game::ActiveModel {
        id: ActiveValue::Set(game.game_id.clone()),
        team_id1: ActiveValue::Set(team_id1),
        team_id2: ActiveValue::Set(team_id2),
        health_team1: ActiveValue::Set(game.teams[0].health),
        health_team2: ActiveValue::Set(game.teams[1].health),
        team_game_mode: ActiveValue::Set(game_mode.to_string()),
        geo_mode: ActiveValue::Set(geo_mode.to_string()),
        start_time: ActiveValue::Set(game.rounds[0].start_time.clone().unwrap()),
        map_id: ActiveValue::Set(game.options.map.slug.clone()),
        rating_before_team1: ActiveValue::Set(rating_before_team1),
        rating_before_team2: ActiveValue::Set(rating_before_team2)
    }
}

async fn get_duels_game_model(
    game: &crate::geo_guessr::DuelsGame,
    game_mode: &TeamGameMode,
    geo_mode: &GeoMode,
) -> crate::entities::duels_game::ActiveModel {
    let players_team1 = &game.teams[0].players;
    let players_team2 = &game.teams[1].players;

    let mut rating_before_team1 = None;
    let mut rating_before_team2 = None;

    if let (Some(progress1), Some(progress2)) = (
        &game.teams[1].players[0].progress_change,
        &game.teams[0].players[0].progress_change,
    ) {
        if let (Some(ranked_team_progress1), Some(ranked_team_progress2)) = (
            &progress1.ranked_system_progress,
            &progress2.ranked_system_progress,
        ) {
            rating_before_team1 = ranked_team_progress1.rating_before;
            rating_before_team2 = ranked_team_progress2.rating_before;
        }
    }

    crate::entities::duels_game::ActiveModel {
        id: ActiveValue::Set(game.game_id.clone()),
        team_id1: ActiveValue::Set(players_team1[0].player_id.clone()),
        team_id2: ActiveValue::Set(players_team2[0].player_id.clone()),
        health_team1: ActiveValue::Set(game.teams[0].health),
        health_team2: ActiveValue::Set(game.teams[1].health),
        team_game_mode: ActiveValue::Set(game_mode.to_string()),
        geo_mode: ActiveValue::Set(geo_mode.to_string()),
        start_time: ActiveValue::Set(game.rounds[0].start_time.clone().unwrap()),
        map_id: ActiveValue::Set(game.options.map.slug.clone()),
        rating_before_team1: ActiveValue::Set(rating_before_team1),
        rating_before_team2: ActiveValue::Set(rating_before_team2)
    }
}

fn get_game_mode(team1_size: usize, team2_size: usize, is_rated: bool) -> TeamGameMode {
    if team1_size == 1 && team2_size == 1 {
        if is_rated {
            TeamGameMode::DuelsRanked
        } else {
            TeamGameMode::Duels
        }
    } else if team1_size == 2 && team2_size == 2 {
        if is_rated {
            TeamGameMode::TeamDuelsRanked
        } else {
            TeamGameMode::TeamDuels
        }
    } else {
        TeamGameMode::TeamFun
    }
}

fn get_geo_mode(movement_options: &MovementOption) -> GeoMode {
    match movement_options {
        MovementOption {
            forbid_moving: false,
            forbid_zooming: false,
            forbid_rotating: false
        } => Moving,
        MovementOption {
            forbid_moving: false,
            forbid_zooming: false,
            forbid_rotating: true
        } => NoPanning,
        MovementOption {
            forbid_moving: false,
            forbid_zooming: true,
            forbid_rotating: false
        } => NoZooming,
        MovementOption {
            forbid_moving: false,
            forbid_zooming: true,
            forbid_rotating: true
        } => NoPanningZooming,
        MovementOption {
            forbid_moving: true,
            forbid_zooming: false,
            forbid_rotating: false
        } => NoMove,
        MovementOption {
            forbid_moving: true,
            forbid_zooming: false,
            forbid_rotating: true
        } => NoPanningMoving,
        MovementOption {
            forbid_moving: true,
            forbid_zooming: true,
            forbid_rotating: false
        } => NoMovingZooming,
        MovementOption {
            forbid_moving: true,
            forbid_zooming: true,
            forbid_rotating: true
        } => NMPZ,
    }
}

pub async fn create_new_player_model(
    player_id: &str,
    client: &Client,
) -> Result<player::ActiveModel, Error> {
    let player_response = client
        .get(format!(
            "https://www.geoguessr.com/api/v3/users/{}",
            player_id
        ))
        .send()
        .await
        .map_err(|_| ErrorInternalServerError("Fetch User operation failed!"))?
        .json::<User>()
        .await
        .map_err(|_| ErrorNotFound(format!("User with id {} could not be found!", player_id)))?;

    let player_ratings_option = client
        .get(format!(
            "https://www.geoguessr.com/api/v4/ranked-system/progress/{}",
            player_id
        ))
        .send()
        .await
        .map_err(|_| {
            ErrorInternalServerError(format!(
                "Fetch Player Ratings operation failed for player {}!",
                player_id
            ))
        })?
        .json::<PlayerRankedSystemProgress>()
        .await
        .ok();

    let player_rating;
    let game_mode_ratings;

    if player_ratings_option.is_none() {
        player_rating = None;

        game_mode_ratings = GameModeRatings {
            standard_duels: None,
            no_move_duels: None,
            nmpz_duels: None
        }
    } else {
        let player_ratings = player_ratings_option.unwrap();
        player_rating = player_ratings.rating;

        if let Some(game_mode_ratings_result) = player_ratings.game_mode_ratings {
            game_mode_ratings = game_mode_ratings_result;
        } else {
            game_mode_ratings = GameModeRatings {
                standard_duels: None,
                no_move_duels: None,
                nmpz_duels: None
            }
        }
    }

    let player = player::ActiveModel {
        id: ActiveValue::Set(player_response.id),
        name: ActiveValue::Set(player_response.nick),
        country_code: ActiveValue::Set(player_response.country_code),
        rating: ActiveValue::Set(player_rating),
        moving_rating: ActiveValue::Set(game_mode_ratings.standard_duels),
        no_move_rating: ActiveValue::Set(game_mode_ratings.no_move_duels),
        nmpz_rating: ActiveValue::Set(game_mode_ratings.nmpz_duels),
        avatar_pin: ActiveValue::Set(player_response.pin.url),
        level: ActiveValue::Set(player_response.br.level),
        is_pro_user: ActiveValue::Set(player_response.is_pro_user),
        is_creator: ActiveValue::Set(player_response.is_creator)
    };

    Ok(player)
}

async fn get_player_model(
    player_id: &str,
    db: &DatabaseConnection,
    client: &Client,
) -> Result<Result<player::ActiveModel, player::ActiveModel>, Error> {
    if let Ok(player_option) = Player::find_by_id(player_id).one(db).await {
        match player_option {
            Some(_) => Ok(Ok(create_new_player_model(player_id, client).await?)),
            None => Ok(Err(create_new_player_model(player_id, client).await?))
        }
    } else {
        Err(ErrorInternalServerError(
            "Database operation get_player failed!"
        ))
    }
}

pub async fn get_game_data_if_not_exists(
    game_id: &str,
    client: &Client,
    cookies: String,
    db: &DatabaseConnection,
) -> Result<GameData, Error> {
    match DuelsGame::find_by_id(game_id).one(db).await {
        Ok(game_option) => match game_option {
            Some(_) => Err(ErrorBadRequest("Game already exists")),
            None => Ok(get_game_data(game_id, client, cookies, db).await?),
        },
        Err(err) => Err(ErrorInternalServerError(err)),
    }
}

async fn insert_game_into_db(game_data: GameData, db: &DatabaseConnection) -> Result<(), Error> {
    let games_data = GamesData {
        duels_games: vec![game_data.duels_game],
        rounds: game_data.rounds,
        guesses: game_data.guesses,
        locations: game_data.locations,
        players: game_data.players,
        comp_teams: game_data.comp_teams,
        fun_teams: game_data.fun_teams,
        maps: vec![game_data.map]
    };
    
    insert_games_into_db(games_data, db).await
}

async fn insert_games_into_db(games_data: GamesData, db: &DatabaseConnection) -> Result<(), Error> {
    match db.transaction::<_, _, DbErr>(|txn| {
        Box::pin(async move {
            DuelsGame::insert_many(games_data.duels_games).exec(txn).await?;

            if !games_data.guesses.is_empty() {
                Guess::insert_many(games_data.guesses).exec(txn).await?;
            }
            if !games_data.rounds.is_empty() {
                DuelsRound::insert_many(games_data.rounds).exec(txn).await?;
            }
            if !games_data.players.is_empty() {
                Player::insert_many(games_data.players)
                    .on_conflict(
                        sea_query::OnConflict::column(player::Column::Id)
                            .update_columns(
                                [
                                    player::Column::Name,
                                    player::Column::CountryCode,
                                    player::Column::AvatarPin,
                                    player::Column::Level,
                                    player::Column::IsProUser,
                                    player::Column::IsCreator,
                                    player::Column::Rating,
                                    player::Column::MovingRating,
                                    player::Column::NoMoveRating,
                                    player::Column::NmpzRating
                                ])
                            .to_owned()
                    )
                    .exec(txn)
                    .await?;
            }
            if !games_data.comp_teams.is_empty() {
                CompTeam::insert_many(games_data.comp_teams)
                    .on_conflict(
                        sea_query::OnConflict::column(comp_team::Column::TeamId)
                            .update_columns([comp_team::Column::Name, comp_team::Column::Rating])
                            .to_owned()
                    )
                    .exec(txn)
                    .await?;
            }
            if !games_data.fun_teams.is_empty() {
                FunTeam::insert_many(games_data.fun_teams)
                    .exec(txn)
                    .await?;
            }
            if !games_data.locations.is_empty() {
                Location::insert_many(games_data.locations)
                    .on_conflict_do_nothing()
                    .exec(txn)
                    .await?;
            }
            if !games_data.maps.is_empty() {
                Map::insert_many(games_data.maps)
                    .on_conflict(
                        sea_query::OnConflict::column(map::Column::Id)
                            .update_columns([map::Column::Name, map::Column::MaxDistance])
                            .to_owned()
                    )
                    .exec(txn)
                    .await?;
            }

            Ok(())
        })
    }).await {
        Ok(()) => info!("All inserts succeeded"),
        Err(err) => {
            error!("Insertion failed, Rolling back: {}", err);

            return if err.to_string().contains("duplicate key value violates unique constraint") {
                Err(ErrorBadRequest("Game with id {} does already exist!"))
            } else {
                Err(ErrorInternalServerError(err.to_string()))
            };
        }
    }
    
    Ok(())
}

pub async fn get_game_data(
    game_id: &str,
    client: &Client,
    cookies: String,
    db: &DatabaseConnection,
) -> Result<GameData, Error> {
    let mut rounds = Vec::new();
    let mut guesses = Vec::new();
    let mut locations = Vec::new();
    let mut players = Vec::new();
    let mut comp_teams = Vec::new();
    let mut fun_teams = Vec::new();

    let game = client
        .get(format!("https://game-server.geoguessr.com/api/duels/{}", game_id))
        .header(COOKIE, cookies)
        .send()
        .await
        .map_err(|_| ErrorInternalServerError("Fetch Game operation failed!"))?
        .json::<crate::geo_guessr::DuelsGame>()
        .await
        .map_err(|_| ErrorBadRequest(format!("Could not find Game with id: {}!", game_id)))?;

    if game.status.as_str() != "Finished" {
        return Err(ErrorBadRequest("Game has not finished yet!"));
    }

    let game_mode = get_game_mode(
        game.teams[0].players.len(),
        game.teams[1].players.len(),
        game.options.is_rated,
    );
    let geo_mode = get_geo_mode(&game.options.movement_options);

    for team in game.teams.iter() {
        for player in team.players.iter() {
            match create_new_player_model(&player.player_id, client).await {
                Ok(new_player) => players.push(new_player),
                Err(internal_server_error) => return Err(internal_server_error)
            }
        }
    }

    let duels_game;

    match &game_mode {
        TeamGameMode::Duels | TeamGameMode::DuelsRanked => {
            duels_game = get_duels_game_model(&game, &game_mode, &geo_mode).await;
        }
        TeamGameMode::TeamDuelsRanked => {
            match insert_comp_team_duels_game_model(&game, &game_mode, &geo_mode, client).await
            {
                Ok((duels_game_model, teams)) => {
                    duels_game = duels_game_model;
                    comp_teams = teams;
                }
                Err(error) => return Err(error)
            };
        }
        TeamGameMode::TeamDuels | TeamGameMode::TeamFun => {
            match insert_fun_team_duels_game_model(&game, &game_mode, &geo_mode, db).await {
                Ok((duels_game_model, team_models)) => {
                    duels_game = duels_game_model;
                    fun_teams = team_models;
                }
                Err(error) => return Err(error)
            }
        }
    }

    let mut max_distance_option = None;

    for (round_number, round) in game
        .rounds
        .iter()
        .filter(|round| round.round_number <= game.current_round_number)
        .enumerate()
    {
        let panorama = &round.panorama;
        let round_id = Uuid::new_v4().to_string();

        let subdivision_codes = STATE_BOUNDARIES.ids(LatLon::new(panorama.lat, panorama.lng).unwrap());
        let subdivision_code = subdivision_codes.into_iter().next().map(String::from);
        
        let location = crate::entities::location::ActiveModel {
            id: ActiveValue::Set(panorama.pano_id.clone()),
            lat: ActiveValue::Set(panorama.lat),
            lng: ActiveValue::Set(panorama.lng),
            heading: ActiveValue::Set(panorama.heading),
            pitch: ActiveValue::Set(panorama.pitch),
            zoom: ActiveValue::Set(panorama.zoom),
            country_code: ActiveValue::Set(panorama.country_code.clone()),
            subdivision_code: ActiveValue::Set(subdivision_code)
        };

        locations.push(location);

        let start_time_option = &round.start_time;
        let round_starting_date: DateTime<Utc> = start_time_option.clone().unwrap().parse().unwrap();

        for (i, team) in game.teams.iter().enumerate() {
            for player in team.players.iter() {
                let geo_guess_option = player
                    .guesses
                    .iter()
                    .find(|guess| guess.round_number as usize - 1 == round_number);

                if let Some(geo_guess) = geo_guess_option {
                    let guess_date: DateTime<Utc> = geo_guess.created.parse().unwrap();

                    let team_id = match &game_mode {
                        TeamGameMode::Duels | TeamGameMode::DuelsRanked => player.player_id.clone(),
                        TeamGameMode::TeamDuels | TeamGameMode::TeamDuelsRanked | TeamGameMode::TeamFun => game.teams[i].id.clone()
                    };
                    
                    let score = geo_guess.score.unwrap_or_else(|| {
                        let max_distance = max_distance_option.unwrap_or_else(|| {
                            let a = geoutils::Location::new(
                                game.map_bounds.min.lat,
                                game.map_bounds.min.lng,
                            );
                            let b = geoutils::Location::new(
                                game.map_bounds.max.lat,
                                game.map_bounds.max.lng,
                            );
                            let distance = a.distance_to(&b).unwrap();
                            max_distance_option = Some(distance.meters());

                            distance.meters()
                        });

                        (5000_f64 * std::f64::consts::E.powf(-10_f64 * (geo_guess.distance / max_distance))) as i32
                    });

                    let subdivision_codes = STATE_BOUNDARIES.ids(LatLon::new(geo_guess.lat, geo_guess.lng).unwrap());
                    let subdivision_code = subdivision_codes.clone().into_iter().next().map(String::from); // remove clone later 
                    
                    let mut codes = COUNTRY_BOUNDARIES.ids(LatLon::new(geo_guess.lat, geo_guess.lng).unwrap());
                    let mut country_code = codes.pop().map(String::from);

                    for code in codes {
                        if PRIORITY_COUNTRIES.contains(code) {
                            country_code = Some(String::from(code));
                            break;
                        }
                    }
                    
                    let guess = GuessModel {
                        id: ActiveValue::Set(Uuid::new_v4().to_string()),
                        game_id: ActiveValue::Set(String::from(game_id)),
                        round_id: ActiveValue::Set(round_id.clone()),
                        team_id: ActiveValue::Set(team_id),
                        lat: ActiveValue::Set(geo_guess.lat),
                        lng: ActiveValue::Set(geo_guess.lng),
                        score: ActiveValue::Set(score),
                        time: ActiveValue::Set(Some((guess_date - round_starting_date).num_seconds() as i32)),
                        distance: ActiveValue::Set(geo_guess.distance),
                        country_code: ActiveValue::Set(country_code),
                        subdivision_code: ActiveValue::Set(subdivision_code),
                        round_country_code: ActiveValue::Set(round.panorama.country_code.clone()),
                        is_teams_best: ActiveValue::Set(geo_guess.is_teams_best_guess_on_round)
                    };

                    guesses.push(guess);
                }
            }
        }

        let round = crate::entities::duels_round::ActiveModel {
            id: ActiveValue::Set(round_id),
            game_id: ActiveValue::Set(game.game_id.clone()),
            location_id: ActiveValue::Set(panorama.pano_id.clone()),
            round_number: ActiveValue::Set(round_number as i32),
            damage_multiplier: ActiveValue::Set(round.damage_multiplier)
        };

        rounds.push(round);
    }

    let map = MapModel {
        id: ActiveValue::Set(game.options.map.slug.clone()),
        name: ActiveValue::Set(game.options.map.name.clone()),
        lat1: ActiveValue::Set(game.map_bounds.min.lat),
        lng1: ActiveValue::Set(game.map_bounds.min.lng),
        lat2: ActiveValue::Set(game.map_bounds.max.lat),
        lng2: ActiveValue::Set(game.map_bounds.max.lng),
        max_distance: ActiveValue::Set(game.options.map.max_error_distance)
    };

    let game_data = GameData {
        duels_game,
        rounds,
        guesses,
        locations,
        players,
        comp_teams,
        fun_teams,
        map
    };

    Ok(game_data)
}
