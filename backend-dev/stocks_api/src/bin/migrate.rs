use anyhow::Result;
use dotenvy::dotenv;
use refinery::embed_migrations;
use std::env;
use tokio_postgres::NoTls;

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations");
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL")?;

    let (mut client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;
    tokio::spawn(async move { connection.await.unwrap() });

    embedded::migrations::runner().run_async(&mut client).await?;
    println!("✅ Migrations terminées");
    Ok(())
}