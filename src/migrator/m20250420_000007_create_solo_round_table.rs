use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000007_create_solo_round_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SoloRound::Table)
                    .col(
                        ColumnDef::new(SoloRound::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SoloRound::GameId).string().not_null())
                    .col(ColumnDef::new(SoloRound::GuessId).string().not_null())
                    .col(ColumnDef::new(SoloRound::LocationId).string().not_null())
                    .col(ColumnDef::new(SoloRound::RoundNumber).integer().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SoloRound::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum SoloRound {
    Table,
    Id,
    GameId,
    GuessId,
    LocationId,
    RoundNumber
}