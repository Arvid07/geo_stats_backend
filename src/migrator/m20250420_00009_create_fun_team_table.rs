use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000011_create_fun_team_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FunTeam::Table)
                    .col(
                        ColumnDef::new(FunTeam::TeamId)
                            .string()
                            .not_null()
                            .primary_key()
                    )
                    .col(ColumnDef::new(FunTeam::PlayerIds).array(ColumnType::String(StringLen::None)).not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FunTeam::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum FunTeam {
    Table,
    TeamId,
    PlayerIds
}