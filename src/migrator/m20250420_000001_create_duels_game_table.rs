use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000001_create_duels_game_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DuelsGame::Table)
                    .col(
                        ColumnDef::new(DuelsGame::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DuelsGame::TeamId1).string().not_null())
                    .col(ColumnDef::new(DuelsGame::TeamId2).string().not_null())
                    .col(ColumnDef::new(DuelsGame::HealthTeam1).integer().not_null())
                    .col(ColumnDef::new(DuelsGame::HealthTeam2).integer().not_null())
                    .col(ColumnDef::new(DuelsGame::TeamGameMode).string().not_null())
                    .col(ColumnDef::new(DuelsGame::GeoMode).string().not_null())
                    .col(ColumnDef::new(DuelsGame::StartTime).string().not_null())
                    .col(ColumnDef::new(DuelsGame::MapId).string().not_null())
                    .col(ColumnDef::new(DuelsGame::RatingBeforeTeam1).integer())
                    .col(ColumnDef::new(DuelsGame::RatingBeforeTeam2).integer())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DuelsGame::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum DuelsGame {
    Table,
    Id,
    TeamId1,
    TeamId2,
    HealthTeam1,
    HealthTeam2,
    TeamGameMode,
    GeoMode,
    StartTime,
    MapId,
    RatingBeforeTeam1,
    RatingBeforeTeam2
}