use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000010_create_map_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Map::Table)
                    .col(
                        ColumnDef::new(Map::Id)
                            .string()
                            .not_null()
                            .primary_key()
                    )
                    .col(ColumnDef::new(Map::Name).string().not_null())
                    .col(ColumnDef::new(Map::Lat1).double().not_null())
                    .col(ColumnDef::new(Map::Lng1).double().not_null())
                    .col(ColumnDef::new(Map::Lat2).double().not_null())
                    .col(ColumnDef::new(Map::Lng2).double().not_null())
                    .col(ColumnDef::new(Map::MaxDistance).integer().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Map::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Map {
    Table,
    Id,
    Name,
    Lat1,
    Lng1,
    Lat2,
    Lng2,
    MaxDistance
}