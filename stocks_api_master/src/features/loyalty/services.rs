use sqlx::{PgPool, Row};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use super::dto::{
    LoyaltyConfigResponse, LoyaltyTransactionResponse,
    UserLoyaltyResponse, UserDiscountResponse,
    AdjustPointsResponse,
};

#[derive(Debug)]
pub enum AdjustPointsError {
    ZeroAdjustment,
    InsufficientBalance { current: i64, requested: i32 },
    Db(sqlx::Error),
}

impl From<sqlx::Error> for AdjustPointsError {
    fn from(e: sqlx::Error) -> Self { AdjustPointsError::Db(e) }
}

pub struct LoyaltyService;

fn row_to_config(row: &sqlx::postgres::PgRow) -> LoyaltyConfigResponse {
    LoyaltyConfigResponse {
        id: row.get("id_lco"),
        euros_per_point: row.get("euros_per_point"),
        points_required: row.get("points_required"),
        discount_percent: row.get("discount_percent"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

impl LoyaltyService {
    pub async fn get_or_create_config(pool: &PgPool) -> Result<LoyaltyConfigResponse, sqlx::Error> {
        let existing = sqlx::query(
            "SELECT id_lco, euros_per_point, points_required, discount_percent, created_at, updated_at
             FROM loyalty_config_lco LIMIT 1"
        )
        .fetch_optional(pool)
        .await?;

        if let Some(row) = existing {
            return Ok(row_to_config(&row));
        }

        let row = sqlx::query(
            "INSERT INTO loyalty_config_lco (euros_per_point, points_required, discount_percent)
             VALUES (2.00, 100, 5.00)
             RETURNING id_lco, euros_per_point, points_required, discount_percent, created_at, updated_at"
        )
        .fetch_one(pool)
        .await?;

        Ok(row_to_config(&row))
    }

    pub async fn update_config(
        pool: &PgPool,
        euros_per_point: Option<Decimal>,
        points_required: Option<i32>,
        discount_percent: Option<Decimal>,
    ) -> Result<LoyaltyConfigResponse, sqlx::Error> {
        Self::get_or_create_config(pool).await?;

        let row = sqlx::query(
            "UPDATE loyalty_config_lco
             SET euros_per_point  = COALESCE($1, euros_per_point),
                 points_required  = COALESCE($2, points_required),
                 discount_percent = COALESCE($3, discount_percent),
                 updated_at = NOW()
             RETURNING id_lco, euros_per_point, points_required, discount_percent, created_at, updated_at"
        )
        .bind(euros_per_point)
        .bind(points_required)
        .bind(discount_percent)
        .fetch_one(pool)
        .await?;

        Ok(row_to_config(&row))
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
            "SELECT id_lpo, order_id_lpo, points_lpo, reason_lpo, created_at
             FROM loyalty_points_lpo
             WHERE user_id_lpo = $1
             ORDER BY created_at DESC"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        let transactions = rows.iter().map(|row| LoyaltyTransactionResponse {
            id: row.get("id_lpo"),
            order_id: row.try_get("order_id_lpo").ok(),
            points: row.get("points_lpo"),
            reason: row.try_get("reason_lpo").ok(),
            created_at: row.get("created_at"),
        }).collect();

        Ok(UserLoyaltyResponse {
            user_id,
            total_points,
            transactions,
        })
    }

    pub async fn adjust_points(
        pool: &PgPool,
        user_id: i32,
        points: i32,
        reason: Option<String>,
    ) -> Result<AdjustPointsResponse, AdjustPointsError> {
        if points == 0 {
            return Err(AdjustPointsError::ZeroAdjustment);
        }

        let mut tx = pool.begin().await?;

        let total_row = sqlx::query(
            "SELECT COALESCE(SUM(points_lpo), 0)::BIGINT as total
             FROM loyalty_points_lpo WHERE user_id_lpo = $1"
        )
        .bind(user_id)
        .fetch_one(&mut *tx)
        .await?;

        let current_total: i64 = total_row.get("total");

        if points < 0 && current_total + (points as i64) < 0 {
            return Err(AdjustPointsError::InsufficientBalance {
                current: current_total,
                requested: points,
            });
        }

        let inserted = sqlx::query(
            "INSERT INTO loyalty_points_lpo (user_id_lpo, order_id_lpo, points_lpo, reason_lpo)
             VALUES ($1, NULL, $2, $3)
             RETURNING id_lpo, order_id_lpo, points_lpo, reason_lpo, created_at"
        )
        .bind(user_id)
        .bind(points)
        .bind(&reason)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        let transaction = LoyaltyTransactionResponse {
            id: inserted.get("id_lpo"),
            order_id: inserted.try_get("order_id_lpo").ok(),
            points: inserted.get("points_lpo"),
            reason: inserted.try_get("reason_lpo").ok(),
            created_at: inserted.get("created_at"),
        };

        Ok(AdjustPointsResponse {
            user_id,
            adjustment: points,
            reason,
            new_total_points: current_total + (points as i64),
            transaction,
        })
    }

    pub async fn get_user_discount(pool: &PgPool, user_id: i32) -> Result<UserDiscountResponse, sqlx::Error> {
        let config = Self::get_or_create_config(pool).await?;

        let total_row = sqlx::query(
            "SELECT COALESCE(SUM(points_lpo), 0)::BIGINT as total FROM loyalty_points_lpo WHERE user_id_lpo = $1"
        )
        .bind(user_id)
        .fetch_one(pool)
        .await?;

        let total_points: i64 = total_row.get("total");

        let tiers = total_points / (config.points_required as i64);
        let raw_discount = Decimal::from(tiers) * config.discount_percent;
        let hundred = Decimal::from(100);
        let available_discount = if raw_discount > hundred { hundred } else { raw_discount };

        let points_used = tiers * (config.points_required as i64);

        Ok(UserDiscountResponse {
            user_id,
            total_points,
            points_required: config.points_required,
            discount_percent: config.discount_percent,
            available_discount_percent: available_discount,
            points_used,
        })
    }
}
