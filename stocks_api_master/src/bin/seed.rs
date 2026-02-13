use anyhow::Result;
use dotenvy::dotenv;
use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand::rngs::StdRng;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::{env, str::FromStr, time::Duration};
use tokio::time::sleep;
use tokio_postgres::NoTls;
use std::collections::HashMap;
use stocks_api::common::security;

// ═══════════════════════════════════════════════════════════════════════════
// CONFIGURATION GLOBALE DU SEEDER
// ═══════════════════════════════════════════════════════════════════════════

// ────────────── Connexion ──────────────
/// Nombre maximum de tentatives de connexion à la base de données avant d'abandonner
const MAX_CONNECTION_ATTEMPTS: usize = 10;

/// Délai d'attente en secondes entre chaque tentative de reconnexion
const RETRY_DELAY_SECONDS: u64 = 2;

// ────────────── Utilisateurs ──────────────
/// Nombre total d'utilisateurs à créer dans la base de données (en plus de admin et manager)
const NB_USERS: usize = 850;

// ────────────── Fournisseurs ──────────────
/// Nombre total de fournisseurs (suppliers) à créer dans la base de données
const NB_SUPPLIERS: usize = 20;

// ────────────── Produits ──────────────
/// Nombre total de produits à créer dans le catalogue
const NB_PRODUCTS: usize = 200;

/// Quantité minimale en stock pour un produit (0 permet de tester les ruptures de stock)
const STOCK_MIN: i32 = 0;

/// Quantité maximale en stock pour un produit
const STOCK_MAX: i32 = 200;

/// Prix d'achat minimum en centimes (100 centimes = 1.00€)
const BUYING_PRICE_MIN_CENTS: i64 = 100;

/// Prix d'achat maximum en centimes (2000 centimes = 20.00€)
const BUYING_PRICE_MAX_CENTS: i64 = 2000;

/// Coefficient multiplicateur pour calculer le prix de vente à partir du prix d'achat (1.20 = marge de 20%)
const SELLING_MARGIN_MULTIPLIER: &str = "1.20";

// ────────────── Commandes ──────────────
/// Nombre total de commandes à générer dans la base de données
const NB_ORDERS: usize = 5000;

/// Nombre minimum de lignes (produits différents) par commande
const MIN_LINES_PER_ORDER: usize = 2;

/// Nombre maximum de lignes (produits différents) par commande
const MAX_LINES_PER_ORDER: usize = 5;

/// Quantité minimale d'un produit par ligne de commande
const MIN_QTY_PER_LINE: i32 = 1;

/// Quantité maximale d'un produit par ligne de commande
const MAX_QTY_PER_LINE: i32 = 5;

/// Plage de dates sur laquelle répartir les commandes (en jours depuis aujourd'hui)
/// Par exemple, 730 = les commandes seront réparties sur les 730 derniers jours (2 ans)
/// Une plage plus large permet d'avoir des clients "habitués" avec un historique plus ancien
const ORDER_DATE_RANGE_DAYS: i32 = 730;

/// Pourcentage de commandes (0.0 à 1.0) qui seront placées dans la période "ancienne"
/// (au-delà de 365 jours). Cela garantit que certains clients auront un historique ancien
/// et seront considérés comme "clients fidèles" lors des analyses
const OLD_ORDERS_RATIO: f32 = 0.40; // 40% des commandes auront plus d'un an

/// Pourcentage d'utilisateurs (0.0 à 1.0) qui seront des "nouveaux clients"
/// (leur première commande sera dans la période récente, moins de 365 jours)
/// Le reste sera des "clients fidèles" (première commande il y a plus d'un an)
const NEW_CLIENTS_RATIO: f32 = 0.30; // 30% des utilisateurs seront de nouveaux clients

// ────────────── Produit forcé ──────────────
/// ID du produit à forcer dans un certain pourcentage de commandes (pour tester des analyses produit)
/// None = aucun produit forcé, Some(id) = forcer le produit avec cet ID
const FORCE_PRODUCT_ID: Option<i32> = Some(118);

/// Probabilité (0.0 à 1.0) qu'une commande contienne le produit forcé
/// 0.20 = 20% des commandes contiendront ce produit
const FORCE_PRODUCT_PROBABILITY: f32 = 0.20;

// ────────────── Réapprovisionnements (Restocks) ──────────────
/// Nombre total de restocks à générer dans la base de données
const NB_RESTOCKS: usize = 500;

/// Nombre minimum de produits différents par restock
const MIN_PRODUCTS_PER_RESTOCK: usize = 1;

/// Nombre maximum de produits différents par restock
const MAX_PRODUCTS_PER_RESTOCK: usize = 8;

/// Quantité minimale par produit dans un réapprovisionnement
const RESTOCK_QTY_MIN: i32 = 10;

/// Quantité maximale par produit dans un réapprovisionnement
const RESTOCK_QTY_MAX: i32 = 500;

/// Plage de dates sur laquelle répartir les restocks (en jours depuis aujourd'hui)
/// Par exemple, 365 = les restocks seront répartis sur l'année écoulée
const RESTOCK_DATE_RANGE_DAYS: i32 = 365;

// ────────────── RNG Seed ──────────────
/// Graine (seed) pour le générateur de nombres aléatoires
const RNG_SEED: u64 = 42;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL")?;

    // 🔁 Connexion avec retries
    let mut attempts = 0;

    let (mut client, connection) = loop {
        match tokio_postgres::connect(&db_url, NoTls).await {
            Ok((client, connection)) => break (client, connection),
            Err(err) => {
                attempts += 1;
                if attempts >= MAX_CONNECTION_ATTEMPTS {
                    eprintln!("❌ Échec de connexion BDD après {} tentatives : {}", attempts, err);
                    return Err(err.into());
                }
                eprintln!(
                    "⏳ Tentative {}/{} échouée. Nouvelle tentative dans {}s... ({})",
                    attempts, MAX_CONNECTION_ATTEMPTS, RETRY_DELAY_SECONDS, err
                );
                sleep(Duration::from_secs(RETRY_DELAY_SECONDS)).await;
            }
        }
    };

    // Surveille la connexion
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Erreur de connexion à la BDD : {}", e);
        }
    });

    // Optionnel : reset complet du jeu de données
    let do_reset = env::var("SEED_RESET").unwrap_or_else(|_| "1".into()) != "0";
    if do_reset {
        client.batch_execute(
            "
            TRUNCATE TABLE
              productrestockprices_prr,
              line_restock_lrs,
              restock_res,
              line_order_lor,
              order_ord,
              productprices_prp,
              products_pro,
              supplier_sup,
              role_user_rus,
              users_usr,
              role_rol
            RESTART IDENTITY CASCADE;
            "
        ).await?;
    }

    // ————————————————————————————————————————————————————————————
    // 1) RÔLES
    // ————————————————————————————————————————————————————————————
    println!("[1/8] Insertion des roles...");
    let roles = vec!["admin", "manager", "seller", "viewer", "user"];
    for role in &roles {
        client
            .execute(
                "INSERT INTO role_rol (name_rol) VALUES ($1)
                 ON CONFLICT (name_rol) DO NOTHING",
                &[role],
            )
            .await?;
    }
    println!("      -> {} roles inseres", roles.len());

    // ————————————————————————————————————————————————————————————
    // 2) UTILISATEURS (50)
    // ————————————————————————————————————————————————————————————
    println!("[2/8] Insertion des utilisateurs...");
    let mut users: Vec<(String, String, String, String, String)> = Vec::new();

    users.push((
        "admin@example.com".to_string(),
        "Root".to_string(),
        "Admin".to_string(),
        "adminpass".to_string(),
        "0600000001".to_string(),
    ));

    users.push((
        "manager@example.com".to_string(),
        "Lead".to_string(),
        "Manager".to_string(),
        "managerpass".to_string(),
        "0600000002".to_string(),
    ));

    // utilisateurs "classiques"
    for i in 1..=NB_USERS {
        let email = format!("user{:02}@example.com", i);
        let last  = format!("UserLast{:02}", i);
        let first = format!("UserFirst{:02}", i);
        let pass  = "password".to_string();
        let phone = format!("06{:08}", 1000 + i);
        users.push((email, last, first, pass, phone));
    }

    for (email, last, first, pass, phone) in &users {
        let hashed_password = security::hash_password(pass).map_err(|err| {
            anyhow::anyhow!("Failed to hash seed password for {}: {}", email, err)
        })?;
        client
            .execute(
                "INSERT INTO users_usr (email_usr, lastname_usr, firstname_usr, password_usr, phone_usr)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (email_usr) DO NOTHING",
                &[email, last, first, &hashed_password, phone],
            )
            .await?;
    }

    // Récupère les ids utilisateurs (email -> id)
    let rows = client
        .query("SELECT id_usr, email_usr FROM users_usr ORDER BY id_usr", &[])
        .await?;
    let users_idx: Vec<(i32, String)> = rows
        .into_iter()
        .map(|r| (r.get::<_, i32>(0), r.get::<_, String>(1)))
        .collect();
    println!("      -> {} utilisateurs inseres", users_idx.len());

    // ————————————————————————————————————————————————————————————
    // 3) ROLE_USER : attributions
    // ————————————————————————————————————————————————————————————
    println!("[3/8] Attribution des roles aux utilisateurs...");
    for (id, email) in &users_idx {
        let role_names: Vec<&str> = if email == "admin@example.com" {
            vec!["admin", "manager"]
        } else if email == "manager@example.com" {
            vec!["manager"]
        } else {
            if id % 3 == 0 { vec!["seller"] } else { vec!["user"] }
        };

        for role_name in role_names {
            client
                .execute(
                    "INSERT INTO role_user_rus (id_role_rus, id_user_rus)
                     SELECT r.id_rol, $2 FROM role_rol r
                     WHERE r.name_rol = $1
                     ON CONFLICT DO NOTHING",
                    &[&role_name, id],
                )
                .await?;
        }
    }
    println!("      -> Roles attribues");

    // ————————————————————————————————————————————————————————————
    // 4) FOURNISSEURS
    // ————————————————————————————————————————————————————————————
    println!("[4/8] Insertion des fournisseurs...");
    let suppliers: Vec<(String, String, String, String)> = (1..=NB_SUPPLIERS)
        .map(|i| {
            (
                format!("Supplier {}", i),
                format!("supplier{}@sup.com", i),
                format!("0601{:06}", i),
                format!("{} rue du Marché", i),
            )
        })
        .collect();

    for (name, email, phone, address) in &suppliers {
        client
            .execute(
                "INSERT INTO supplier_sup (name_sup, email_sup, phone_sup, address_sup)
                 VALUES ($1, $2, $3, $4)",
                &[name, email, phone, address],
            )
            .await?;
    }

    // Récupère ids fournisseurs
    let rows = client
        .query("SELECT id_sup FROM supplier_sup ORDER BY id_sup", &[])
        .await?;
    let supplier_ids: Vec<i32> = rows.into_iter().map(|r| r.get::<_, i32>(0)).collect();
    println!("      -> {} fournisseurs inseres", supplier_ids.len());

    // ————————————————————————————————————————————————————————————
    // 5) PRODUITS
    // ————————————————————————————————————————————————————————————
    println!("[5/8] Insertion des produits...");
    let categories = vec![
        "Alimentaire","Électronique","Hygiène","Maison",
        "Vêtements","Papeterie","Jardin","Jeux",
    ];

    let mut rng_thread = rand::thread_rng();
    let mut product_ids: Vec<i32> = Vec::new();
    let mut product_buy_prices: Vec<(i32, Decimal)> = Vec::new();

    for i in 1..=NB_PRODUCTS {
        let name = format!("Produit {}", i);
        let category = categories.choose(&mut rng_thread).unwrap().to_string();
        let reference = format!("REF{:04}", i);

        // fournisseur aléatoire
        let sup_id = *supplier_ids.choose(&mut rng_thread).unwrap();

        // stock (permet de tester ruptures)
        let stock: i32 = rng_thread.gen_range(STOCK_MIN..=STOCK_MAX);

        // prix d'achat
        let cents: i64 = rng_thread.gen_range(BUYING_PRICE_MIN_CENTS..=BUYING_PRICE_MAX_CENTS);
        let buying_price = Decimal::from_i64(cents).unwrap() / Decimal::from_i64(100).unwrap();

        // statut du produit en fonction du stock
        let status = if stock == 0 {
            // 70% out_of_stock, 30% ordered
            if rng_thread.gen_bool(0.7) { "out_of_stock" } else { "ordered" }
        } else if stock < 20 {
            // Low stock: 80% in_stock, 10% ordered, 10% discontinued
            let r = rng_thread.gen_range(0..10);
            if r < 8 { "in_stock" } else if r < 9 { "ordered" } else { "discontinued" }
        } else {
            // Good stock: 90% in_stock, 5% ordered, 5% discontinued
            let r = rng_thread.gen_range(0..100);
            if r < 90 { "in_stock" } else if r < 95 { "ordered" } else { "discontinued" }
        };

        // INSERT + RETURNING pour récupérer l'id
        // Note: We format the status directly in the SQL since it's a controlled value
        let sql = format!(
            "INSERT INTO products_pro
             (name_pro, category_pro, reference_pro, supplier_id_pro, stock_quantity_pro, buying_price_pro, status_pro, date_last_reassor_pro)
             VALUES ($1, $2, $3, $4, $5, $6, '{}'::product_status_enum, NOW())
             ON CONFLICT (name_pro) DO UPDATE SET
                category_pro = EXCLUDED.category_pro,
                reference_pro = EXCLUDED.reference_pro,
                supplier_id_pro = EXCLUDED.supplier_id_pro,
                stock_quantity_pro = EXCLUDED.stock_quantity_pro,
                buying_price_pro = EXCLUDED.buying_price_pro,
                status_pro = EXCLUDED.status_pro,
                date_last_reassor_pro = EXCLUDED.date_last_reassor_pro
             RETURNING id_pro, buying_price_pro",
            status
        );

        let row = client
            .query_one(
                &sql,
                &[&name, &category, &reference, &sup_id, &stock, &buying_price],
            )
            .await?;

        let id_pro: i32 = row.get(0);
        let bp: Decimal = row.get(1);
        product_ids.push(id_pro);
        product_buy_prices.push((id_pro, bp));
    }
    println!("      -> {} produits inseres", product_ids.len());

    // ————————————————————————————————————————————————————————————
    // 6) PRIX PRODUITS (prix de vente avec marge) - HISTORIQUE MULTIPLE
    // ————————————————————————————————————————————————————————————
    println!("[6/8] Insertion des prix de vente avec historique...");
    // (on nettoie au cas où)
    client.execute("DELETE FROM productprices_prp", &[]).await?;
    let margin = Decimal::from_str(SELLING_MARGIN_MULTIPLIER).unwrap();
    let mut rng_prices = StdRng::seed_from_u64(RNG_SEED + 2);
    let price_history_start = chrono::Utc::now() - chrono::Duration::days(ORDER_DATE_RANGE_DAYS as i64);

    let mut total_price_entries = 0;
    for (id_pro, buying_price) in &product_buy_prices {
        // Générer entre 2 et 5 changements de prix par produit pour simuler un historique
        let nb_price_changes = rng_prices.gen_range(2..=5);

        // Date de départ pour les prix (il y a 2 ans)
        let mut current_date = price_history_start;

        // Prix de base avec marge
        let mut current_price = (*buying_price * margin).round_dp(2);

        for i in 0..nb_price_changes {
            // Insérer le prix actuel
            client
                .execute(
                    "INSERT INTO productprices_prp (product_ref_prp, price_prp, created_at, updated_at)
                     VALUES ($1, $2, $3, $3)",
                    &[id_pro, &current_price, &current_date],
                )
                .await?;

            total_price_entries += 1;

            // Pour le prochain changement, appliquer une variation aléatoire
            if i < nb_price_changes - 1 {
                // Variation entre -3% et +8% (plus souvent des hausses)
                let variation = Decimal::from_i32(rng_prices.gen_range(-3..=8)).unwrap() / Decimal::from_i32(100).unwrap();
                current_price = (current_price * (Decimal::ONE + variation)).round_dp(2);

                // Avancer la date de quelques mois (60 à 180 jours entre chaque changement)
                let days_between_changes = rng_prices.gen_range(60..=180);
                current_date = current_date + chrono::Duration::days(days_between_changes);

                // S'assurer de ne pas dépasser la date actuelle
                if current_date > chrono::Utc::now() {
                    current_date = chrono::Utc::now() - chrono::Duration::days(rng_prices.gen_range(1..=30) as i64);
                }
            }
        }
    }

    // map des prix de vente en base (source de vérité pour les lignes - prendre le plus récent)
    let price_rows = client.query(
        "SELECT DISTINCT ON (p.id_pro) p.id_pro, pr.price_prp
         FROM products_pro p
         JOIN productprices_prp pr ON pr.product_ref_prp = p.id_pro
         ORDER BY p.id_pro, pr.created_at DESC", &[]
    ).await?;
    let mut price_map: HashMap<i32, Decimal> = HashMap::with_capacity(price_rows.len());
    for r in price_rows {
        price_map.insert(r.get::<_, i32>(0), r.get::<_, Decimal>(1));
    }
    println!("      -> {} entrées de prix de vente insérées ({} produits avec historique)", total_price_entries, price_map.len());

    // ————————————————————————————————————————————————————————————
    // 7) COMMANDES + LIGNES
    // ————————————————————————————————————————————————————————————
    println!("[7/8] Insertion des commandes et lignes de commandes...");
    let statuses = ["processing", "shipped", "delivered", "cancelled"];

    // RNG seedé pour des données reproductibles
    let mut rng = StdRng::seed_from_u64(RNG_SEED);

    // prépare les statements
    let stmt_insert_line = client
        .prepare(
            "INSERT INTO line_order_lor (order_id_lor, product_id_lor, quantity_lor, unit_price_lor, line_total_lor)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .await?;

    let all_product_ids = product_ids.clone();

    // Détermine pour chaque utilisateur s'il sera "nouveau client" ou "client fidèle"
    // en lui assignant une date minimale pour sa première commande
    let mut user_min_days: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
    for (user_id, _email) in &users_idx {
        let is_new_client = rng.gen::<f32>() < NEW_CLIENTS_RATIO;
        let min_days = if is_new_client {
            0
        } else {
            365
        };
        user_min_days.insert(*user_id, min_days);
    }

    // transaction globale pour perf + cohérence
    let trans = client.transaction().await?;

    for _ in 0..NB_ORDERS {
        let (user_id, _email) = users_idx.choose(&mut rng).expect("no users");
        let status = *statuses.choose(&mut rng).unwrap();

        // Récupère la contrainte de date minimale pour cet utilisateur
        let min_days = *user_min_days.get(user_id).unwrap();

        // Génération de la date de commande avec distribution bimodale :
        // - OLD_ORDERS_RATIO% des commandes seront "anciennes" (entre 365 et ORDER_DATE_RANGE_DAYS jours)
        // - Le reste sera "récent" (entre min_days et 365 jours)
        let days_ago = if rng.gen::<f32>() < OLD_ORDERS_RATIO && min_days >= 365 {
            // Commande ancienne (seulement pour les clients fidèles)
            rng.gen_range(365..=ORDER_DATE_RANGE_DAYS)
        } else {
            // Commande récente (respecte la contrainte min_days de l'utilisateur)
            rng.gen_range(min_days..=365.min(ORDER_DATE_RANGE_DAYS))
        };

        // nombre de lignes
        let lines = rng.gen_range(MIN_LINES_PER_ORDER..=MAX_LINES_PER_ORDER);

        // sélection de produits sans doublons
        let mut chosen: Vec<i32> = Vec::with_capacity(lines);

        // contrainte produit forcé avec probabilité
        if let Some(forced_pid) = FORCE_PRODUCT_ID {
            if rng.gen::<f32>() < FORCE_PRODUCT_PROBABILITY && all_product_ids.binary_search(&forced_pid).is_ok() {
                chosen.push(forced_pid);
            }
        }

        while chosen.len() < lines {
            let pid = *all_product_ids.choose(&mut rng).unwrap();
            if !chosen.contains(&pid) {
                chosen.push(pid);
            }
        }

        // crée l'ordre avec total provisoire 0.00 et la date calculée
        let zero = Decimal::from_i32(0).unwrap();
        let row = trans
            .query_one(
                "INSERT INTO order_ord (user_id_ord, order_date_ord, status_ord, amount_ord)
                 VALUES ($1, NOW() - $2::int * INTERVAL '1 day', $3, $4)
                 RETURNING id_ord",
                &[user_id, &days_ago, &status, &zero]
            )
            .await?;
        let order_id: i32 = row.get(0);

        // insère les lignes + calcule le total exact
        let mut order_total = Decimal::ZERO;
        for pid in chosen {
            let unit_price = *price_map.get(&pid).expect("price missing");
            let qty: i32 = rng.gen_range(MIN_QTY_PER_LINE..=MAX_QTY_PER_LINE);
            let line_total = (unit_price * Decimal::from_i32(qty).unwrap()).round_dp(2);

            trans
                .execute(&stmt_insert_line, &[&order_id, &pid, &qty, &unit_price, &line_total])
                .await?;

            order_total += line_total;
        }
        order_total = order_total.round_dp(2);

        // met à jour le total
        trans
            .execute(
                "UPDATE order_ord SET amount_ord = $1 WHERE id_ord = $2",
                &[&order_total, &order_id],
            )
            .await?;
    }

    trans.commit().await?;
    println!("      -> {} commandes avec lignes inserees", NB_ORDERS);

    // ————————————————————————————————————————————————————————————
    // 8) RESTOCKS (RÉAPPROVISIONNEMENTS) - Variété de cas
    // ————————————————————————————————————————————————————————————
    println!("[8/8] Insertion des restocks et lignes de restocks...");

    let mut rng = StdRng::seed_from_u64(RNG_SEED + 1); // Seed différent pour les restocks
    let restock_start_date = chrono::Utc::now() - chrono::Duration::days(RESTOCK_DATE_RANGE_DAYS as i64);

    // Construire une map des prix d'achat par produit pour les restocks
    let buying_price_map: HashMap<i32, Decimal> = product_buy_prices.iter()
        .map(|(id, price)| (*id, *price))
        .collect();

    // Groupe 1: Restocks réguliers avec quantités petites-moyennes (40% des restocks)
    let regular_restocks = (NB_RESTOCKS * 40) / 100;
    for _ in 0..regular_restocks {
        let days_offset = rng.gen_range(0..RESTOCK_DATE_RANGE_DAYS);
        let restock_date = restock_start_date + chrono::Duration::days(days_offset as i64);

        // Déterminer le nombre de produits dans ce restock
        let nb_products = rng.gen_range(MIN_PRODUCTS_PER_RESTOCK..=MAX_PRODUCTS_PER_RESTOCK);

        // Sélectionner des produits sans doublons
        let mut selected_products: Vec<i32> = Vec::with_capacity(nb_products);
        while selected_products.len() < nb_products {
            let pid = *product_ids.choose(&mut rng).unwrap();
            if !selected_products.contains(&pid) {
                selected_products.push(pid);
            }
        }

        // Calculer la quantité totale du restock
        let total_quantity: i32 = selected_products.iter()
            .map(|_| rng.gen_range(RESTOCK_QTY_MIN..=100))
            .sum();

        // Créer le restock et récupérer son ID
        let row = client
            .query_one(
                "INSERT INTO restock_res (quantity_res, supplier_id_res, status_res, restock_date_res, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, NOW(), NOW())
                 RETURNING id_res",
                &[&total_quantity, &supplier_ids.choose(&mut rng), &"pending", &restock_date],
            )
            .await?;
        let restock_id: i32 = row.get(0);

        // Créer une ligne de restock pour chaque produit
        for product_id in selected_products {
            let quantity = rng.gen_range(RESTOCK_QTY_MIN..=100);

            // Obtenir le prix d'achat avec une variation aléatoire (-5% à +5%)
            let base_buying_price = *buying_price_map.get(&product_id).expect("buying price missing");
            let variation = Decimal::from_i32(rng.gen_range(-5..=5)).unwrap() / Decimal::from_i32(100).unwrap();
            let unit_price = (base_buying_price * (Decimal::ONE + variation)).round_dp(2);

            // Créer la ligne de restock
            client
                .execute(
                    "INSERT INTO line_restock_lrs (restock_id_lrs, product_id_lrs, quantity_lrs, unit_price_lrs)
                     VALUES ($1, $2, $3, $4)",
                    &[&restock_id, &product_id, &quantity, &unit_price],
                )
                .await?;

            // Insérer manuellement dans productrestockprices_prr avec la date du restock
            client
                .execute(
                    "INSERT INTO productrestockprices_prr (product_ref_prr, buying_price_prr, restock_id_prr, restock_date_prr, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $4, $4)
                     ON CONFLICT DO NOTHING",
                    &[&product_id, &unit_price, &restock_id, &restock_date],
                )
                .await?;
        }
    }

    // Groupe 2: Gros restocks avec quantités importantes (25% des restocks)
    let bulk_restocks = (NB_RESTOCKS * 25) / 100;
    for _ in 0..bulk_restocks {
        let days_offset = rng.gen_range(0..RESTOCK_DATE_RANGE_DAYS);
        let restock_date = restock_start_date + chrono::Duration::days(days_offset as i64);

        let nb_products = rng.gen_range(MIN_PRODUCTS_PER_RESTOCK..=MAX_PRODUCTS_PER_RESTOCK);
        let mut selected_products: Vec<i32> = Vec::with_capacity(nb_products);
        while selected_products.len() < nb_products {
            let pid = *product_ids.choose(&mut rng).unwrap();
            if !selected_products.contains(&pid) {
                selected_products.push(pid);
            }
        }

        let total_quantity: i32 = selected_products.iter()
            .map(|_| rng.gen_range(200..=500))
            .sum();

        let row = client
            .query_one(
                "INSERT INTO restock_res (quantity_res, supplier_id_res, status_res, restock_date_res, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, NOW(), NOW())
                 RETURNING id_res",
                &[&total_quantity, &supplier_ids.choose(&mut rng), &"pending", &restock_date],
            )
            .await?;
        let restock_id: i32 = row.get(0);

        for product_id in selected_products {
            let quantity = rng.gen_range(200..=500);
            let base_buying_price = *buying_price_map.get(&product_id).expect("buying price missing");
            let variation = Decimal::from_i32(rng.gen_range(-5..=5)).unwrap() / Decimal::from_i32(100).unwrap();
            let unit_price = (base_buying_price * (Decimal::ONE + variation)).round_dp(2);

            client
                .execute(
                    "INSERT INTO line_restock_lrs (restock_id_lrs, product_id_lrs, quantity_lrs, unit_price_lrs)
                     VALUES ($1, $2, $3, $4)",
                    &[&restock_id, &product_id, &quantity, &unit_price],
                )
                .await?;

            client
                .execute(
                    "INSERT INTO productrestockprices_prr (product_ref_prr, buying_price_prr, restock_id_prr, restock_date_prr, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $4, $4)
                     ON CONFLICT DO NOTHING",
                    &[&product_id, &unit_price, &restock_id, &restock_date],
                )
                .await?;
        }
    }

    // Groupe 3: Restocks d'urgence avec petites quantités (20% des restocks)
    let emergency_restocks = (NB_RESTOCKS * 20) / 100;
    for _ in 0..emergency_restocks {
        let days_offset = rng.gen_range(0..=30); // Plutôt récents
        let restock_date = restock_start_date + chrono::Duration::days(days_offset as i64);

        // Restocks d'urgence ont souvent moins de produits (1-3)
        let nb_products = rng.gen_range(MIN_PRODUCTS_PER_RESTOCK..=3.min(MAX_PRODUCTS_PER_RESTOCK));
        let mut selected_products: Vec<i32> = Vec::with_capacity(nb_products);
        while selected_products.len() < nb_products {
            let pid = *product_ids.choose(&mut rng).unwrap();
            if !selected_products.contains(&pid) {
                selected_products.push(pid);
            }
        }

        let total_quantity: i32 = selected_products.iter()
            .map(|_| rng.gen_range(10..=50))
            .sum();

        let row = client
            .query_one(
                "INSERT INTO restock_res (quantity_res, supplier_id_res, status_res, restock_date_res, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, NOW(), NOW())
                 RETURNING id_res",
                &[&total_quantity, &supplier_ids.choose(&mut rng), &"pending", &restock_date],
            )
            .await?;
        let restock_id: i32 = row.get(0);

        for product_id in selected_products {
            let quantity = rng.gen_range(10..=50); // Petites quantités
            let base_buying_price = *buying_price_map.get(&product_id).expect("buying price missing");
            // Prix d'urgence souvent plus élevé (+3% à +10%)
            let variation = Decimal::from_i32(rng.gen_range(3..=10)).unwrap() / Decimal::from_i32(100).unwrap();
            let unit_price = (base_buying_price * (Decimal::ONE + variation)).round_dp(2);

            client
                .execute(
                    "INSERT INTO line_restock_lrs (restock_id_lrs, product_id_lrs, quantity_lrs, unit_price_lrs)
                     VALUES ($1, $2, $3, $4)",
                    &[&restock_id, &product_id, &quantity, &unit_price],
                )
                .await?;

            client
                .execute(
                    "INSERT INTO productrestockprices_prr (product_ref_prr, buying_price_prr, restock_id_prr, restock_date_prr, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $4, $4)
                     ON CONFLICT DO NOTHING",
                    &[&product_id, &unit_price, &restock_id, &restock_date],
                )
                .await?;
        }
    }

    // Groupe 4: Restocks saisonniers (15% des restocks)
    let seasonal_restocks = NB_RESTOCKS - regular_restocks - bulk_restocks - emergency_restocks;

    for _ in 0..seasonal_restocks {
        let days_offset = rng.gen_range(0..RESTOCK_DATE_RANGE_DAYS);
        let restock_date = restock_start_date + chrono::Duration::days(days_offset as i64);

        let nb_products = rng.gen_range(MIN_PRODUCTS_PER_RESTOCK..=MAX_PRODUCTS_PER_RESTOCK);
        let mut selected_products: Vec<i32> = Vec::with_capacity(nb_products);
        while selected_products.len() < nb_products {
            let pid = *product_ids.choose(&mut rng).unwrap();
            if !selected_products.contains(&pid) {
                selected_products.push(pid);
            }
        }

        let total_quantity: i32 = selected_products.iter()
            .map(|_| rng.gen_range(RESTOCK_QTY_MIN..=200))
            .sum();

        let row = client
            .query_one(
                "INSERT INTO restock_res (quantity_res, supplier_id_res, status_res, restock_date_res, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, NOW(), NOW())
                 RETURNING id_res",
                &[&total_quantity, &supplier_ids.choose(&mut rng), &"pending", &restock_date],
            )
            .await?;
        let restock_id: i32 = row.get(0);

        for product_id in selected_products {
            let quantity = rng.gen_range(RESTOCK_QTY_MIN..=200);
            let base_buying_price = *buying_price_map.get(&product_id).expect("buying price missing");
            let variation = Decimal::from_i32(rng.gen_range(-5..=5)).unwrap() / Decimal::from_i32(100).unwrap();
            let unit_price = (base_buying_price * (Decimal::ONE + variation)).round_dp(2);

            client
                .execute(
                    "INSERT INTO line_restock_lrs (restock_id_lrs, product_id_lrs, quantity_lrs, unit_price_lrs)
                     VALUES ($1, $2, $3, $4)",
                    &[&restock_id, &product_id, &quantity, &unit_price],
                )
                .await?;

            client
                .execute(
                    "INSERT INTO productrestockprices_prr (product_ref_prr, buying_price_prr, restock_id_prr, restock_date_prr, created_at, updated_at)
                     VALUES ($1, $2, $3, $4, $4, $4)
                     ON CONFLICT DO NOTHING",
                    &[&product_id, &unit_price, &restock_id, &restock_date],
                )
                .await?;
        }
    }

    println!("      -> {} restocks inseres (reguliers: {}, gros: {}, urgence: {}, saisonniers: ~{})",
        NB_RESTOCKS, regular_restocks, bulk_restocks, emergency_restocks, seasonal_restocks);

    println!("\nSeeding termine avec succes !");
    println!("Recap: {} users, {} suppliers, {} products, {} orders, {} restocks",
        NB_USERS + 2, NB_SUPPLIERS, NB_PRODUCTS, NB_ORDERS, NB_RESTOCKS);
    Ok(())
}