use rust_decimal::Decimal;
use sqlx::{PgPool, Row};

use super::dto::{
    ApplicableDiscount, AppliedDiscountSummary, CheckDiscountRequest, CheckDiscountResponse,
    CheckLineItem, CreateDiscountRequest, DiscountAction, DiscountOrderSummary, DiscountResponse,
    DiscountScope, DiscountTrigger, OrderDiscountSummary, UpdateDiscountRequest,
};

pub struct DiscountService;

impl DiscountService {
    pub async fn get_all(pool: &PgPool) -> Result<Vec<DiscountResponse>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT d.id_dis, d.name_dis, d.trigger_type_dis, d.trigger_product_id_dis,
                    d.trigger_min_amount_dis, d.trigger_min_qty_dis,
                    d.action_type_dis, d.action_value_dis, d.scope_dis, d.scope_product_id_dis,
                    d.cumulative_dis, d.valid_from_dis, d.valid_until_dis,
                    d.is_active_dis, d.created_at, d.updated_at,
                    COUNT(odc.order_id_odc) AS usage_count
             FROM discounts_dis d
             LEFT JOIN order_discounts_odc odc ON odc.discount_id_odc = d.id_dis
             GROUP BY d.id_dis
             ORDER BY d.created_at DESC",
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.iter().map(DiscountResponse::from_row).collect())
    }

    pub async fn get_by_id(
        pool: &PgPool,
        id: i32,
    ) -> Result<Option<DiscountResponse>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT d.id_dis, d.name_dis, d.trigger_type_dis, d.trigger_product_id_dis,
                    d.trigger_min_amount_dis, d.trigger_min_qty_dis,
                    d.action_type_dis, d.action_value_dis, d.scope_dis, d.scope_product_id_dis,
                    d.cumulative_dis, d.valid_from_dis, d.valid_until_dis,
                    d.is_active_dis, d.created_at, d.updated_at,
                    COUNT(odc.order_id_odc) AS usage_count
             FROM discounts_dis d
             LEFT JOIN order_discounts_odc odc ON odc.discount_id_odc = d.id_dis
             WHERE d.id_dis = $1
             GROUP BY d.id_dis",
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| DiscountResponse::from_row(&r)))
    }

    pub async fn get_usage_orders(
        pool: &PgPool,
        id: i32,
    ) -> Result<Vec<DiscountOrderSummary>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT o.id_ord, o.order_date_ord, odc.saving_amount_odc, o.amount_ord
             FROM order_discounts_odc odc
             JOIN order_ord o ON o.id_ord = odc.order_id_odc
             WHERE odc.discount_id_odc = $1
             ORDER BY o.order_date_ord DESC",
        )
        .bind(id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| DiscountOrderSummary {
                order_id: r.get("id_ord"),
                order_date: r.get("order_date_ord"),
                saving_amount: r.get("saving_amount_odc"),
                order_total: r.get("amount_ord"),
            })
            .collect())
    }

    pub async fn get_order_discounts(
        pool: &PgPool,
        order_id: i32,
    ) -> Result<Vec<OrderDiscountSummary>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT d.id_dis, d.name_dis, d.action_type_dis, d.action_value_dis,
                    odc.saving_amount_odc
             FROM order_discounts_odc odc
             JOIN discounts_dis d ON d.id_dis = odc.discount_id_odc
             WHERE odc.order_id_odc = $1",
        )
        .bind(order_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| OrderDiscountSummary {
                discount_id: r.get("id_dis"),
                discount_name: r.get("name_dis"),
                action_type: r.get("action_type_dis"),
                action_value: r.get("action_value_dis"),
                saving_amount: r.get("saving_amount_odc"),
            })
            .collect())
    }

    pub async fn create(
        pool: &PgPool,
        req: &CreateDiscountRequest,
    ) -> Result<DiscountResponse, sqlx::Error> {
        let row = sqlx::query(
            "INSERT INTO discounts_dis (
                name_dis, trigger_type_dis, trigger_product_id_dis,
                trigger_min_amount_dis, trigger_min_qty_dis,
                action_type_dis, action_value_dis, scope_dis, scope_product_id_dis,
                cumulative_dis, valid_from_dis, valid_until_dis, is_active_dis
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
             RETURNING id_dis, name_dis, trigger_type_dis, trigger_product_id_dis,
                       trigger_min_amount_dis, trigger_min_qty_dis,
                       action_type_dis, action_value_dis, scope_dis, scope_product_id_dis,
                       cumulative_dis, valid_from_dis, valid_until_dis,
                       is_active_dis, created_at, updated_at",
        )
        .bind(&req.name)
        .bind(req.trigger_type)
        .bind(req.trigger_product_id)
        .bind(req.trigger_min_amount)
        .bind(req.trigger_min_qty)
        .bind(req.action_type)
        .bind(req.action_value)
        .bind(req.scope)
        .bind(req.scope_product_id)
        .bind(req.cumulative.unwrap_or(false))
        .bind(req.valid_from)
        .bind(req.valid_until)
        .bind(req.is_active.unwrap_or(true))
        .fetch_one(pool)
        .await?;

        Ok(DiscountResponse::from_row(&row))
    }

    pub async fn update(
        pool: &PgPool,
        id: i32,
        req: &UpdateDiscountRequest,
    ) -> Result<Option<DiscountResponse>, sqlx::Error> {
        let row = sqlx::query(
            "UPDATE discounts_dis SET
                name_dis         = COALESCE($1, name_dis),
                action_value_dis = COALESCE($2, action_value_dis),
                cumulative_dis   = COALESCE($3, cumulative_dis),
                valid_from_dis   = COALESCE($4, valid_from_dis),
                valid_until_dis  = COALESCE($5, valid_until_dis),
                is_active_dis    = COALESCE($6, is_active_dis),
                updated_at       = NOW()
             WHERE id_dis = $7
             RETURNING id_dis, name_dis, trigger_type_dis, trigger_product_id_dis,
                       trigger_min_amount_dis, trigger_min_qty_dis,
                       action_type_dis, action_value_dis, scope_dis, scope_product_id_dis,
                       cumulative_dis, valid_from_dis, valid_until_dis,
                       is_active_dis, created_at, updated_at",
        )
        .bind(&req.name)
        .bind(req.action_value)
        .bind(req.cumulative)
        .bind(req.valid_from)
        .bind(req.valid_until)
        .bind(req.is_active)
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|r| DiscountResponse::from_row(&r)))
    }

    pub async fn delete(pool: &PgPool, id: i32) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM discounts_dis WHERE id_dis = $1")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    // ── Core evaluation logic ─────────────────────────────────────────────────

    fn compute_saving(
        action_type: DiscountAction,
        action_value: Decimal,
        scope: DiscountScope,
        scope_product_id: Option<i32>,
        line_items: &[CheckLineItem],
        total_amount: Decimal,
    ) -> Decimal {
        let hundred = Decimal::new(100, 0);
        match (action_type, scope) {
            (DiscountAction::FixedEur, _) => action_value,
            (DiscountAction::Percentage, DiscountScope::Global) => {
                (total_amount * action_value / hundred).round_dp(2)
            }
            (DiscountAction::Percentage, DiscountScope::PerProduct) => {
                let base = if let Some(pid) = scope_product_id {
                    line_items
                        .iter()
                        .filter(|i| i.product_id == pid)
                        .map(|i| i.line_total)
                        .fold(Decimal::ZERO, |a, b| a + b)
                } else {
                    total_amount
                };
                (base * action_value / hundred).round_dp(2)
            }
        }
    }

    fn is_triggered(
        trigger_type: DiscountTrigger,
        trigger_product_id: Option<i32>,
        trigger_min_amount: Option<Decimal>,
        trigger_min_qty: Option<i32>,
        line_items: &[CheckLineItem],
        total_amount: Decimal,
    ) -> bool {
        match trigger_type {
            DiscountTrigger::Product => trigger_product_id
                .map(|pid| line_items.iter().any(|i| i.product_id == pid))
                .unwrap_or(false),
            DiscountTrigger::TotalAmount => trigger_min_amount
                .map(|min| total_amount >= min)
                .unwrap_or(false),
            DiscountTrigger::Quantity => {
                let min_qty = trigger_min_qty.unwrap_or(0);
                if let Some(pid) = trigger_product_id {
                    line_items
                        .iter()
                        .filter(|i| i.product_id == pid)
                        .map(|i| i.quantity)
                        .sum::<i32>()
                        >= min_qty
                } else {
                    line_items.iter().map(|i| i.quantity).sum::<i32>() >= min_qty
                }
            }
        }
    }

    // Apply cumul rule: all cumulatives + best non-cumulative
    fn select_discounts(applicable: &[ApplicableDiscount]) -> Vec<usize> {
        let mut selected: Vec<usize> = applicable
            .iter()
            .enumerate()
            .filter(|(_, d)| d.cumulative)
            .map(|(i, _)| i)
            .collect();

        if let Some((best_idx, _)) = applicable
            .iter()
            .enumerate()
            .filter(|(_, d)| !d.cumulative)
            .max_by(|(_, a), (_, b)| a.saving_amount.cmp(&b.saving_amount))
        {
            selected.push(best_idx);
        }
        selected
    }

    pub async fn check_discounts(
        pool: &PgPool,
        req: &CheckDiscountRequest,
    ) -> Result<CheckDiscountResponse, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id_dis, name_dis, trigger_type_dis, trigger_product_id_dis,
                    trigger_min_amount_dis, trigger_min_qty_dis,
                    action_type_dis, action_value_dis, scope_dis, scope_product_id_dis,
                    cumulative_dis
             FROM discounts_dis
             WHERE is_active_dis = TRUE
               AND (valid_from_dis  IS NULL OR valid_from_dis  <= NOW())
               AND (valid_until_dis IS NULL OR valid_until_dis >= NOW())",
        )
        .fetch_all(pool)
        .await?;

        let total_before = req.total_amount;
        let mut applicable: Vec<ApplicableDiscount> = Vec::new();

        for row in &rows {
            let trigger_type: DiscountTrigger = row.get("trigger_type_dis");
            let trigger_product_id: Option<i32> = row.get("trigger_product_id_dis");
            let trigger_min_amount: Option<Decimal> = row.get("trigger_min_amount_dis");
            let trigger_min_qty: Option<i32> = row.get("trigger_min_qty_dis");
            let action_type: DiscountAction = row.get("action_type_dis");
            let action_value: Decimal = row.get("action_value_dis");
            let scope: DiscountScope = row.get("scope_dis");
            let scope_product_id: Option<i32> = row.get("scope_product_id_dis");
            let cumulative: bool = row.get("cumulative_dis");

            if !Self::is_triggered(
                trigger_type,
                trigger_product_id,
                trigger_min_amount,
                trigger_min_qty,
                &req.line_items,
                total_before,
            ) {
                continue;
            }

            let saving = Self::compute_saving(
                action_type,
                action_value,
                scope,
                scope_product_id,
                &req.line_items,
                total_before,
            )
            .min(total_before);

            applicable.push(ApplicableDiscount {
                id: row.get("id_dis"),
                name: row.get("name_dis"),
                trigger_type,
                action_type,
                action_value,
                scope,
                scope_product_id,
                saving_amount: saving,
                cumulative,
            });
        }

        let selected_indices = Self::select_discounts(&applicable);
        let total_saving = selected_indices
            .iter()
            .map(|&i| applicable[i].saving_amount)
            .fold(Decimal::ZERO, |a, b| a + b)
            .min(total_before);

        Ok(CheckDiscountResponse {
            applicable_discounts: applicable,
            total_before,
            total_after: total_before - total_saving,
            total_saving,
        })
    }

    /// Validates and computes savings for a specific list of discount IDs at order creation time.
    /// Returns (total_saving, per_discount_summaries).
    pub async fn compute_order_savings(
        pool: &PgPool,
        discount_ids: &[i32],
        line_items: &[CheckLineItem],
        total_amount: Decimal,
    ) -> Result<(Decimal, Vec<AppliedDiscountSummary>), sqlx::Error> {
        if discount_ids.is_empty() {
            return Ok((Decimal::ZERO, Vec::new()));
        }

        let rows = sqlx::query(
            "SELECT id_dis, name_dis, trigger_type_dis, trigger_product_id_dis,
                    trigger_min_amount_dis, trigger_min_qty_dis,
                    action_type_dis, action_value_dis, scope_dis, scope_product_id_dis
             FROM discounts_dis
             WHERE id_dis = ANY($1)
               AND is_active_dis = TRUE
               AND (valid_from_dis  IS NULL OR valid_from_dis  <= NOW())
               AND (valid_until_dis IS NULL OR valid_until_dis >= NOW())",
        )
        .bind(discount_ids)
        .fetch_all(pool)
        .await?;

        let mut summaries: Vec<AppliedDiscountSummary> = Vec::new();

        for row in &rows {
            let trigger_type: DiscountTrigger = row.get("trigger_type_dis");
            let trigger_product_id: Option<i32> = row.get("trigger_product_id_dis");
            let trigger_min_amount: Option<Decimal> = row.get("trigger_min_amount_dis");
            let trigger_min_qty: Option<i32> = row.get("trigger_min_qty_dis");
            let action_type: DiscountAction = row.get("action_type_dis");
            let action_value: Decimal = row.get("action_value_dis");
            let scope: DiscountScope = row.get("scope_dis");
            let scope_product_id: Option<i32> = row.get("scope_product_id_dis");

            if !Self::is_triggered(
                trigger_type,
                trigger_product_id,
                trigger_min_amount,
                trigger_min_qty,
                line_items,
                total_amount,
            ) {
                continue;
            }

            let saving = Self::compute_saving(
                action_type,
                action_value,
                scope,
                scope_product_id,
                line_items,
                total_amount,
            )
            .min(total_amount);

            summaries.push(AppliedDiscountSummary {
                id: row.get("id_dis"),
                name: row.get("name_dis"),
                action_type,
                action_value,
                saving_amount: saving,
            });
        }

        let total_saving = summaries
            .iter()
            .map(|s| s.saving_amount)
            .fold(Decimal::ZERO, |a, b| a + b)
            .min(total_amount);

        Ok((total_saving, summaries))
    }
}
