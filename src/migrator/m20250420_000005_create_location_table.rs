use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000001_create_location_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Location::Table)
                    .col(
                        ColumnDef::new(Location::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Location::Lat).double().not_null())
                    .col(ColumnDef::new(Location::Lng).double().not_null())
                    .col(ColumnDef::new(Location::Heading).double())
                    .col(ColumnDef::new(Location::Pitch).string())
                    .col(ColumnDef::new(Location::Zoom).string())
                    .col(ColumnDef::new(Location::CountryCode).string())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Location::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Location {
    Table,
    Id,
    Lat,
    Lng,
    Heading,
    Pitch,
    Zoom,
    CountryCode
}