CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Rôles
CREATE TABLE IF NOT EXISTS role_rol (
    id_rol SERIAL PRIMARY KEY,
    name_rol TEXT UNIQUE NOT NULL
);

-- Utilisateurs
CREATE TABLE IF NOT EXISTS users_usr (
    id_usr SERIAL PRIMARY KEY,
    email_usr TEXT UNIQUE NOT NULL,
    lastname_usr TEXT NOT NULL,
    firstname_usr TEXT NOT NULL,
    password_usr TEXT NOT NULL,
    phone_usr TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Association rôle <-> utilisateur
CREATE TABLE IF NOT EXISTS role_user_rus (
    id_role_rus INT NOT NULL REFERENCES role_rol(id_rol),
    id_user_rus INT NOT NULL REFERENCES users_usr(id_usr),
    PRIMARY KEY (id_role_rus, id_user_rus)
);

-- Fournisseurs
CREATE TABLE IF NOT EXISTS supplier_sup (
    id_sup SERIAL PRIMARY KEY,
    name_sup TEXT NOT NULL,
    email_sup TEXT,
    phone_sup TEXT,
    address_sup TEXT
);

-- Enum statut produit
DO $$ BEGIN
    CREATE TYPE product_status_enum AS ENUM (
        'in_stock', 'out_of_stock', 'ordered', 'discontinued'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- Produits
CREATE TABLE IF NOT EXISTS products_pro (
    id_pro SERIAL PRIMARY KEY,
    name_pro TEXT UNIQUE NOT NULL,
    category_pro TEXT,
    reference_pro TEXT,
    supplier_id_pro INT REFERENCES supplier_sup(id_sup),
    stock_quantity_pro INT NOT NULL DEFAULT 0,
    buying_price_pro DECIMAL(10,2) NOT NULL DEFAULT 0,
    status_pro product_status_enum NOT NULL DEFAULT 'in_stock',
    date_last_reassor_pro TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Prix de vente des produits (historique)
CREATE TABLE IF NOT EXISTS productprices_prp (
    id_prp SERIAL PRIMARY KEY,
    product_ref_prp INT NOT NULL REFERENCES products_pro(id_pro),
    price_prp DECIMAL(10,2) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Commandes
CREATE TABLE IF NOT EXISTS order_ord (
    id_ord SERIAL PRIMARY KEY,
    user_id_ord INT NOT NULL REFERENCES users_usr(id_usr),
    order_date_ord TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status_ord TEXT NOT NULL DEFAULT 'processing',
    amount_ord DECIMAL(10,2) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Lignes de commande
CREATE TABLE IF NOT EXISTS line_order_lor (
    id_lor SERIAL PRIMARY KEY,
    order_id_lor INT NOT NULL REFERENCES order_ord(id_ord) ON DELETE CASCADE,
    product_id_lor INT NOT NULL REFERENCES products_pro(id_pro),
    quantity_lor INT NOT NULL DEFAULT 1,
    unit_price_lor DECIMAL(10,2) NOT NULL,
    line_total_lor DECIMAL(10,2) NOT NULL
);

-- Réapprovisionnements
CREATE TABLE IF NOT EXISTS restock_res (
    id_res SERIAL PRIMARY KEY,
    quantity_res INT NOT NULL,
    supplier_id_res INT REFERENCES supplier_sup(id_sup),
    status_res TEXT NOT NULL DEFAULT 'pending',
    restock_date_res TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Lignes de réapprovisionnement
CREATE TABLE IF NOT EXISTS line_restock_lrs (
    id_lrs SERIAL PRIMARY KEY,
    restock_id_lrs INT NOT NULL REFERENCES restock_res(id_res) ON DELETE CASCADE,
    product_id_lrs INT NOT NULL REFERENCES products_pro(id_pro),
    quantity_lrs INT NOT NULL DEFAULT 1,
    unit_price_lrs DECIMAL(10,2) NOT NULL
);

-- Prix d'achat liés aux réapprovisionnements (historique)
CREATE TABLE IF NOT EXISTS productrestockprices_prr (
    id_prr SERIAL PRIMARY KEY,
    product_ref_prr INT NOT NULL REFERENCES products_pro(id_pro),
    buying_price_prr DECIMAL(10,2) NOT NULL,
    restock_id_prr INT REFERENCES restock_res(id_res),
    restock_date_prr TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
