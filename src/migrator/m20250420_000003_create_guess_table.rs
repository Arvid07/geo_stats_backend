use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000004_create_guess_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Guess::Table)
                    .col(
                        ColumnDef::new(Guess::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Guess::Lat).double().not_null())
                    .col(ColumnDef::new(Guess::Lng).double().not_null())
                    .col(ColumnDef::new(Guess::Score).integer().not_null())
                    .col(ColumnDef::new(Guess::Time).integer())
                    .col(ColumnDef::new(Guess::Distance).double().not_null())
                    .col(ColumnDef::new(Guess::RoundCountryCode).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Guess::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Guess {
    Table,
    Id,
    Lat,
    Lng,
    Score,
    Time,
    Distance,
    RoundCountryCode
}