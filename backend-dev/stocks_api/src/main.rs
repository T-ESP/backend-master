// src/main.rs
use anyhow::Result;
use tokio_postgres::NoTls;
use std::env;
use dotenvy::dotenv;          // ← ici  (au lieu de `dotenv::dotenv`)
                               // plus besoin de `use refinery::embed_migrations;` en haut :
                               // l’appel est déjà fait dans le module `embedded`

// Compile‑time import de tout le dossier `migrations/`
mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations");
}

#[tokio::main]
async fn main() -> Result<()> {
    // Charge .env
    dotenv().ok();
    let db_url = env::var("DATABASE_URL")
        .expect("⚠️  DATABASE_URL n'est pas défini");

    // Connexion Postgres
    let (mut client, connection) = tokio_postgres::connect(&db_url, NoTls).await?;

    // Garde la connexion vivante
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
    });

    // Exécute les migrations manquantes
    embedded::migrations::runner()
        .run_async(&mut client)
        .await?;

    println!("✅ Base de données à jour !");
    Ok(())
}
