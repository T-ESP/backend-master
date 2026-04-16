use sqlx::PgPool;
use uuid::Uuid;

use super::model::Tenant;

pub async fn find_all(pool: &PgPool) -> Result<Vec<Tenant>, sqlx::Error> {
    let tenants = sqlx::query_as::<_, Tenant>(
        "SELECT * FROM commerces ORDER BY created_at DESC"
    )
        .fetch_all(pool)
        .await?;

    Ok(tenants)
}

pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Tenant>, sqlx::Error> {
    let tenant = sqlx::query_as::<_, Tenant>(
        "SELECT * FROM commerces WHERE id = $1"
    )
        .bind(id)
        .fetch_optional(pool)
        .await?;

    Ok(tenant)
}

pub async fn find_by_slug(pool: &PgPool, slug: &str) -> Result<Option<Tenant>, sqlx::Error> {
    let tenant = sqlx::query_as::<_, Tenant>(
        "SELECT * FROM commerces WHERE slug = $1"
    )
        .bind(slug)
        .fetch_optional(pool)
        .await?;

    Ok(tenant)
}

pub async fn update(
    pool: &PgPool,
    id: Uuid,
    name: Option<String>,
    email: Option<String>,
    phone: Option<String>,
    address: Option<String>,
    siret: Option<String>,
    status: Option<String>,
) -> Result<Option<Tenant>, sqlx::Error> {
    let tenant = sqlx::query_as::<_, Tenant>(
        r#"
        UPDATE commerces SET
            name    = COALESCE($2, name),
            email   = COALESCE($3, email),
            phone   = COALESCE($4, phone),
            address = COALESCE($5, address),
            siret   = COALESCE($6, siret),
            status  = COALESCE($7, status)
        WHERE id = $1
        RETURNING *
        "#
    )
        .bind(id)
        .bind(name)
        .bind(email)
        .bind(phone)
        .bind(address)
        .bind(siret)
        .bind(status)
        .fetch_optional(pool)
        .await?;

    Ok(tenant)
}

pub async fn delete(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM commerces WHERE id = $1"
    )
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}
