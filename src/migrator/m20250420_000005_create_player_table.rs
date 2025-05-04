use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20250420_000006_create_player_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Player::Table)
                    .col(
                        ColumnDef::new(Player::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Player::Name).string().not_null())
                    .col(ColumnDef::new(Player::CountryCode).string().not_null())
                    .col(ColumnDef::new(Player::AvatarPin).string().not_null())
                    .col(ColumnDef::new(Player::Level).integer().not_null())
                    .col(ColumnDef::new(Player::IsProUser).boolean().not_null())
                    .col(ColumnDef::new(Player::IsCreator).boolean().not_null())
                    .col(ColumnDef::new(Player::Rating).integer())
                    .col(ColumnDef::new(Player::MovingRating).integer())
                    .col(ColumnDef::new(Player::NoMoveRating).integer())
                    .col(ColumnDef::new(Player::NMPZRating).integer())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Player::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Player {
    Table,
    Id,
    Name,
    CountryCode,
    AvatarPin,
    Level,
    IsProUser,
    IsCreator,
    Rating,
    MovingRating,
    NoMoveRating,
    NMPZRating
}