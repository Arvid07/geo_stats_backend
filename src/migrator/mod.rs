mod m20250420_000001_create_duels_game_table;
mod m20250420_000002_create_duels_round_table;
mod m20250420_000003_create_guess_table;
mod m20250420_000004_create_location_table;
mod m20250420_000005_create_player_table;
mod m20250420_000006_create_solo_game_table;
mod m20250420_000007_create_solo_round_table;
mod m20250420_00008_create_comp_team_table;
mod m20250420_00009_create_fun_team_table;
mod m20250420_000010_create_map_table;
mod m20250504_000011_create_user_table;
mod m20250420_000012_create_session_table;

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250420_000001_create_duels_game_table::Migration),
            Box::new(m20250420_000002_create_duels_round_table::Migration),
            Box::new(m20250420_000003_create_guess_table::Migration),
            Box::new(m20250420_000004_create_location_table::Migration),
            Box::new(m20250420_000005_create_player_table::Migration),
            Box::new(m20250420_000006_create_solo_game_table::Migration),
            Box::new(m20250420_000007_create_solo_round_table::Migration),
            Box::new(m20250420_00008_create_comp_team_table::Migration),
            Box::new(m20250420_00009_create_fun_team_table::Migration),
            Box::new(m20250420_000010_create_map_table::Migration),
            Box::new(m20250504_000011_create_user_table::Migration),
            Box::new(m20250420_000012_create_session_table::Migration)
        ]
    }
}