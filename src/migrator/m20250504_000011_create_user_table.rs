use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000011_create_user_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .col(
                        ColumnDef::new(User::Id)
                            .string()
                            .not_null()
                            .primary_key()
                    )
                    .col(ColumnDef::new(User::Email).string().not_null())
                    .col(ColumnDef::new(User::Salt).string().not_null())
                    .col(ColumnDef::new(User::SaltedPasswordHash).string().not_null())
                    .col(ColumnDef::new(User::PlayerId).string())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum User {
    Table,
    Id,
    Email,
    Salt,
    SaltedPasswordHash,
    PlayerId
}