from datetime import datetime, timedelta
import psycopg2 
import os
import csv
import secrets
from faker import Faker
from unidecode import unidecode
import numpy as np
import random
import requests

# Initialisation des variables globales
nb_user = 111
nb_supplier = 18
nb_role = 6
nb_products = 118

date = datetime.now()
faker = Faker('fr_FR')

# Fonction de création du numéro de fidelité
def fidelity_number():
    return secrets.randbelow(90000000) + 10000000

# Fonction de création du numéro de téléphone
def phone_number():
    return f"+33{secrets.choice([6,7])}{secrets.randbelow(100000000):08d}"

# Scrpit seeder users
def user_seeder(cursor, nb_iteration: int, sql_filename: str) -> None:
    print(f'Initialisation du processus de sérialisation vers : {sql_filename}')

    # Assure que le dossier parent du fichier SQL existe
    parent_dir = os.path.dirname(sql_filename)
    if parent_dir:
        os.makedirs(parent_dir, exist_ok=True)

    try:
        import bcrypt
        has_bcrypt = True
    except ImportError:
        print("Attention : le package 'bcrypt' n'est pas installé. Les mots de passe seront insérés en clair dans la base de données.")
        has_bcrypt = False

    domains = ["gmail.com", "outlook.fr", "yahoo.fr", "protonmail.com", "orange.fr", "laposte.net"]

    with open(sql_filename, mode='w', encoding='utf-8') as sql_file:
        sql_file.write("-- Seed utilisateurs\n\n")

        for i in range(nb_iteration):
            prenom = faker.first_name()
            nom = faker.last_name()

            email = f"{unidecode(prenom).lower().replace(' ', '')}.{unidecode(nom).lower().replace(' ', '')}@{domains[i%6]}"
            mdp = faker.password().replace("'", "''")
            tel = phone_number()
            fid = f"FID-{fidelity_number()}"

            if has_bcrypt:
                hashed_mdp = bcrypt.hashpw(mdp.encode('utf-8'), bcrypt.gensalt(10)).decode('utf-8')
            else:
                hashed_mdp = mdp

            # SQL
            sql_file.write(
                f"INSERT INTO users_usr (email_usr, lastname_usr, firstname_usr, password_usr, phone_usr, fidelity_code_usr) VALUES "
                f"('{email}', '{nom.replace(chr(39), chr(39)+chr(39))}', '{prenom.replace(chr(39), chr(39)+chr(39))}', '{hashed_mdp}', '{tel}', '{fid}');\n"
            )

            # Insertion directe dans la base de données
            try:
                cursor.execute(
                    """
                    INSERT INTO users_usr (email_usr, lastname_usr, firstname_usr, password_usr, phone_usr, fidelity_code_usr)
                    VALUES (%s, %s, %s, %s, %s, %s)
                    ON CONFLICT (email_usr) DO NOTHING;
                    """,
                    (email, nom, prenom, hashed_mdp, tel, fid)
                )
            except Exception as e:
                print(f"Erreur d'insertion dans la base de données pour {email} : {e}")

    print('Opération I/O et BDD (utilisateurs) terminée avec succès.')


# Script seeder suppliers
def supplier_seeder(cursor, nb_iteration: int, filename: str) -> None:
    print(f'Initialisation du processus de sérialisation vers le fichier : {filename}')

    # Assure que le dossier parent existe
    parent_dir = os.path.dirname(filename)
    if parent_dir:
        os.makedirs(parent_dir, exist_ok=True)

    seen_names = set()
    with open(filename, mode='w', encoding='utf-8') as sql_file:
        sql_file.write("-- Seed Supplier\n\n")

        for i in range(nb_iteration):
            nom = faker.company()
            while nom in seen_names:
                nom = faker.company()
            seen_names.add(nom)
            
            email = f"{nom.lower().replace(' ', '').replace('.','').replace(chr(39), '')}@example.com"
            tel = phone_number()
            adress = faker.address()

            # SQL avec les bons noms de tables et de colonnes de la BDD
            sql_file.write(
                f"INSERT INTO supplier_sup (id_sup, name_sup, email_sup, phone_sup, address_sup) VALUES "
                f"({i + 1}, '{nom.replace(chr(39), chr(39)+chr(39))}', '{email}', '{tel}', '{adress.replace(chr(39), chr(39)+chr(39))}');\n"
            )

            # Insertion directe dans la base de données
            try:
                cursor.execute(
                    """
                    INSERT INTO supplier_sup (id_sup, name_sup, email_sup, phone_sup, address_sup)
                    VALUES (%s, %s, %s, %s, %s)
                    ON CONFLICT (id_sup) DO NOTHING;
                    """,
                    (i + 1, nom, email, tel, adress)
                )
            except Exception as e:
                print(f"Erreur d'insertion dans la base de données pour le fournisseur {nom} : {e}")

    print('Opération I/O et BDD (fournisseurs) terminée avec succès.')


# Script création role user
def role_usr_seeder(cursor, filename: str) -> None:
    print(f'Initialisation du processus de sérialisation vers le fichier : {filename}')

    # Assure que le dossier parent existe
    parent_dir = os.path.dirname(filename)
    if parent_dir:
        os.makedirs(parent_dir, exist_ok=True)

    # Récupérer les vrais IDs d'utilisateurs insérés depuis la base de données
    cursor.execute("SELECT id_usr FROM users_usr ORDER BY id_usr;")
    user_ids = [row[0] for row in cursor.fetchall()]

    if not user_ids:
        print("Aucun utilisateur trouvé dans la base de données. Impossible d'associer des rôles.")
        return

    # Récupération propre de la date courante pour les colonnes de audit
    created_at_str = date.strftime('%Y-%m-%d %H:%M:%S')

    with open(filename, mode='w', encoding='utf-8') as sql_file:
        sql_file.write("-- Seed Role User\n\n")

        for id_usr in user_ids:
            # Utilisation de np.random.choice pour récupérer un scalaire propre
            id_rol = int(np.random.choice(5, p=[0, 0.02, 0.02, 0.02, 0.94])) 

            # SQL avec les bons noms de tables et colonnes
            sql_file.write(
                f"INSERT INTO role_user_rus (id_role_rus, id_user_rus, created_at, updated_at) VALUES "
                f"('{id_rol}', '{id_usr}', '{created_at_str}', '{created_at_str}');\n"
            )

            # Insertion directe dans la base de données
            try:
                cursor.execute(
                    """
                    INSERT INTO role_user_rus (id_role_rus, id_user_rus, created_at, updated_at)
                    VALUES (%s, %s, %s, %s)
                    ON CONFLICT (id_role_rus, id_user_rus) DO NOTHING;
                    """,
                    (id_rol, id_usr, date, date)
                )
            except Exception as e:
                print(f"Erreur d'insertion dans la base de données pour l'association rôle {id_rol} - user {id_usr} : {e}")

    print('Opération I/O et BDD (rôles) terminée avec succès.')


# Script création roles
def role_seeder(cursor, filename: str) -> None:
    print(f'Initialisation du processus de sérialisation vers le fichier : {filename}')

    # Assure que le dossier parent existe
    parent_dir = os.path.dirname(filename)
    if parent_dir:
        os.makedirs(parent_dir, exist_ok=True)

    roles = ["admin", "manager", "seller", "viewer", "customer"]

    with open(filename, mode='w', encoding='utf-8') as sql_file:
        sql_file.write("-- Seed Roles\n\n")

        for i, role in enumerate(roles):
            # SQL avec les bons noms de tables et colonnes
            sql_file.write(
                f"INSERT INTO role_rol (id_rol, name_rol) VALUES "
                f"({i}, '{role}');\n"
            )

            # Insertion directe dans la base de données
            try:
                cursor.execute(
                    """
                    INSERT INTO role_rol (id_rol, name_rol)
                    VALUES (%s, %s)
                    ON CONFLICT (id_rol) DO UPDATE SET name_rol = EXCLUDED.name_rol;
                    """,
                    (i, role)
                )
            except Exception as e:
                print(f"Erreur d'insertion dans la base de données pour le rôle {role} : {e}")

    print('Opération I/O et BDD (définition des rôles) terminée avec succès.')


HEADERS = {
    "User-Agent": "MyStoreSeeder/1.0 (contact@mystore.com)"
}

CATEGORIES = [
    "cereals", "dairy", "beverages", "snacks", "fruits",
    "vegetables", "meat", "fish", "pasta", "sauces",
]

# Fourchettes de prix d'achat réalistes par catégorie (fallback si OFF ne fournit pas de prix)
CATEGORY_PRICE_RANGES = {
    "cereals":    (0.80, 4.00),
    "dairy":      (0.50, 5.00),
    "beverages":  (0.30, 3.00),
    "snacks":     (0.80, 5.00),
    "fruits":     (0.50, 4.00),
    "vegetables": (0.40, 3.50),
    "meat":       (3.00, 18.00),
    "fish":       (2.50, 15.00),
    "pasta":      (0.60, 3.50),
    "sauces":     (0.80, 4.50),
}

# Fonction pour avoir une date aléatoire des 2 derniers mois
def random_date_last_2_months() -> str:
    today = datetime.today()
    days_ago = random.randint(0, 60)
    random_date = today - timedelta(days=days_ago)
    return random_date.strftime("%Y-%m-%d")


# Script création produits
def fetch_products_from_off(nb: int) -> list[dict]:
    products = []
    seen_barcodes = set()
    category_index = 0
    page = 1
    product_id_counter = 1

    while len(products) < nb:
        category = CATEGORIES[category_index % len(CATEGORIES)]
        url = (
            f"https://fr.openfoodfacts.org/cgi/search.pl"
            f"?action=process"
            f"&tagtype_0=categories&tag_contains_0=contains&tag_0={category}"
            f"&tagtype_1=countries&tag_contains_1=contains&tag_1=france"
            f"&fields=code,product_name_fr,product_name,categories_tags,price_100g,price"
            f"&json=1&page_size=50&page={page}"
        )

        try:
            response = requests.get(url, headers=HEADERS, timeout=10)
            response.raise_for_status()
            data = response.json()
        except Exception:
            category_index += 1
            page = 1
            continue

        raw_products = data.get("products", [])
        if not raw_products:
            category_index += 1
            page = 1
            continue

        for p in raw_products:
            if len(products) >= nb:
                break

            barcode = p.get("code", "").strip()
            # On prend en priorité le nom français de l'article
            name = p.get("product_name_fr", "").strip() or p.get("product_name", "").strip()
            if not barcode or not name or barcode in seen_barcodes or name.isdigit():
                continue

            cats = p.get("categories_tags", [])
            category_clean = None
            
            # Chercher d'abord la catégorie en français (fr:)
            for c in cats:
                if c.startswith("fr:"):
                    category_clean = c.replace("fr:", "").replace("-", " ").title()
                    break
            
            # Repli sur l'anglais (en:) si non trouvée en français
            if not category_clean:
                for c in cats:
                    if c.startswith("en:"):
                        category_clean = c.replace("en:", "").replace("-", " ").title()
                        break
            
            # Repli par défaut sur la catégorie recherchée
            if not category_clean:
                category_clean = category.replace("-", " ").title()

            seen_barcodes.add(barcode)

            # Prix depuis OFF (champ price ou price_100g * quantité estimée)
            off_price = None
            raw_price = p.get("price")
            if raw_price:
                try:
                    off_price = round(float(raw_price), 2)
                except (ValueError, TypeError):
                    pass
            if off_price is None:
                raw_price_100g = p.get("price_100g")
                if raw_price_100g:
                    try:
                        # Estimation : produit moyen de 400g
                        off_price = round(float(raw_price_100g) * 4, 2)
                    except (ValueError, TypeError):
                        pass
            if off_price is None or off_price <= 0 or off_price > 100:
                # Fallback : fourchette réaliste par catégorie
                low, high = CATEGORY_PRICE_RANGES.get(category, (0.5, 10.0))
                off_price = round(random.uniform(low, high), 2)

            # Détermination du statut en fonction du stock
            stock_qty = random.randint(4, 68)
            status = 'in_stock' if stock_qty > 0 else 'out_of_stock'

            products.append({
                "id_pro": product_id_counter,
                "name_pro": name[:255],
                "category_pro": category_clean[:100],
                "reference_pro": barcode[:50],
                "supplier_id_pro": random.randint(1, 10),
                "stock_quantity_pro": stock_qty,
                "buying_price_pro": off_price,
                "status_pro": status,
                "date_last_reassor_pro": random_date_last_2_months(),
                "created_at": "2024-01-01",
            })
            product_id_counter += 1

        print(f"  -> {len(products)}/{nb} produits collectés (catégorie: {category}, page: {page})")
        page += 1
        if page > 3:
            category_index += 1
            page = 1

    print(f"[OK] {len(products)} produits récupérés.")
    return products

# Formatage des données pour enlever les apostrophes dans les nom
def escape_sql(value: str) -> str:
    return value.replace("'", "''")

# Script de création des produits
def product_seeder(cursor, nb_iteration: int, filename: str) -> None:
    print(f'Initialisation du processus de sérialisation des produits vers : {filename}')

    # Assure que le dossier parent existe
    parent_dir = os.path.dirname(filename)
    if parent_dir:
        os.makedirs(parent_dir, exist_ok=True)

    products = fetch_products_from_off(nb_iteration)

    lines = [
        "-- ============================================================",
        f"-- Seed produits — généré le {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        f"-- {len(products)} produits",
        "-- ============================================================",
        "",
    ]

    for p in products:
        line = (
            f"INSERT INTO products_pro (id_pro, name_pro, category_pro, reference_pro, supplier_id_pro, stock_quantity_pro, buying_price_pro, status_pro, date_last_reassor_pro, created_at) VALUES ("
            f"{p['id_pro']}, "
            f"'{escape_sql(p['name_pro'])}', "
            f"'{escape_sql(p['category_pro'])}', "
            f"'{escape_sql(p['reference_pro'])}', "
            f"{p['supplier_id_pro']}, "
            f"{p['stock_quantity_pro']}, "
            f"{p['buying_price_pro']}, "
            f"'{p['status_pro']}'::product_status_enum, "
            f"'{p['date_last_reassor_pro']}', "
            f"'{p['created_at']}'"
            f");"
        )
        lines.append(line)

        # Insertion directe dans la base de données
        try:
            cursor.execute(
                """
                INSERT INTO products_pro (id_pro, name_pro, category_pro, reference_pro, supplier_id_pro, stock_quantity_pro, buying_price_pro, status_pro, date_last_reassor_pro, created_at)
                VALUES (%s, %s, %s, %s, %s, %s, %s, %s::product_status_enum, %s, %s)
                ON CONFLICT (name_pro) DO NOTHING;
                """,
                (
                    p["id_pro"],
                    p["name_pro"],
                    p["category_pro"],
                    p["reference_pro"],
                    p["supplier_id_pro"],
                    p["stock_quantity_pro"],
                    p["buying_price_pro"],
                    p["status_pro"],
                    p["date_last_reassor_pro"],
                    p["created_at"]
                )
            )
        except Exception as e:
            print(f"Erreur d'insertion dans la base de données pour le produit {p['name_pro']} : {e}")

    with open(filename, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")

    print(f"Opération I/O et BDD (produits) terminée avec succès. ({len(products)} lignes)")


# Script de création de la variation des prix des produits (sur 2 ans)
def random_date_last_2_years() -> datetime:
    today = datetime.today()
    days_ago = random.randint(0, 365 * 2)
    return today - timedelta(days=days_ago)


def generate_price_history(cursor, output_sql: str) -> None:
    print("Récupération des produits depuis la base de données...")
    cursor.execute("SELECT id_pro, buying_price_pro FROM products_pro;")
    products = cursor.fetchall()

    print(f"{len(products)} produits trouvés. Génération de l'historique des prix...")

    lines = []
    lines.append("-- ============================================================")
    lines.append(f"-- Seed historique des prix — généré le {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    lines.append("-- ============================================================")
    lines.append("")

    record_id = 1
    created_at = "2024-01-01 00:00:00"

    for product_id, buying_price_pro in products:
        buying_price = float(buying_price_pro)

        # Nombre de variations de prix
        nb_variations = random.randint(8, 15)

        # Dates aléatoires ordonnées pour remonter le temps
        dates = sorted(
            [random_date_last_2_years() for _ in range(nb_variations)],
            reverse=True
        )

        # On applique un coefficient pour le prix de vente initial (ex: marge de 15% à 40%)
        margin = random.uniform(0.15, 0.40)
        current_price = round(buying_price * (1.0 + margin), 2)

        for date in dates:
            direction = random.choice([1, -1])
            variation = round(current_price * 0.20, 2)
            old_price = round(current_price + (direction * variation), 2)
            
            date_str = date.strftime('%Y-%m-%d %H:%M:%S')

            lines.append(
                f"INSERT INTO productprices_prp (product_ref_prp, price_prp, created_at, updated_at) VALUES ("
                f"{product_id}, "
                f"{old_price}, "
                f"'{date_str}', "
                f"'{date_str}'"
                f");"
            )

            # Insertion directe en base de données
            try:
                cursor.execute(
                    """
                    INSERT INTO productprices_prp (product_ref_prp, price_prp, created_at, updated_at)
                    VALUES (%s, %s, %s, %s);
                    """,
                    (product_id, old_price, date, date)
                )
            except Exception as e:
                print(f"Erreur d'insertion dans la base de données pour le prix historique du produit {product_id} : {e}")

            record_id += 1
            current_price = old_price

    # Assure que le dossier parent du fichier SQL existe
    parent_dir = os.path.dirname(output_sql)
    if parent_dir:
        os.makedirs(parent_dir, exist_ok=True)

    with open(output_sql, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")

    print(f"[OK] Fichier SQL généré : {output_sql} ({record_id - 1} lignes insérées)")




# ============================================================
# Distribution clients par segment pour les commandes
# ============================================================
CLIENT_SEGMENTS = {
    "ponctuel": {"ratio": 0.60, "commandes_range": (1, 3)},
    "regulier": {"ratio": 0.30, "commandes_range": (4, 8)},
    "fidele":   {"ratio": 0.10, "commandes_range": (9, 12)},
}

ORDER_STATUSES = ["processing", "shipped", "delivered", "cancelled"]
ORDER_STATUS_WEIGHTS = [15, 25, 50, 10]


def realistic_order_date() -> datetime:
    """Génère une date réaliste sur les 2 dernières années."""
    today = datetime.today()
    # Distribution exponentielle : concentre les dates récentes
    days_ago = int(random.expovariate(1 / 365))
    days_ago = min(days_ago, 730)
    days_ago = max(days_ago, 1)
    d = today - timedelta(days=days_ago)
    return d.replace(
        hour=random.randint(6, 22),
        minute=random.randint(0, 59),
        second=random.randint(0, 59),
        microsecond=0
    )


def assign_segments(user_ids: list[int]) -> dict:
    """Assigne chaque utilisateur à un segment et génère ses dates de commandes."""
    user_segments = {}
    shuffled = user_ids[:]
    random.shuffle(shuffled)

    idx = 0
    for segment_name, seg_info in CLIENT_SEGMENTS.items():
        nb_clients = max(1, int(len(shuffled) * seg_info["ratio"]))
        min_cmd, max_cmd = seg_info["commandes_range"]

        for _ in range(nb_clients):
            if idx >= len(shuffled):
                break
            uid = shuffled[idx]
            idx += 1

            # Générer les dates sur 24 mois
            dates = []
            for _ in range(24):
                for _ in range(random.randint(min_cmd, max_cmd)):
                    dates.append(realistic_order_date())

            user_segments[uid] = {
                "segment": segment_name,
                "dates": sorted(dates),
            }

    return user_segments


def generate_orders(cursor, output_sql: str) -> None:
    """Génère les commandes et lignes de commandes depuis la BDD et écrit le fichier SQL."""
    print("Recuperation des produits et utilisateurs depuis la base de donnees...")

    cursor.execute("SELECT id_pro, buying_price_pro FROM products_pro ORDER BY id_pro;")
    products = cursor.fetchall()  # list of (id_pro, buying_price_pro)

    cursor.execute("SELECT id_usr FROM users_usr ORDER BY id_usr;")
    user_ids = [row[0] for row in cursor.fetchall()]

    if not products or not user_ids:
        print("Aucun produit ou utilisateur trouve en base de donnees. Annulation.")
        return

    print(f"{len(products)} produits et {len(user_ids)} utilisateurs trouves.")

    user_segments = assign_segments(user_ids)

    lines = [
        "-- ============================================================",
        f"-- Seed commandes — généré le {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        "-- ============================================================",
        "",
    ]

    total_orders = 0
    total_lines = 0

    for uid, seg_data in sorted(user_segments.items()):
        for order_date in seg_data["dates"]:
            # Choisir un nombre réaliste de lignes (1-8 produits par commande)
            nb_lines = random.choices(
                [1, 2, 3, 4, 5, 6, 7, 8],
                weights=[25, 30, 20, 12, 7, 3, 2, 1]
            )[0]

            selected = random.sample(products, min(nb_lines, len(products)))
            status = random.choices(ORDER_STATUSES, weights=ORDER_STATUS_WEIGHTS)[0]
            amount_total = 0.0
            order_date_str = order_date.strftime("%Y-%m-%d %H:%M:%S")

            # Insérer la commande et récupérer son id
            try:
                cursor.execute(
                    """
                    INSERT INTO order_ord (user_id_ord, order_date_ord, status_ord, amount_ord)
                    VALUES (%s, %s, %s, %s)
                    RETURNING id_ord;
                    """,
                    (uid, order_date, status, 0.0)
                )
                order_id = cursor.fetchone()[0]
            except Exception as e:
                print(f"Erreur insertion commande user {uid} : {e}")
                continue

            order_lines_sql = []

            for (product_id, buying_price_raw) in selected:
                buying_price = float(buying_price_raw)
                # Prix de vente = prix d'achat + marge aléatoire 15-40%
                unit_price = round(buying_price * random.uniform(1.15, 1.40), 2)
                quantity = random.randint(1, 5)
                line_total = round(unit_price * quantity, 2)
                amount_total += line_total

                order_lines_sql.append(
                    f"INSERT INTO line_order_lor "
                    f"(order_id_lor, product_id_lor, quantity_lor, unit_price_lor, line_total_lor) "
                    f"VALUES ({order_id}, {product_id}, {quantity}, {unit_price}, {line_total});"
                )

                try:
                    cursor.execute(
                        """
                        INSERT INTO line_order_lor
                          (order_id_lor, product_id_lor, quantity_lor, unit_price_lor, line_total_lor)
                        VALUES (%s, %s, %s, %s, %s);
                        """,
                        (order_id, product_id, quantity, unit_price, line_total)
                    )
                    total_lines += 1
                except Exception as e:
                    print(f"Erreur insertion ligne commande {order_id} / produit {product_id} : {e}")

            # Mettre à jour le montant total de la commande
            amount_total = round(amount_total, 2)
            try:
                cursor.execute(
                    "UPDATE order_ord SET amount_ord = %s WHERE id_ord = %s;",
                    (amount_total, order_id)
                )
            except Exception as e:
                print(f"Erreur mise a jour montant commande {order_id} : {e}")

            # Construire les lignes SQL pour le fichier
            lines.append(
                f"INSERT INTO order_ord (id_ord, user_id_ord, order_date_ord, status_ord, amount_ord) "
                f"VALUES ({order_id}, {uid}, '{order_date_str}', '{status}', {amount_total});"
            )
            lines.extend(order_lines_sql)
            lines.append("")

            total_orders += 1

    # Écrire le fichier SQL
    parent_dir = os.path.dirname(output_sql)
    if parent_dir:
        os.makedirs(parent_dir, exist_ok=True)

    with open(output_sql, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")

    print(f"[OK] Fichier SQL genere : {output_sql}")
    print(f"     {total_orders} commandes, {total_lines} lignes de commande insérées.")


# Initialisation des données
if __name__ == "__main__":
    print("Connexion à la base de données PostgreSQL...")
    try:
        db_url = os.environ.get("DATABASE_URL", "postgresql://user:pass@127.0.0.1:5432/stocks")
        if db_url.startswith("postgres://"):
            db_url = db_url.replace("postgres://", "postgresql://", 1)
        conn = psycopg2.connect(db_url)
        conn.autocommit = True
        cursor = conn.cursor()

        # Nettoyage de la base de données pour repartir sur une base propre
        print("Nettoyage des tables existantes...")
        cursor.execute(
            """
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
            """
        )

        # On insère les rôles
        role_seeder(cursor, 'seeds/seed_roles.sql')

        # On insère les fournisseurs
        supplier_seeder(cursor, nb_supplier, 'seeds/seed_suppliers.sql')

        # On insère les produits (qui dépendent des fournisseurs)
        product_seeder(cursor, nb_products, 'seeds/seed_products.sql')

        # On insère les utilisateurs
        user_seeder(cursor, nb_user, 'seeds/seed_users.sql')

        # On insère les associations de rôles aux utilisateurs
        role_usr_seeder(cursor, 'seeds/seed_role_usr.sql')

        # On insère les variations de prix des produits
        generate_price_history(cursor, 'seeds/seed_price_history.sql')

        # On insère les commandes et lignes de commandes
        generate_orders(cursor, 'seeds/seed_orders.sql')

        conn.commit()
        cursor.close()
        conn.close()
        print("Toutes les données ont été insérées et enregistrées avec succès dans la base de données.")
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(f"Une erreur générale est survenue : {e}")
