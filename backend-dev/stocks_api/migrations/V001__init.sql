-- Enable UUIDs or extensions si besoin
-- CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 1. rôle table -----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS role_rol (
    id_rol     SERIAL PRIMARY KEY,
    name_rol   VARCHAR NOT NULL UNIQUE, -- ✅ Ajout UNIQUE pour supporter ON CONFLICT
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- 2. users table ----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS users_usr (
    id_usr        SERIAL PRIMARY KEY,
    email_usr     VARCHAR NOT NULL UNIQUE, -- ✅ déjà unique
    lastname_usr  VARCHAR NOT NULL,
    firstname_usr VARCHAR NOT NULL,
    password_usr  VARCHAR NOT NULL,
    phone_usr     VARCHAR,
    status_usr    VARCHAR DEFAULT 'active',
    created_at    TIMESTAMP DEFAULT NOW(),
    updated_at    TIMESTAMP DEFAULT NOW()
);

-- 3. role_user link table --------------------------------------------------------
CREATE TABLE IF NOT EXISTS role_user_rus (
    id_role_rus INTEGER NOT NULL,
    id_user_rus INTEGER NOT NULL,
    created_at  TIMESTAMP DEFAULT NOW(),
    updated_at  TIMESTAMP DEFAULT NOW(),
    PRIMARY KEY (id_role_rus, id_user_rus),
    FOREIGN KEY (id_role_rus) REFERENCES role_rol(id_rol) ON DELETE CASCADE,
    FOREIGN KEY (id_user_rus) REFERENCES users_usr(id_usr) ON DELETE CASCADE
);

-- 4. supplier table --------------------------------------------------------------
CREATE TABLE IF NOT EXISTS supplier_sup (
    id_sup    SERIAL PRIMARY KEY,
    name_sup  VARCHAR NOT NULL UNIQUE,  -- ✅ Fournisseur unique par nom
    email_sup VARCHAR NOT NULL UNIQUE,  -- ✅ Fournisseur unique par email
    phone_sup VARCHAR NOT NULL,
    address_sup VARCHAR NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- 5. product table ---------------------------------------------------------------
CREATE TABLE IF NOT EXISTS products_pro (
    id_pro                 SERIAL PRIMARY KEY,
    name_pro               VARCHAR NOT NULL UNIQUE, -- ✅ déjà unique
    category_pro           VARCHAR NOT NULL,
    reference_pro          VARCHAR NOT NULL UNIQUE, -- ✅ ajout UNIQUE sur la référence produit
    supplier_id_pro        INTEGER NOT NULL,
    stock_quantity_pro     INTEGER NOT NULL,
    buying_price_pro       NUMERIC NOT NULL,
    date_last_reassor_pro  TIMESTAMP NOT NULL,
    created_at             TIMESTAMP DEFAULT NOW(),
    updated_at             TIMESTAMP DEFAULT NOW(),
    FOREIGN KEY (supplier_id_pro) REFERENCES supplier_sup(id_sup) ON DELETE RESTRICT
);

-- 6. product prices table --------------------------------------------------------
CREATE TABLE IF NOT EXISTS productprices_prp (  
    id_prp          SERIAL PRIMARY KEY,
    product_ref_prp INTEGER NOT NULL UNIQUE, 
    price_prp       NUMERIC NOT NULL,
    created_at      TIMESTAMP DEFAULT NOW(),
    updated_at      TIMESTAMP DEFAULT NOW(),
    FOREIGN KEY (product_ref_prp) REFERENCES products_pro(id_pro) ON DELETE CASCADE
);

-- 7. orders table ----------------------------------------------------------------
CREATE TABLE IF NOT EXISTS order_ord (
    id_ord         SERIAL PRIMARY KEY,
    user_id_ord    INTEGER NOT NULL,
    order_date_ord TIMESTAMP NOT NULL,
    status_ord     VARCHAR NOT NULL,
    amount_ord     NUMERIC NOT NULL,
    created_at     TIMESTAMP DEFAULT NOW(),
    updated_at     TIMESTAMP DEFAULT NOW(),
    FOREIGN KEY (user_id_ord) REFERENCES users_usr(id_usr) ON DELETE RESTRICT
);

-- 8. line items table ------------------------------------------------------------
CREATE TABLE IF NOT EXISTS line_order_lor (
    id_lor         SERIAL PRIMARY KEY,
    order_id_lor   INTEGER NOT NULL,
    product_id_lor INTEGER NOT NULL,
    quantity_lor   INTEGER NOT NULL,
    line_total_lor NUMERIC NOT NULL,
    created_at     TIMESTAMP DEFAULT NOW(),
    updated_at     TIMESTAMP DEFAULT NOW(),
    FOREIGN KEY (order_id_lor)   REFERENCES order_ord(id_ord)   ON DELETE CASCADE,
    FOREIGN KEY (product_id_lor) REFERENCES products_pro(id_pro) ON DELETE RESTRICT
);