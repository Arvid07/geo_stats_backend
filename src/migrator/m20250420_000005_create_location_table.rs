use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000005_create_location_table"
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
                    .col(ColumnDef::new(Location::Heading).double().not_null())
                    .col(ColumnDef::new(Location::Pitch).double().not_null())
                    .col(ColumnDef::new(Location::Zoom).double().not_null())
                    .col(ColumnDef::new(Location::CountryCode).string().not_null())
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