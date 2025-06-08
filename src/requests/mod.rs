use crate::entities::comp_team::ActiveModel as CompTeamModel;
use crate::entities::duels_game::ActiveModel as DuelsGameModel;
use crate::entities::duels_round::ActiveModel as DuelsRoundModel;
use crate::entities::fun_team::ActiveModel as FunTeamModel;
use crate::entities::guess::ActiveModel as GuessModel;
use crate::entities::location::ActiveModel as LocationModel;
use crate::entities::map::ActiveModel as MapModel;
use crate::entities::player::ActiveModel as PlayerModel;
use chrono::{DateTime, TimeDelta, Utc};
use country_boundaries::CountryBoundaries;
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use tokio::sync::Mutex;

pub mod insertion_requests;
pub mod general_stats_requests;
pub mod geo_login;
pub mod import_games;
mod country_stats_request;

const CASH_EXPIRE_TIME: TimeDelta = TimeDelta::seconds(90);

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
        String::from("CW"),
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
    
    static ref CASHED_ITEMS: Mutex<HashMap<String, DateTime<Utc>>> = Mutex::new(HashMap::new());
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