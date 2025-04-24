use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000011_create_comp_team_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(CompTeam::Table)
                    .col(
                        ColumnDef::new(CompTeam::TeamId)
                            .string()
                            .not_null()
                            .primary_key()
                    )
                    .col(ColumnDef::new(CompTeam::PlayerId1).string().not_null())
                    .col(ColumnDef::new(CompTeam::PlayerId2).string().not_null())
                    .col(ColumnDef::new(CompTeam::Name).string().not_null())
                    .col(ColumnDef::new(CompTeam::Rating).integer())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(CompTeam::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum CompTeam {
    Table,
    TeamId,
    PlayerId1,
    PlayerId2,
    Name,
    Rating
}