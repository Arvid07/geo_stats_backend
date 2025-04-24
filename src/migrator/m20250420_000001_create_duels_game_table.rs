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
                    .col(ColumnDef::new(DuelsGame::PlayerId1).string().not_null())
                    .col(ColumnDef::new(DuelsGame::PlayerId2).string().not_null())
                    .col(ColumnDef::new(DuelsGame::GameMode).string().not_null())
                    .col(ColumnDef::new(DuelsGame::StartTime).string().not_null())
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
    PlayerId1,
    PlayerId2,
    GameMode,
    StartTime
}