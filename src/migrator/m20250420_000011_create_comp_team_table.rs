use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000011_create_team_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Team::Table)
                    .col(
                        ColumnDef::new(Team::PlayerId1)
                            .string()
                            .not_null()
                    ).col(
                        ColumnDef::new(Team::PlayerId2)
                            .string()
                            .not_null()
                    )
                    .col(ColumnDef::new(Team::Name).string().not_null())
                    .col(ColumnDef::new(Team::NameHistory).array(ColumnType::String(StringLen::None)))
                    .primary_key(
                        Index::create()
                            .col(Team::PlayerId1)
                            .col(Team::PlayerId2)
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Team::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Team {
    Table,
    PlayerId1,
    PlayerId2,
    Name,
    NameHistory
}