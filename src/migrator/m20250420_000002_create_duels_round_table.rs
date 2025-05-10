use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000002_create_duels_round_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(DuelsRound::Table)
                    .col(
                        ColumnDef::new(DuelsRound::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DuelsRound::GameId).string().not_null())
                    .col(ColumnDef::new(DuelsRound::LocationId).string().not_null())
                    .col(ColumnDef::new(DuelsRound::RoundNumber).integer().not_null())
                    .col(ColumnDef::new(DuelsRound::DamageMultiplier).double().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(DuelsRound::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum DuelsRound {
    Table,
    Id,
    GameId,
    LocationId,
    RoundNumber,
    DamageMultiplier
}