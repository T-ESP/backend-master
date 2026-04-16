use anyhow::Result;
use dotenvy::dotenv;
use rand::distributions::WeightedIndex;
use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand::rngs::StdRng;
use rand_distr::{LogNormal, Normal, Poisson, Distribution};
use rust_decimal::prelude::FromPrimitive;
use rust_decimal::Decimal;
use std::{env, time::Duration};
use std::f64::consts::PI;
use tokio::time::sleep;
use tokio_postgres::NoTls;
use std::collections::HashMap;
use stocks_api::common::security;
use chrono::Datelike;

// ═══════════════════════════════════════════════════════════════════════════
// CONFIGURATION GLOBALE DU SEEDER
// ═══════════════════════════════════════════════════════════════════════════

const MAX_CONNECTION_ATTEMPTS: usize = 10;
const RETRY_DELAY_SECONDS: u64 = 2;

const NB_USERS: usize = 850;
const NB_SUPPLIERS: usize = 20;
const NB_PRODUCTS: usize = 200;
const STOCK_MIN: i32 = 0;
const STOCK_MAX: i32 = 200;

// Orders: day-by-day generation with Poisson distribution
const ORDER_DATE_RANGE_DAYS: i64 = 730;
const BASE_DAILY_ORDERS: f64 = 20.0; // Poisson lambda (~15000 total over 730 days)
const MIN_LINES_PER_ORDER: usize = 1;
const MAX_LINES_PER_ORDER: usize = 8;

// Price history: 8-20 entries per product with trends and anomalies
const MIN_PRICE_CHANGES: usize = 8;
const MAX_PRICE_CHANGES: usize = 20;
const PRICE_ANOMALY_RATIO: f64 = 0.10;

// Sales anomaly: forced high/low volume product indices
const FORCE_HIGH_VOLUME: &[(usize, f64)] = &[(18, 0.15), (42, 0.10), (100, 0.08)];
const FORCE_LOW_VOLUME: &[usize] = &[55, 130, 175];

// Restocks
const NB_RESTOCKS: usize = 500;
const RESTOCK_DATE_RANGE_DAYS: i64 = 365;

const RNG_SEED: u64 = 42;

// ═══════════════════════════════════════════════════════════════════════════
// PRODUCT CATALOG DEFINITIONS (16 categories with realistic French products)
// ═══════════════════════════════════════════════════════════════════════════

struct CategoryDef {
    name: &'static str,
    products: &'static [&'static str],
    buying_price_min_cents: i64,
    buying_price_max_cents: i64,
    margin_min: f64,
    margin_max: f64,
}

const CATEGORIES: &[CategoryDef] = &[
    CategoryDef {
        name: "Alimentaire",
        products: &[
            "Riz Basmati 1kg", "Huile d'olive vierge 75cl", "Confiture de fraise 350g",
            "Pates penne 500g", "Farine de ble T55 1kg", "Sucre en poudre 1kg",
            "Sel de mer fin 500g", "Sauce tomate basilic 400g", "Moutarde de Dijon 200g",
            "Vinaigre balsamique 25cl", "Lentilles vertes 500g", "Couscous moyen 1kg",
            "Conserve de thon naturel 160g",
        ],
        buying_price_min_cents: 50,
        buying_price_max_cents: 800,
        margin_min: 0.10,
        margin_max: 0.25,
    },
    CategoryDef {
        name: "Boissons",
        products: &[
            "Eau minerale 6x1.5L", "Jus d'orange frais 1L", "Cafe moulu arabica 250g",
            "The vert bio 25 sachets", "Sirop de grenadine 75cl", "Limonade artisanale 75cl",
            "Lait demi-ecreme 1L", "Boisson energisante 50cl", "Chocolat chaud poudre 400g",
            "Infusion camomille 20 sachets", "Nectar de mangue 1L", "Biere blonde 6x33cl",
            "Vin rouge Bordeaux 75cl",
        ],
        buying_price_min_cents: 80,
        buying_price_max_cents: 1500,
        margin_min: 0.15,
        margin_max: 0.30,
    },
    CategoryDef {
        name: "Electronique",
        products: &[
            "Cable USB-C 1m", "Casque audio bluetooth", "Batterie externe 10000mAh",
            "Cle USB 64Go", "Chargeur rapide 20W", "Souris sans fil ergonomique",
            "Hub USB 4 ports", "Ecouteurs intra-auriculaires", "Support telephone voiture",
            "Lampe LED bureau USB", "Tapis de souris XL", "Webcam HD 1080p",
            "Enceinte bluetooth portable",
        ],
        buying_price_min_cents: 500,
        buying_price_max_cents: 20000,
        margin_min: 0.25,
        margin_max: 0.60,
    },
    CategoryDef {
        name: "Hygiene",
        products: &[
            "Savon liquide mains 500ml", "Dentifrice menthe fraiche 75ml",
            "Shampoing antipelliculaire 250ml", "Gel douche surgras 250ml",
            "Deodorant pierre d'alun 75g", "Creme hydratante visage 50ml",
            "Brosse a dents souple", "Cotons-tiges bambou x200", "Rasoir jetable lot de 5",
            "Mousse a raser 200ml", "Bain de bouche menthol 500ml",
            "Creme solaire SPF50 200ml", "Lingettes demaquillantes x25",
        ],
        buying_price_min_cents: 100,
        buying_price_max_cents: 2000,
        margin_min: 0.20,
        margin_max: 0.40,
    },
    CategoryDef {
        name: "Maison",
        products: &[
            "Ampoule LED E27 10W", "Drap housse 140x190 coton", "Torchon microfibre lot de 3",
            "Bougie parfumee vanille", "Rideau occultant 140x260", "Coussin decoratif 45x45",
            "Tapis de bain 50x80", "Poubelle pedale 30L", "Cintre bois lot de 5",
            "Boite de rangement 10L", "Cadre photo 20x30", "Plaid polaire 130x170",
            "Horloge murale silencieuse",
        ],
        buying_price_min_cents: 200,
        buying_price_max_cents: 8000,
        margin_min: 0.20,
        margin_max: 0.45,
    },
    CategoryDef {
        name: "Vetements",
        products: &[
            "T-shirt coton bio homme", "Jean slim femme bleu", "Chaussettes lot de 5 paires",
            "Pull col roule laine", "Veste legere impermeable", "Short sport homme",
            "Robe ete fleurie", "Chemise lin homme", "Legging sport femme",
            "Pyjama coton femme", "Echarpe laine merinos", "Casquette baseball",
            "Ceinture cuir marron",
        ],
        buying_price_min_cents: 300,
        buying_price_max_cents: 6000,
        margin_min: 0.40,
        margin_max: 0.65,
    },
    CategoryDef {
        name: "Papeterie",
        products: &[
            "Stylo bille bleu lot de 10", "Cahier A4 96 pages", "Classeur a levier A4",
            "Surligneur jaune fluo", "Bloc-notes A5 ligne", "Gomme blanche classique",
            "Ruban adhesif transparent", "Chemise cartonnee A4 lot 10", "Ciseaux 21cm",
            "Trousse scolaire", "Agrafeuse 24/6", "Correcteur liquide 20ml",
            "Marqueur permanent noir",
        ],
        buying_price_min_cents: 30,
        buying_price_max_cents: 1500,
        margin_min: 0.25,
        margin_max: 0.50,
    },
    CategoryDef {
        name: "Jardin",
        products: &[
            "Terreau universel 20L", "Secateur inox", "Graines de tomate cerise",
            "Arrosoir plastique 10L", "Gants de jardinage", "Pot terre cuite 30cm",
            "Engrais universel 1L", "Tuteur bambou 120cm x10", "Pulverisateur 1.5L",
            "Semences gazon 1kg", "Rateau 14 dents", "Bac a compost 300L",
            "Plantation de basilic",
        ],
        buying_price_min_cents: 200,
        buying_price_max_cents: 5000,
        margin_min: 0.20,
        margin_max: 0.40,
    },
    CategoryDef {
        name: "Jeux et Jouets",
        products: &[
            "Puzzle 1000 pieces paysage", "Jeu de cartes UNO", "Ballon de foot taille 5",
            "Jeu de societe Monopoly", "Peluche ours 30cm", "Lego Classic 500 pieces",
            "Poupee 32cm articulee", "Voiture telecommandee", "Jeu d'echecs bois",
            "Mikado geant exterieur", "Figurines animaux lot 12", "Pistolet a eau",
            "Circuit de billes",
        ],
        buying_price_min_cents: 300,
        buying_price_max_cents: 8000,
        margin_min: 0.30,
        margin_max: 0.55,
    },
    CategoryDef {
        name: "Bricolage",
        products: &[
            "Tournevis cruciforme PH2", "Ruban adhesif electricien", "Perceuse sans fil 18V",
            "Metre ruban 5m", "Niveau a bulle 40cm", "Pince multiprise",
            "Jeu de cles Allen", "Vis a bois 4x40 x200", "Scie a metaux",
            "Colle universelle 125ml", "Papier de verre grain 120", "Lampe frontale LED",
            "Boite a outils vide",
        ],
        buying_price_min_cents: 150,
        buying_price_max_cents: 10000,
        margin_min: 0.20,
        margin_max: 0.45,
    },
    CategoryDef {
        name: "Animaux",
        products: &[
            "Croquettes chat adulte 2kg", "Jouet souris pour chat", "Litiere agglomerante 10L",
            "Laisse chien nylon 1.2m", "Os a macher lot de 5", "Gamelle inox 0.5L",
            "Collier anti-puces chat", "Sac a croquettes hermetique", "Shampoing chien 250ml",
            "Griffoir chat carton", "Balle tennis pour chien", "Aquarium 20L kit complet",
            "Nourriture poisson flocons 100ml",
        ],
        buying_price_min_cents: 100,
        buying_price_max_cents: 4000,
        margin_min: 0.20,
        margin_max: 0.35,
    },
    CategoryDef {
        name: "Bebe",
        products: &[
            "Couches T3 pack 30", "Biberon 300ml anti-colique", "Lingettes nettoyantes x72",
            "Creme pour le change 100ml", "Bavoir coton lot de 3", "Sucette silicone 0-6 mois",
            "Body manches longues", "Doudou lapin", "Anneau de dentition",
            "Gel lavant doux 500ml", "Mouche-bebe manuel", "Thermometre frontal",
            "Lange mousseline 120x120",
        ],
        buying_price_min_cents: 200,
        buying_price_max_cents: 3500,
        margin_min: 0.25,
        margin_max: 0.45,
    },
    CategoryDef {
        name: "Sport",
        products: &[
            "Tapis de yoga 6mm", "Gourde isotherme 750ml", "Bande de resistance lot de 3",
            "Corde a sauter reglable", "Sac de sport 40L", "Genouillere sport",
            "Montre chronometre", "Balle de tennis x3", "Maillot de bain homme",
            "Lunettes de natation", "Serviette microfibre sport", "Haltere vinyle 2kg",
            "Sifflet arbitre",
        ],
        buying_price_min_cents: 500,
        buying_price_max_cents: 12000,
        margin_min: 0.25,
        margin_max: 0.50,
    },
    CategoryDef {
        name: "Beaute",
        products: &[
            "Creme hydratante nuit 50ml", "Mascara volume noir", "Vernis a ongles rouge",
            "Fond de teint fluide", "Rouge a levres mat", "Palette fards a paupieres",
            "Eau de toilette 50ml", "Pinceau maquillage set de 5", "Serum vitamine C 30ml",
            "Huile de coco bio 100ml", "Gommage visage 75ml", "Masque capillaire 200ml",
            "Brume fixante maquillage 100ml",
        ],
        buying_price_min_cents: 300,
        buying_price_max_cents: 6000,
        margin_min: 0.35,
        margin_max: 0.60,
    },
    CategoryDef {
        name: "Epicerie fine",
        products: &[
            "Chocolat noir 70% 100g", "Miel de lavande 250g", "Huile de truffe 10cl",
            "Foie gras de canard 200g", "Confiture de figue artisanale 250g",
            "Vinaigre de Xeres 25cl", "Fleur de sel de Guerande 250g",
            "Piment d'Espelette 50g", "Biscuits sables bretons 200g",
            "Nougat de Montelimar 200g", "Calisson d'Aix lot de 12",
            "Pate d'amande 250g", "Marrons glaces x6",
        ],
        buying_price_min_cents: 400,
        buying_price_max_cents: 5000,
        margin_min: 0.30,
        margin_max: 0.55,
    },
    CategoryDef {
        name: "Entretien",
        products: &[
            "Lessive liquide 2L", "Eponge grattante lot de 3", "Desodorisant spray lavande",
            "Nettoyant multi-surfaces 750ml", "Liquide vaisselle 500ml",
            "Javel concentree 1L", "Sacs poubelle 50L x20", "Gants menage taille M",
            "Balai microfibre", "Serpillere 50x60", "Nettoyant vitres 500ml",
            "Pastilles lave-vaisselle x30", "Detartrant WC gel 750ml",
        ],
        buying_price_min_cents: 100,
        buying_price_max_cents: 1500,
        margin_min: 0.15,
        margin_max: 0.30,
    },
];

// ═══════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════

/// Weekly seasonality: busier Tue-Thu, slower weekends
fn weekly_multiplier(weekday: chrono::Weekday) -> f64 {
    match weekday {
        chrono::Weekday::Mon => 0.90,
        chrono::Weekday::Tue => 1.15,
        chrono::Weekday::Wed => 1.20,
        chrono::Weekday::Thu => 1.15,
        chrono::Weekday::Fri => 1.00,
        chrono::Weekday::Sat => 0.70,
        chrono::Weekday::Sun => 0.50,
    }
}

/// Yearly seasonality: cosine wave peaking in December + holiday spikes
fn yearly_multiplier(day_of_year: u32, month: u32) -> f64 {
    // Cosine wave: peak ~Dec 10 (day 345), trough ~June 10 (day 162)
    let base = 1.0 + 0.15 * ((2.0 * PI * (day_of_year as f64 - 345.0)) / 365.0).cos();

    // Holiday multipliers
    let holiday = if month == 11 && day_of_year >= 325 && day_of_year <= 330 {
        2.0 // Black Friday week
    } else if month == 12 && day_of_year <= 357 {
        1.5 // Christmas run-up (Dec 1-23)
    } else if month == 12 {
        0.6 // Christmas week (Dec 24-31)
    } else if month == 2 && day_of_year >= 40 && day_of_year <= 45 {
        1.3 // Valentine's Day
    } else if month == 9 && day_of_year >= 244 && day_of_year <= 260 {
        1.25 // Back to school
    } else if month == 7 && day_of_year >= 182 && day_of_year <= 200 {
        1.2 // Summer sales
    } else {
        1.0
    };

    base * holiday
}

/// Lookup the selling price active at a given date (price history sorted chronologically)
fn price_at_date(history: &[(chrono::DateTime<chrono::Utc>, Decimal)], date: chrono::DateTime<chrono::Utc>) -> Decimal {
    let mut result = history[0].1;
    for (d, p) in history.iter() {
        if *d <= date {
            result = *p;
        } else {
            break;
        }
    }
    result
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN
// ═══════════════════════════════════════════════════════════════════════════

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
                    eprintln!("Echec de connexion BDD apres {} tentatives : {}", attempts, err);
                    return Err(err.into());
                }
                eprintln!(
                    "Tentative {}/{} echouee. Nouvelle tentative dans {}s... ({})",
                    attempts, MAX_CONNECTION_ATTEMPTS, RETRY_DELAY_SECONDS, err
                );
                sleep(Duration::from_secs(RETRY_DELAY_SECONDS)).await;
            }
        }
    };

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Erreur de connexion BDD : {}", e);
        }
    });

    // Optionnel : reset complet
    let do_reset = env::var("SEED_RESET").unwrap_or_else(|_| "1".into()) != "0";
    if do_reset {
        client.batch_execute(
            "TRUNCATE TABLE
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
            RESTART IDENTITY CASCADE;"
        ).await?;
    }

    // ════════════════════════════════════════════════════════════════════
    // 1) ROLES
    // ════════════════════════════════════════════════════════════════════
    println!("[1/8] Insertion des roles...");
    let roles = vec!["admin", "manager", "seller", "viewer", "user"];
    for role in &roles {
        client
            .execute(
                "INSERT INTO role_rol (name_rol) VALUES ($1) ON CONFLICT (name_rol) DO NOTHING",
                &[role],
            )
            .await?;
    }
    println!("      -> {} roles inseres", roles.len());

    // ════════════════════════════════════════════════════════════════════
    // 2) UTILISATEURS
    // ════════════════════════════════════════════════════════════════════
    println!("[2/8] Insertion des utilisateurs...");
    let mut users: Vec<(String, String, String, String, String)> = Vec::new();

    users.push((
        "admin@example.com".into(), "Root".into(), "Admin".into(),
        "adminpass".into(), "0600000001".into(),
    ));
    users.push((
        "manager@example.com".into(), "Lead".into(), "Manager".into(),
        "managerpass".into(), "0600000002".into(),
    ));

    for i in 1..=NB_USERS {
        users.push((
            format!("user{:02}@example.com", i),
            format!("UserLast{:02}", i),
            format!("UserFirst{:02}", i),
            "password".into(),
            format!("06{:08}", 1000 + i),
        ));
    }

    for (email, last, first, pass, phone) in &users {
        let hashed = security::hash_password(pass)
            .map_err(|e| anyhow::anyhow!("Hash error for {}: {}", email, e))?;
        client
            .execute(
                "INSERT INTO users_usr (email_usr, lastname_usr, firstname_usr, password_usr, phone_usr)
                 VALUES ($1, $2, $3, $4, $5) ON CONFLICT (email_usr) DO NOTHING",
                &[email, last, first, &hashed, phone],
            )
            .await?;
    }

    let rows = client.query("SELECT id_usr, email_usr FROM users_usr ORDER BY id_usr", &[]).await?;
    let users_idx: Vec<(i32, String)> = rows.into_iter().map(|r| (r.get(0), r.get(1))).collect();
    println!("      -> {} utilisateurs inseres", users_idx.len());

    // ════════════════════════════════════════════════════════════════════
    // 3) ROLE_USER
    // ════════════════════════════════════════════════════════════════════
    println!("[3/8] Attribution des roles...");
    for (id, email) in &users_idx {
        let role_names: Vec<&str> = if email == "admin@example.com" {
            vec!["admin", "manager"]
        } else if email == "manager@example.com" {
            vec!["manager"]
        } else if id % 3 == 0 {
            vec!["seller"]
        } else {
            vec!["user"]
        };
        for role_name in role_names {
            client
                .execute(
                    "INSERT INTO role_user_rus (id_role_rus, id_user_rus)
                     SELECT r.id_rol, $2 FROM role_rol r WHERE r.name_rol = $1
                     ON CONFLICT DO NOTHING",
                    &[&role_name, id],
                )
                .await?;
        }
    }
    println!("      -> Roles attribues");

    // ════════════════════════════════════════════════════════════════════
    // 4) FOURNISSEURS
    // ════════════════════════════════════════════════════════════════════
    println!("[4/8] Insertion des fournisseurs...");
    let supplier_names = [
        "ProDistrib France", "AlphaSupply", "MegaStock Europe", "FreshLine Import",
        "NordLogistique", "SudExpress Distrib", "OuestAppro", "EstPartners",
        "EuroFournisseur", "TransAlpes Supply", "AtlantiqueGros", "MediTrade",
        "CentreDistrib", "RhoneAlpes Appro", "BretagneSupply", "ProvenceStock",
        "AquitainePro", "NormandieTrade", "AlsaceImport", "PicardieGros",
    ];
    for i in 0..NB_SUPPLIERS {
        let name = supplier_names[i];
        client
            .execute(
                "INSERT INTO supplier_sup (name_sup, email_sup, phone_sup, address_sup)
                 VALUES ($1, $2, $3, $4)",
                &[
                    &name.to_string(),
                    &format!("contact@{}.com", name.to_lowercase().replace(' ', "")),
                    &format!("0601{:06}", i + 1),
                    &format!("{} zone industrielle", i + 1),
                ],
            )
            .await?;
    }

    let rows = client.query("SELECT id_sup FROM supplier_sup ORDER BY id_sup", &[]).await?;
    let supplier_ids: Vec<i32> = rows.into_iter().map(|r| r.get(0)).collect();
    println!("      -> {} fournisseurs inseres", supplier_ids.len());

    // ════════════════════════════════════════════════════════════════════
    // 5) PRODUITS (with popularity weights, variability classes, price trends)
    // ════════════════════════════════════════════════════════════════════
    println!("[5/8] Insertion des produits...");
    let mut rng = StdRng::seed_from_u64(RNG_SEED);
    let ln_dist = LogNormal::new(0.0_f64, 1.0_f64).unwrap();

    let mut product_ids: Vec<i32> = Vec::with_capacity(NB_PRODUCTS);
    let mut product_buy_prices: Vec<(i32, Decimal)> = Vec::with_capacity(NB_PRODUCTS);
    let mut popularity_weights: Vec<f64> = Vec::with_capacity(NB_PRODUCTS);
    let mut variability_classes: Vec<u8> = Vec::with_capacity(NB_PRODUCTS); // 0=stable 1=moderate 2=erratic
    let mut price_trends: Vec<u8> = Vec::with_capacity(NB_PRODUCTS); // 0=rising 1=stable 2=declining
    let mut product_margins: Vec<f64> = Vec::with_capacity(NB_PRODUCTS);

    for i in 0..NB_PRODUCTS {
        let cat_idx = i % CATEGORIES.len();
        let prod_idx = i / CATEGORIES.len();
        let cat = &CATEGORIES[cat_idx];
        let name = cat.products[prod_idx].to_string();
        let category = cat.name.to_string();
        let reference = format!("REF{:04}", i + 1);

        // Supplier: round-robin across all suppliers
        let sup_id = supplier_ids[i % supplier_ids.len()];

        // Stock
        let stock: i32 = rng.gen_range(STOCK_MIN..=STOCK_MAX);

        // Buying price from category-specific range
        let cents: i64 = rng.gen_range(cat.buying_price_min_cents..=cat.buying_price_max_cents);
        let buying_price = Decimal::from_i64(cents).unwrap() / Decimal::from_i64(100).unwrap();

        // Margin from category-specific range
        let margin: f64 = rng.gen_range(cat.margin_min..=cat.margin_max);
        product_margins.push(margin);

        // Popularity weight (LogNormal distribution for Pareto-like revenue)
        let weight: f64 = ln_dist.sample(&mut rng);
        popularity_weights.push(weight);

        // Variability class: 30% stable, 40% moderate, 30% erratic
        let var_r: f64 = rng.gen();
        variability_classes.push(if var_r < 0.30 { 0 } else if var_r < 0.70 { 1 } else { 2 });

        // Price trend: 30% rising, 40% stable, 30% declining
        let trend_r: f64 = rng.gen();
        price_trends.push(if trend_r < 0.30 { 0 } else if trend_r < 0.70 { 1 } else { 2 });

        // Status based on stock level
        let status = if stock == 0 {
            if rng.gen_bool(0.7) { "out_of_stock" } else { "ordered" }
        } else if stock < 20 {
            let r = rng.gen_range(0..10);
            if r < 8 { "in_stock" } else if r < 9 { "ordered" } else { "discontinued" }
        } else {
            let r = rng.gen_range(0..100);
            if r < 90 { "in_stock" } else if r < 95 { "ordered" } else { "discontinued" }
        };

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
            .query_one(&sql, &[&name, &category, &reference, &sup_id, &stock, &buying_price])
            .await?;

        let id_pro: i32 = row.get(0);
        let bp: Decimal = row.get(1);
        product_ids.push(id_pro);
        product_buy_prices.push((id_pro, bp));
    }

    // Compute popularity tiers from weight distribution
    let mut sorted_w = popularity_weights.clone();
    sorted_w.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p30 = sorted_w[(NB_PRODUCTS as f64 * 0.30) as usize];
    let p70 = sorted_w[(NB_PRODUCTS as f64 * 0.70) as usize];
    let popularity_tiers: Vec<u8> = popularity_weights.iter()
        .map(|w| if *w > p70 { 2 } else if *w > p30 { 1 } else { 0 })
        .collect();

    // Build WeightedIndex for product selection
    let product_dist = WeightedIndex::new(&popularity_weights).unwrap();

    println!("      -> {} produits inseres (16 categories, marges {:.0}%-{:.0}%)",
        product_ids.len(),
        product_margins.iter().cloned().fold(f64::INFINITY, f64::min) * 100.0,
        product_margins.iter().cloned().fold(f64::NEG_INFINITY, f64::max) * 100.0);

    // ════════════════════════════════════════════════════════════════════
    // 6) PRIX PRODUITS - Historique enrichi avec tendances et anomalies
    // ════════════════════════════════════════════════════════════════════
    println!("[6/8] Insertion des prix de vente (historique enrichi)...");
    client.execute("DELETE FROM productprices_prp", &[]).await?;

    let price_start = chrono::Utc::now() - chrono::Duration::days(ORDER_DATE_RANGE_DAYS);
    let mut rng_prices = StdRng::seed_from_u64(RNG_SEED + 2);
    let mut price_history: HashMap<i32, Vec<(chrono::DateTime<chrono::Utc>, Decimal)>> = HashMap::new();
    let mut total_price_entries = 0;

    for idx in 0..NB_PRODUCTS {
        let (id_pro, buying_price) = product_buy_prices[idx];
        let margin = product_margins[idx];
        let trend = price_trends[idx];
        let nb_changes = rng_prices.gen_range(MIN_PRICE_CHANGES..=MAX_PRICE_CHANGES);

        // Initial selling price with category-specific margin
        let multiplier = Decimal::from_f64(1.0 + margin).unwrap();
        let base_price = (buying_price * multiplier).round_dp(2);

        // Build price entries in memory first
        let mut entries: Vec<(chrono::DateTime<chrono::Utc>, Decimal)> = Vec::with_capacity(nb_changes);
        let mut current_price = base_price;
        let mut current_date = price_start;

        for j in 0..nb_changes {
            entries.push((current_date, current_price));

            if j < nb_changes - 1 {
                // Apply trend-based price change
                let variation_pct = match trend {
                    0 => rng_prices.gen_range(1.0..=5.0) / 100.0,   // Rising
                    1 => rng_prices.gen_range(-2.0..=2.0) / 100.0,  // Stable
                    _ => rng_prices.gen_range(-4.0..=-1.0) / 100.0,  // Declining
                };
                let factor = Decimal::from_f64(1.0 + variation_pct).unwrap();
                current_price = (current_price * factor).round_dp(2);

                // Ensure price never drops below buying price
                if current_price < buying_price {
                    current_price = (buying_price * Decimal::from_f64(1.0 + margin * 0.5).unwrap()).round_dp(2);
                }

                // Advance date (spread entries evenly across 730 days)
                let days_between = rng_prices.gen_range(20..=90);
                current_date = current_date + chrono::Duration::days(days_between);
                if current_date > chrono::Utc::now() {
                    current_date = chrono::Utc::now() - chrono::Duration::days(rng_prices.gen_range(1..=15));
                }
            }
        }

        // Inject price anomalies for ~10% of products
        if rng_prices.gen::<f64>() < PRICE_ANOMALY_RATIO && entries.len() > 3 {
            let n_anomalies = rng_prices.gen_range(1..=2);
            for _ in 0..n_anomalies {
                let anom_idx = rng_prices.gen_range(1..entries.len() - 1);
                let factor = if rng_prices.gen_bool(0.5) {
                    rng_prices.gen_range(1.30..=1.50) // 30-50% spike
                } else {
                    rng_prices.gen_range(0.65..=0.80) // 20-35% drop
                };
                let anomaly_price = (entries[anom_idx].1 * Decimal::from_f64(factor).unwrap()).round_dp(2);
                entries[anom_idx].1 = anomaly_price;
            }
        }

        // Insert all price entries into DB
        for (date, price) in &entries {
            client
                .execute(
                    "INSERT INTO productprices_prp (product_ref_prp, price_prp, created_at, updated_at)
                     VALUES ($1, $2, $3, $3)",
                    &[&id_pro, price, date],
                )
                .await?;
            total_price_entries += 1;
        }

        // Store for time-aware order pricing
        price_history.insert(id_pro, entries);
    }

    // Build latest-price map (for restocks)
    let price_rows = client.query(
        "SELECT DISTINCT ON (p.id_pro) p.id_pro, pr.price_prp
         FROM products_pro p
         JOIN productprices_prp pr ON pr.product_ref_prp = p.id_pro
         ORDER BY p.id_pro, pr.created_at DESC", &[]
    ).await?;
    let mut price_map: HashMap<i32, Decimal> = HashMap::with_capacity(price_rows.len());
    for r in price_rows {
        price_map.insert(r.get(0), r.get(1));
    }
    println!("      -> {} entrees de prix ({} produits, tendances: rising/stable/declining)",
        total_price_entries, price_map.len());

    // ════════════════════════════════════════════════════════════════════
    // 7) COMMANDES + LIGNES - Day-by-day with seasonal patterns
    // ════════════════════════════════════════════════════════════════════
    println!("[7/8] Generation des commandes (saisonnalite + Pareto)...");
    let statuses = ["processing", "shipped", "delivered", "cancelled"];
    let start_date = chrono::Utc::now() - chrono::Duration::days(ORDER_DATE_RANGE_DAYS);

    let stmt_insert_line = client
        .prepare(
            "INSERT INTO line_order_lor (order_id_lor, product_id_lor, quantity_lor, unit_price_lor, line_total_lor)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .await?;

    let trans = client.transaction().await?;
    let mut total_orders: usize = 0;
    let mut total_lines: usize = 0;

    for day_offset in 0..ORDER_DATE_RANGE_DAYS {
        let base_date = start_date + chrono::Duration::days(day_offset);
        let weekday = base_date.weekday();
        let day_of_year = base_date.ordinal();
        let month = base_date.month();

        // Compute daily order rate with seasonality
        let rate = BASE_DAILY_ORDERS
            * weekly_multiplier(weekday)
            * yearly_multiplier(day_of_year, month)
            * (1.0 + 0.10 * (day_offset as f64 / ORDER_DATE_RANGE_DAYS as f64)); // 10% growth trend

        // Sample number of orders for this day
        let n_orders = Poisson::new(rate.max(0.1)).unwrap().sample(&mut rng) as usize;

        for _ in 0..n_orders {
            let (user_id, _) = users_idx.choose(&mut rng).expect("no users");
            let status = *statuses.choose(&mut rng).unwrap();

            // Add random time of day
            let hours: i64 = rng.gen_range(8..=22);
            let minutes: i64 = rng.gen_range(0..=59);
            let order_date = base_date
                + chrono::Duration::hours(hours)
                + chrono::Duration::minutes(minutes);

            // Determine number of line items
            let n_lines = rng.gen_range(MIN_LINES_PER_ORDER..=MAX_LINES_PER_ORDER);
            let mut chosen: Vec<(i32, usize)> = Vec::with_capacity(n_lines); // (product_id, product_index)

            // Inject forced high-volume products
            for &(prod_idx, probability) in FORCE_HIGH_VOLUME {
                if prod_idx < product_ids.len() && rng.gen::<f64>() < probability {
                    let pid = product_ids[prod_idx];
                    if !chosen.iter().any(|(id, _)| *id == pid) {
                        chosen.push((pid, prod_idx));
                    }
                }
            }

            // Fill remaining lines with weighted product selection
            let mut attempts = 0;
            while chosen.len() < n_lines && attempts < n_lines * 3 {
                attempts += 1;
                let prod_idx = product_dist.sample(&mut rng);
                let pid = product_ids[prod_idx];

                // Skip forced low-volume products 90% of the time
                if FORCE_LOW_VOLUME.contains(&prod_idx) && rng.gen::<f64>() < 0.90 {
                    continue;
                }

                if !chosen.iter().any(|(id, _)| *id == pid) {
                    chosen.push((pid, prod_idx));
                }
            }

            // Create order
            let zero = Decimal::from_i32(0).unwrap();
            let row = trans
                .query_one(
                    "INSERT INTO order_ord (user_id_ord, order_date_ord, status_ord, amount_ord)
                     VALUES ($1, $2, $3, $4) RETURNING id_ord",
                    &[user_id, &order_date, &status, &zero],
                )
                .await?;
            let order_id: i32 = row.get(0);

            // Create line items with variability-class-based quantities
            let mut order_total = Decimal::ZERO;
            for (pid, prod_idx) in &chosen {
                let tier = popularity_tiers[*prod_idx];
                let var_class = variability_classes[*prod_idx];

                // Base quantity by popularity tier
                let (mean, _) = match tier {
                    2 => (6.0, 2.0),  // High popularity
                    1 => (3.0, 1.5),  // Medium
                    _ => (1.5, 0.7),  // Low
                };

                // Quantity with variability class
                let qty_f: f64 = match var_class {
                    0 => {
                        // Stable (X target): narrow spread, CV < 0.5
                        Normal::new(mean, mean * 0.15).unwrap().sample(&mut rng)
                    }
                    1 => {
                        // Moderate (Y target): medium spread, CV 0.5-1.0
                        Normal::new(mean, mean * 0.55).unwrap().sample(&mut rng)
                    }
                    _ => {
                        // Erratic (Z target): wide spread with spikes, CV >= 1.0
                        let spike: f64 = rng.gen();
                        if spike < 0.12 {
                            mean * rng.gen_range(4.0..=8.0) // Big spike
                        } else if spike < 0.22 {
                            1.0 // Near-zero
                        } else {
                            Normal::new(mean, mean * 0.9).unwrap().sample(&mut rng)
                        }
                    }
                };
                let qty = (qty_f.round() as i32).max(1).min(30);

                // Time-aware price lookup
                let unit_price = if let Some(hist) = price_history.get(pid) {
                    price_at_date(hist, order_date)
                } else {
                    *price_map.get(pid).expect("price missing")
                };

                let line_total = (unit_price * Decimal::from_i32(qty).unwrap()).round_dp(2);

                trans
                    .execute(&stmt_insert_line, &[&order_id, pid, &qty, &unit_price, &line_total])
                    .await?;
                order_total += line_total;
                total_lines += 1;
            }

            // Update order total
            order_total = order_total.round_dp(2);
            trans
                .execute(
                    "UPDATE order_ord SET amount_ord = $1 WHERE id_ord = $2",
                    &[&order_total, &order_id],
                )
                .await?;
            total_orders += 1;
        }
    }

    trans.commit().await?;
    println!("      -> {} commandes, {} lignes (Poisson + saisonnalite hebdo/annuelle + Pareto)", total_orders, total_lines);

    // ════════════════════════════════════════════════════════════════════
    // 8) RESTOCKS - Popularity-weighted product selection
    // ════════════════════════════════════════════════════════════════════
    println!("[8/8] Insertion des restocks...");
    let mut rng = StdRng::seed_from_u64(RNG_SEED + 1);
    let restock_start = chrono::Utc::now() - chrono::Duration::days(RESTOCK_DATE_RANGE_DAYS);

    let buying_price_map: HashMap<i32, Decimal> = product_buy_prices.iter()
        .map(|(id, price)| (*id, *price))
        .collect();

    // Helper closure: select N unique products using weighted distribution
    let select_products = |rng: &mut StdRng, n: usize| -> Vec<(i32, usize)> {
        let mut selected: Vec<(i32, usize)> = Vec::with_capacity(n);
        let mut attempts = 0;
        while selected.len() < n && attempts < n * 5 {
            attempts += 1;
            let idx = product_dist.sample(rng);
            let pid = product_ids[idx];
            if !selected.iter().any(|(id, _)| *id == pid) {
                selected.push((pid, idx));
            }
        }
        selected
    };

    // Groupe 1: Restocks reguliers (40%)
    let regular_restocks = (NB_RESTOCKS * 40) / 100;
    for _ in 0..regular_restocks {
        let days_offset = rng.gen_range(0..RESTOCK_DATE_RANGE_DAYS);
        let restock_date = restock_start + chrono::Duration::days(days_offset);
        let nb_products = rng.gen_range(1..=8);
        let selected = select_products(&mut rng, nb_products);

        let total_quantity: i32 = selected.iter().map(|_| rng.gen_range(10..=100)).sum();
        let row = client.query_one(
            "INSERT INTO restock_res (quantity_res, supplier_id_res, status_res, restock_date_res, created_at, updated_at)
             VALUES ($1, $2, $3, $4, NOW(), NOW()) RETURNING id_res",
            &[&total_quantity, &supplier_ids.choose(&mut rng), &"pending", &restock_date],
        ).await?;
        let restock_id: i32 = row.get(0);

        for (product_id, _) in &selected {
            let quantity = rng.gen_range(10..=100);
            let base_bp = *buying_price_map.get(product_id).expect("buying price");
            let variation = Decimal::from_i32(rng.gen_range(-5..=5)).unwrap() / Decimal::from_i32(100).unwrap();
            let unit_price = (base_bp * (Decimal::ONE + variation)).round_dp(2);

            client.execute(
                "INSERT INTO line_restock_lrs (restock_id_lrs, product_id_lrs, quantity_lrs, unit_price_lrs) VALUES ($1, $2, $3, $4)",
                &[&restock_id, product_id, &quantity, &unit_price],
            ).await?;
            client.execute(
                "INSERT INTO productrestockprices_prr (product_ref_prr, buying_price_prr, restock_id_prr, restock_date_prr, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $4, $4) ON CONFLICT DO NOTHING",
                &[product_id, &unit_price, &restock_id, &restock_date],
            ).await?;
        }
    }

    // Groupe 2: Gros restocks (25%)
    let bulk_restocks = (NB_RESTOCKS * 25) / 100;
    for _ in 0..bulk_restocks {
        let days_offset = rng.gen_range(0..RESTOCK_DATE_RANGE_DAYS);
        let restock_date = restock_start + chrono::Duration::days(days_offset);
        let nb_products = rng.gen_range(1..=8);
        let selected = select_products(&mut rng, nb_products);

        let total_quantity: i32 = selected.iter().map(|_| rng.gen_range(200..=500)).sum();
        let row = client.query_one(
            "INSERT INTO restock_res (quantity_res, supplier_id_res, status_res, restock_date_res, created_at, updated_at)
             VALUES ($1, $2, $3, $4, NOW(), NOW()) RETURNING id_res",
            &[&total_quantity, &supplier_ids.choose(&mut rng), &"pending", &restock_date],
        ).await?;
        let restock_id: i32 = row.get(0);

        for (product_id, _) in &selected {
            let quantity = rng.gen_range(200..=500);
            let base_bp = *buying_price_map.get(product_id).expect("buying price");
            let variation = Decimal::from_i32(rng.gen_range(-5..=5)).unwrap() / Decimal::from_i32(100).unwrap();
            let unit_price = (base_bp * (Decimal::ONE + variation)).round_dp(2);

            client.execute(
                "INSERT INTO line_restock_lrs (restock_id_lrs, product_id_lrs, quantity_lrs, unit_price_lrs) VALUES ($1, $2, $3, $4)",
                &[&restock_id, product_id, &quantity, &unit_price],
            ).await?;
            client.execute(
                "INSERT INTO productrestockprices_prr (product_ref_prr, buying_price_prr, restock_id_prr, restock_date_prr, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $4, $4) ON CONFLICT DO NOTHING",
                &[product_id, &unit_price, &restock_id, &restock_date],
            ).await?;
        }
    }

    // Groupe 3: Restocks d'urgence (20%) - recent dates
    let emergency_restocks = (NB_RESTOCKS * 20) / 100;
    for _ in 0..emergency_restocks {
        // Emergency restocks are recent (last 30 days)
        let days_offset = rng.gen_range((RESTOCK_DATE_RANGE_DAYS - 30)..=RESTOCK_DATE_RANGE_DAYS);
        let restock_date = restock_start + chrono::Duration::days(days_offset);
        let nb_products = rng.gen_range(1..=3);
        let selected = select_products(&mut rng, nb_products);

        let total_quantity: i32 = selected.iter().map(|_| rng.gen_range(10..=50)).sum();
        let row = client.query_one(
            "INSERT INTO restock_res (quantity_res, supplier_id_res, status_res, restock_date_res, created_at, updated_at)
             VALUES ($1, $2, $3, $4, NOW(), NOW()) RETURNING id_res",
            &[&total_quantity, &supplier_ids.choose(&mut rng), &"pending", &restock_date],
        ).await?;
        let restock_id: i32 = row.get(0);

        for (product_id, _) in &selected {
            let quantity = rng.gen_range(10..=50);
            let base_bp = *buying_price_map.get(product_id).expect("buying price");
            // Emergency: higher prices (+3% to +10%)
            let variation = Decimal::from_i32(rng.gen_range(3..=10)).unwrap() / Decimal::from_i32(100).unwrap();
            let unit_price = (base_bp * (Decimal::ONE + variation)).round_dp(2);

            client.execute(
                "INSERT INTO line_restock_lrs (restock_id_lrs, product_id_lrs, quantity_lrs, unit_price_lrs) VALUES ($1, $2, $3, $4)",
                &[&restock_id, product_id, &quantity, &unit_price],
            ).await?;
            client.execute(
                "INSERT INTO productrestockprices_prr (product_ref_prr, buying_price_prr, restock_id_prr, restock_date_prr, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $4, $4) ON CONFLICT DO NOTHING",
                &[product_id, &unit_price, &restock_id, &restock_date],
            ).await?;
        }
    }

    // Groupe 4: Restocks saisonniers (15%)
    let seasonal_restocks = NB_RESTOCKS - regular_restocks - bulk_restocks - emergency_restocks;
    for _ in 0..seasonal_restocks {
        let days_offset = rng.gen_range(0..RESTOCK_DATE_RANGE_DAYS);
        let restock_date = restock_start + chrono::Duration::days(days_offset);
        let nb_products = rng.gen_range(1..=8);
        let selected = select_products(&mut rng, nb_products);

        let total_quantity: i32 = selected.iter().map(|_| rng.gen_range(10..=200)).sum();
        let row = client.query_one(
            "INSERT INTO restock_res (quantity_res, supplier_id_res, status_res, restock_date_res, created_at, updated_at)
             VALUES ($1, $2, $3, $4, NOW(), NOW()) RETURNING id_res",
            &[&total_quantity, &supplier_ids.choose(&mut rng), &"pending", &restock_date],
        ).await?;
        let restock_id: i32 = row.get(0);

        for (product_id, _) in &selected {
            let quantity = rng.gen_range(10..=200);
            let base_bp = *buying_price_map.get(product_id).expect("buying price");
            let variation = Decimal::from_i32(rng.gen_range(-5..=5)).unwrap() / Decimal::from_i32(100).unwrap();
            let unit_price = (base_bp * (Decimal::ONE + variation)).round_dp(2);

            client.execute(
                "INSERT INTO line_restock_lrs (restock_id_lrs, product_id_lrs, quantity_lrs, unit_price_lrs) VALUES ($1, $2, $3, $4)",
                &[&restock_id, product_id, &quantity, &unit_price],
            ).await?;
            client.execute(
                "INSERT INTO productrestockprices_prr (product_ref_prr, buying_price_prr, restock_id_prr, restock_date_prr, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $4, $4) ON CONFLICT DO NOTHING",
                &[product_id, &unit_price, &restock_id, &restock_date],
            ).await?;
        }
    }

    println!("      -> {} restocks (reguliers: {}, gros: {}, urgence: {}, saisonniers: {})",
        NB_RESTOCKS, regular_restocks, bulk_restocks, emergency_restocks, seasonal_restocks);

    // ── Fix restock created_at and assign realistic statuses per supplier tier ──
    client.execute(
        "UPDATE restock_res SET
            created_at = restock_date_res - (floor(random() * 10 + 2)::int || ' days')::interval,
            updated_at = restock_date_res",
        &[],
    ).await?;

    // Tiered supplier reliability for status assignment
    client.execute(
        "UPDATE restock_res SET status_res = (
            CASE
                WHEN restock_date_res < NOW() - interval '14 days' THEN
                    CASE
                        WHEN random() < (CASE
                            WHEN supplier_id_res <= 5  THEN 0.90
                            WHEN supplier_id_res <= 10 THEN 0.75
                            WHEN supplier_id_res <= 15 THEN 0.55
                            ELSE 0.35
                        END) THEN 'received'
                        WHEN random() < 0.4 THEN 'cancelled'
                        ELSE 'in_transit'
                    END
                WHEN restock_date_res < NOW() - interval '3 days' THEN
                    CASE
                        WHEN random() < 0.35 THEN 'received'
                        WHEN random() < 0.5  THEN 'in_transit'
                        ELSE 'pending'
                    END
                ELSE
                    CASE WHEN random() < 0.3 THEN 'in_transit' ELSE 'pending' END
            END
        )",
        &[],
    ).await?;

    // Log status distribution
    let status_rows = client.query(
        "SELECT status_res, COUNT(*) FROM restock_res GROUP BY status_res ORDER BY status_res",
        &[],
    ).await?;
    print!("      -> Statuts restocks: ");
    for row in &status_rows {
        let status: &str = row.get(0);
        let count: i64 = row.get(1);
        print!("{status}={count} ");
    }
    println!();

    // ════════════════════════════════════════════════════════════════════
    // 9. Recalibrate stock quantities based on actual demand
    //    (triggers have inflated stock via received restocks)
    // ════════════════════════════════════════════════════════════════════
    println!("[POST] Recalibration des stocks en fonction de la demande...");

    // Set stock = avg_daily_demand * random_days_of_supply
    // Distribution of days-of-supply:
    //   ~15% products: 2-7 days   (URGENT for forecaster)
    //   ~20%: 8-14 days           (HIGH)
    //   ~30%: 15-30 days          (MEDIUM)
    //   ~35%: 31-90 days          (LOW)
    client.execute(
        "WITH daily_demand AS (
            SELECT lo.product_id_lor AS pid,
                   GREATEST(SUM(lo.quantity_lor)::float / 730.0, 0.1) AS avg_daily
            FROM line_order_lor lo
            GROUP BY lo.product_id_lor
        ),
        stock_target AS (
            SELECT pid, avg_daily,
                   CASE
                       WHEN random() < 0.15 THEN floor(avg_daily * (2 + random() * 5))
                       WHEN random() < 0.35 THEN floor(avg_daily * (8 + random() * 6))
                       WHEN random() < 0.65 THEN floor(avg_daily * (15 + random() * 15))
                       ELSE floor(avg_daily * (31 + random() * 59))
                   END AS new_stock
            FROM daily_demand
        )
        UPDATE products_pro SET
            stock_quantity_pro = GREATEST(st.new_stock::int, 0)
        FROM stock_target st
        WHERE products_pro.id_pro = st.pid",
        &[],
    ).await?;

    // Also set a few products to 0 stock for out_of_stock scenarios
    client.execute(
        "UPDATE products_pro SET stock_quantity_pro = 0
         WHERE id_pro IN (SELECT id_pro FROM products_pro ORDER BY random() LIMIT 5)",
        &[],
    ).await?;

    // Log the new stock distribution
    let stock_stats = client.query_one(
        "SELECT
            ROUND(AVG(stock_quantity_pro)::numeric, 1) AS avg_stock,
            MIN(stock_quantity_pro) AS min_stock,
            MAX(stock_quantity_pro) AS max_stock
         FROM products_pro",
        &[],
    ).await?;
    let avg_s: Decimal = stock_stats.get(0);
    let min_s: i32 = stock_stats.get(1);
    let max_s: i32 = stock_stats.get(2);
    println!("      -> Stock recalibre: avg={avg_s}, min={min_s}, max={max_s}");

    println!("\nSeeding termine avec succes !");
    println!("Recap: {} users, {} suppliers, {} products, ~{} orders, {} restocks",
        NB_USERS + 2, NB_SUPPLIERS, NB_PRODUCTS, total_orders, NB_RESTOCKS);
    Ok(())
}
