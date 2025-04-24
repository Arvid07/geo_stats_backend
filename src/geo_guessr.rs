use std::fmt;
use serde::Deserialize;

#[derive(Debug)]
pub enum GeoMode {
    Moving,
    NoMove,
    NMPZ,
    NoPanningZooming,
    NoPanningMoving,
    NoMovingZooming,
    NoPanning,
    NoZooming
}

#[derive(Debug, PartialEq)]
pub enum TeamGameMode {
    Duels,
    DuelsRanked,
    TeamDuels,
    TeamDuelsRanked,
    TeamFun
}

impl fmt::Display for GeoMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for TeamGameMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RankedTeam {
    pub team_id: String,
    pub division_number: i32,
    pub division_name: String,
    pub rating: Option<i32>,
    pub tier: String,
    pub team_name: String
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlayerRankedSystemProgress {
    pub division_number: i32,
    pub division_name: String,
    pub rating: Option<i32>,
    pub tier: String,
    pub game_mode_ratings: Option<GameModeRatings>,
    pub guessed_first_rate: f64,
    pub win_streak: i32
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameModeRatings {
    pub standard_duels: Option<i32>,
    pub no_move_duels: Option<i32>,
    pub nmpz: Option<i32> // TODO: might be wrong
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub nick: String,
    pub created: String,
    pub is_pro_user: bool,
    #[serde(rename = "type")]
    pub type_entry: String,
    pub consumed_trial: bool,
    pub is_verified: bool,
    pub pin: UserPin,
    pub full_body_pin: String,
    pub color: f64,
    pub url: String,
    pub id: String,
    pub country_code: String,
    pub br: Br,
    pub is_banned: bool,
    pub chat_ban: bool,
    pub avatar: Avatar,
    pub is_bot_user: bool,
    pub suspended_until: Option<String>,
    pub is_creator: bool
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Avatar {
    pub full_body_path: String
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Br {
    pub level: i32,
    pub division: i32
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserPin {
    url: String,
    anchor: String,
    is_default: bool
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SoloGame {
    pub token: String,
    #[serde(rename = "type")]
    pub type_entry: String,
    pub mode: String,
    pub state: String,
    pub round_count: i32,
    pub time_limit: i32,
    pub forbid_moving: bool,
    pub forbid_zooming: bool,
    pub forbid_rotating: bool,
    pub streak_type: String,
    pub map: String,
    pub map_name: String,
    pub panorama_provider: i32,
    pub bounds: Bound,
    pub round: i32,
    pub rounds: Vec<SoloRound>,
    pub player: SoloPlayer,
    pub progress_change: Option<ProgressChange>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SoloPlayer {
    pub total_score: TotalScore,
    pub total_distance: TotalDistance,
    pub total_distance_in_meters: f64,
    pub total_steps_count: i32,
    pub total_time: i32,
    pub total_streak: i32,
    pub guesses: Vec<SoloGuess>,
    pub is_leader: bool,
    pub current_position: i32,
    pub pin: PlayerPin,
    pub id: String,
    pub nick: String,
    pub is_verified: bool,
    pub flair: i32,
    pub country_code: String
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SoloGuess {
    pub lat: f64,
    pub lng: f64,
    pub timed_out: bool,
    pub timed_out_with_guess: bool,
    pub skipped_round: bool,
    pub round_score: RoundScore,
    pub round_score_in_percentage: f64,
    pub round_score_in_points: i32,
    pub distance: TotalDistance,
    pub distance_in_meters: f64,
    pub steps_count: i32,
    pub streak_location_code: Option<String>,
    pub time: i32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPin {
    pub url: String,
    pub anchor: String,
    pub is_default: bool
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RoundScore {
    pub amount: String,
    pub unit: String,
    pub percentage: f64
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TotalDistance {
    pub meters: Distance,
    pub miles: Distance
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Distance {
    pub amount: String,
    pub unit: String
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TotalScore {
    pub amount: String,
    pub unit: String,
    pub percentage: f64
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SoloRound {
    pub lat: f64,
    pub lng: f64,
    pub pano_id: String,
    pub heading: f64,
    pub pitch: f64,
    pub zoom: f64,
    pub streak_location_code: String,
    pub start_time: Option<String>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DuelsGame {
    pub game_id: String,
    pub teams: Vec<Team>,
    pub rounds: Vec<DuelsRound>,
    pub current_round_number: i32,
    pub status: String,
    pub version: i32,
    pub options: GameOption,
    pub movement_options: MovementOption,
    pub map_bounds: Bound,
    pub initial_health: i32,
    pub max_number_of_rounds: i32,
    pub result: Result,
    pub is_paused: bool
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameOption {
    pub initial_health: i32,
    pub individual_initial_health: bool,
    pub initial_health_team_one: i32,
    pub initial_health_team_two: i32,
    pub round_time: i32,
    pub max_round_time: i32,
    pub grace_period_time: i32,
    pub game_time_out: i32,
    pub max_number_of_rounds: i32,
    pub healing_rounds: Vec<i32>,
    pub movement_options: MovementOption,
    pub map_slug: String,
    pub is_rated: bool,
    pub map: Map,
    pub duel_round_options: Option<Vec<()>>,
    pub rounds_without_damage_multiplier: i32,
    pub disable_multipliers: bool,
    pub multiplier_increment: i32,
    pub disable_healing: bool,
    pub is_team_duels: bool,
    pub game_context: GameContext,
    pub round_starting_behavior: String,
    pub flashback_rounds: Vec<i32>,
    pub competitive_game_mode: String,
    pub count_all_guesses: bool,
    pub master_control_auto_start_rounds: bool,
    pub consumed_locations_identifier: String,
    pub use_curated_locations: bool,
    pub extra_wait_time_between_rounds: i32,
    pub round_countdown_delay: i32
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Map {
    pub name: String,
    pub slug: String,
    pub bounds: Bound,
    pub max_error_distance: i32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Result {
    pub is_draw: bool,
    pub winning_team_id: String,
    pub winner_style: String
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameContext {
    #[serde(rename = "type")]
    pub type_entry: String,
    pub id: String
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Bound {
    pub min: Pin,
    pub max: Pin
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MovementOption {
    pub forbid_moving: bool,
    pub forbid_zooming: bool,
    pub forbid_rotating: bool
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DuelsRound {
    pub round_number: i32,
    pub panorama: Panorama,
    pub has_processed_round_timeout: bool,
    pub is_healing_round: bool,
    pub multiplier: f64,
    pub damage_multiplier: f64,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub timer_start_time: Option<String>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Panorama {
    pub pano_id: String,
    pub lat: f64,
    pub lng: f64,
    pub country_code: String,
    pub heading: f64,
    pub pitch: f64,
    pub zoom: f64
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: String,
    pub name: String,
    pub health: i32,
    pub players: Vec<DuelsPlayer>,
    pub round_results: Vec<RoundResults>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RoundResults {
    pub round_number: i32,
    pub score: i32,
    pub health_before: i32,
    pub health_after: i32,
    pub best_guess: Option<DuelsGuess>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DuelsPlayer {
    pub player_id: String,
    pub guesses: Vec<DuelsGuess>,
    pub rating: Option<i32>,
    pub country_code: String,
    pub progress_change: Option<ProgressChange>,
    pub pin: Option<Pin>,
    pub help_requested: bool
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DuelsGuess {
    pub round_number: i32,
    pub lat: f64,
    pub lng: f64,
    pub distance: f64,
    pub created: String,
    pub is_teams_best_guess_on_round: bool,
    pub score: Option<i32>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProgressChange {
    // pub xp_progressions: Vec<>,
    // pub awarded_xp: Vec<>,
    pub medal: i32,
    // pub competitive_progress: Option<>,
    pub ranked_system_progress: Option<RankedSystemProgress>,
    pub ranked_team_duels_progress: Option<RankedTeamDuelsProgress>
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RankedSystemProgress {
    pub points: Points,
    pub total_weekly_points: i32,
    pub weekly_cap: i32,
    pub games_played_within_weekly_cap: i32,
    pub position_before: Option<i32>,
    pub position_after: Option<i32>,
    pub rating_before: Option<i32>,
    pub rating_after: Option<i32>,
    pub win_streak: i32,
    pub bucket_sorted_by: String,
    pub game_mode: String,
    pub game_mode_rating_before: Option<i32>,
    pub game_mode_rating_after: Option<i32>,
    pub game_mode_games_played: i32,
    pub game_mode_games_required: i32,
    pub placement_games_played: i32,
    pub placement_games_required: i32
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Pin {
    pub lat: f64,
    pub lng: f64
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RankedTeamDuelsProgress {
   pub rating_before: Option<i32>,
   pub rating_after: Option<i32>,
   pub points: Points,
   pub total_weekly_points: i32,
   pub weekly_cap: i32,
   pub games_played_within_weekly_cap: i32,
   pub position_before: Option<i32>,
   pub position_after: Option<i32>,
   pub win_streak: i32,
   pub bucket_sorted_by: String
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Points {
    pub win_within_weekly_cap: Option<i32>,
    pub first_win_of_the_day: Option<i32>,
    pub win_rounds: Option<i32>
}