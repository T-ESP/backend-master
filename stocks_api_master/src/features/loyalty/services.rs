use sqlx::{PgPool, Row};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use super::dto::{LoyaltyConfigResponse, LoyaltyTransactionResponse, UserLoyaltyResponse};

pub struct LoyaltyService;

impl LoyaltyService {
    pub async fn get_or_create_config(pool: &PgPool) -> Result<LoyaltyConfigResponse, sqlx::Error> {
        let existing = sqlx::query(
            "SELECT id_lco, euros_per_point, created_at, updated_at FROM loyalty_config_lco LIMIT 1"
        )
        .fetch_optional(pool)
        .await?;

        if let Some(row) = existing {
            return Ok(LoyaltyConfigResponse {
                id: row.get("id_lco"),
                euros_per_point: row.get("euros_per_point"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        let row = sqlx::query(
            "INSERT INTO loyalty_config_lco (euros_per_point) VALUES (2.00)
             RETURNING id_lco, euros_per_point, created_at, updated_at"
        )
        .fetch_one(pool)
        .await?;

        Ok(LoyaltyConfigResponse {
            id: row.get("id_lco"),
            euros_per_point: row.get("euros_per_point"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn update_config(pool: &PgPool, euros_per_point: Decimal) -> Result<LoyaltyConfigResponse, sqlx::Error> {
        Self::get_or_create_config(pool).await?;

        let row = sqlx::query(
            "UPDATE loyalty_config_lco SET euros_per_point = $1, updated_at = NOW()
             RETURNING id_lco, euros_per_point, created_at, updated_at"
        )
        .bind(euros_per_point)
        .fetch_one(pool)
        .await?;

        Ok(LoyaltyConfigResponse {
            id: row.get("id_lco"),
            euros_per_point: row.get("euros_per_point"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    pub async fn award_points(
        pool: &PgPool,
        user_id: i32,
        order_id: i32,
        order_amount: Decimal,
    ) -> Result<i32, sqlx::Error> {
        let config = Self::get_or_create_config(pool).await?;

        if config.euros_per_point <= Decimal::ZERO {
            return Ok(0);
        }

        let points = (order_amount / config.euros_per_point)
            .floor()
            .to_i32()
            .unwrap_or(0);

        if points <= 0 {
            return Ok(0);
        }

        sqlx::query(
            "INSERT INTO loyalty_points_lpo (user_id_lpo, order_id_lpo, points_lpo) VALUES ($1, $2, $3)"
        )
        .bind(user_id)
        .bind(order_id)
        .bind(points)
        .execute(pool)
        .await?;

        Ok(points)
    }

    pub async fn get_user_loyalty(pool: &PgPool, user_id: i32) -> Result<UserLoyaltyResponse, sqlx::Error> {
        let total_row = sqlx::query(
            "SELECT COALESCE(SUM(points_lpo), 0)::BIGINT as total FROM loyalty_points_lpo WHERE user_id_lpo = $1"
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        let total_points: i64 = total_row.get("total");

        let rows = sqlx::query(
            "SELECT id_lpo, order_id_lpo, points_lpo, created_at
             FROM loyalty_points_lpo
             WHERE user_id_lpo = $1
             ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        let transactions = rows.iter().map(|row| LoyaltyTransactionResponse {
            id: row.get("id_lpo"),
            order_id: row.get("order_id_lpo"),
            points: row.get("points_lpo"),
            created_at: row.get("created_at"),
        }).collect();

        Ok(UserLoyaltyResponse {
            user_id,
            total_points,
            transactions,
        })
    }
}
