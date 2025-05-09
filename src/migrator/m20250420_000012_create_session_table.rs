use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000012_create_session_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Session::Table)
                    .col(
                        ColumnDef::new(Session::Id)
                            .string()
                            .not_null()
                            .primary_key()
                    )
                    .col(ColumnDef::new(Session::UserId).string().not_null())
                    .col(ColumnDef::new(Session::ExpireDate).string().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Session::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Session {
    Table,
    Id,
    UserId,
    ExpireDate
}