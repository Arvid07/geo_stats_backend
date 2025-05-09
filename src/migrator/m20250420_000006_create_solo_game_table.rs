use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000006_create_solo_game_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SoloGame::Table)
                    .col(
                        ColumnDef::new(SoloGame::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SoloGame::PlayerId).string().not_null())
                    .col(ColumnDef::new(SoloGame::GeoMode).string().not_null())
                    .col(ColumnDef::new(SoloGame::StartTime).string().not_null())
                    .col(ColumnDef::new(SoloGame::MapId).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SoloGame::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum SoloGame {
    Table,
    Id,
    PlayerId,
    GeoMode,
    StartTime,
    MapId
}