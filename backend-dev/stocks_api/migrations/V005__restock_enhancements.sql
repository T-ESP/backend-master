-- Migration V005: Améliorations du suivi des prix pour les restocks et les commandes

-- 1. Créer la table line_restock pour suivre les détails de chaque produit dans un restock
CREATE TABLE IF NOT EXISTS line_restock_lrs (
    id_lrs              SERIAL PRIMARY KEY,
    restock_id_lrs      INTEGER NOT NULL,
    product_id_lrs      INTEGER NOT NULL,
    quantity_lrs        INTEGER NOT NULL CHECK (quantity_lrs > 0),
    unit_price_lrs      NUMERIC NOT NULL CHECK (unit_price_lrs >= 0),
    total_price_lrs     NUMERIC NOT NULL CHECK (total_price_lrs >= 0),
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (restock_id_lrs) REFERENCES restock_res(id_res) ON DELETE CASCADE,
    FOREIGN KEY (product_id_lrs) REFERENCES products_pro(id_pro) ON DELETE RESTRICT
);

-- 2. Ajouter les colonnes supplier_id et status à la table restock_res
ALTER TABLE restock_res
ADD COLUMN IF NOT EXISTS supplier_id_res INTEGER,
ADD COLUMN IF NOT EXISTS status_res VARCHAR(50) DEFAULT 'pending' CHECK (status_res IN ('pending', 'in_transit', 'received', 'cancelled')),
ADD CONSTRAINT fk_restock_supplier FOREIGN KEY (supplier_id_res) REFERENCES supplier_sup(id_sup) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_restock_supplier ON restock_res(supplier_id_res);
CREATE INDEX IF NOT EXISTS idx_restock_status ON restock_res(status_res);

COMMENT ON COLUMN restock_res.supplier_id_res IS 'Fournisseur principal du réapprovisionnement';
COMMENT ON COLUMN restock_res.status_res IS 'Statut du réapprovisionnement: pending (en attente), in_transit (en cours), received (reçu), cancelled (annulé)';

-- 3. Modifier line_order pour ajouter le prix unitaire et le prix total
ALTER TABLE line_order_lor
ADD COLUMN IF NOT EXISTS unit_price_lor NUMERIC CHECK (unit_price_lor >= 0);

-- Renommer line_total_lor si nécessaire pour clarifier (déjà présent, on garde tel quel)
COMMENT ON COLUMN line_order_lor.line_total_lor IS 'Prix total de la ligne (unit_price_lor * quantity_lor)';
COMMENT ON COLUMN line_order_lor.unit_price_lor IS 'Prix unitaire du produit au moment de la commande';

-- 4. Créer la table productrestockprices_prr pour suivre l'historique des prix d'achat lors des restocks
CREATE TABLE IF NOT EXISTS productrestockprices_prr (
    id_prr              SERIAL PRIMARY KEY,
    product_ref_prr     INTEGER NOT NULL,
    buying_price_prr    NUMERIC NOT NULL CHECK (buying_price_prr >= 0),
    restock_id_prr      INTEGER NOT NULL,
    restock_date_prr    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at          TIMESTAMPTZ DEFAULT NOW(),
    updated_at          TIMESTAMPTZ DEFAULT NOW(),
    FOREIGN KEY (product_ref_prr) REFERENCES products_pro(id_pro) ON DELETE CASCADE,
    FOREIGN KEY (restock_id_prr) REFERENCES restock_res(id_res) ON DELETE CASCADE
);

-- 5. Index pour améliorer les performances
CREATE INDEX IF NOT EXISTS idx_line_restock_restock_id ON line_restock_lrs(restock_id_lrs);
CREATE INDEX IF NOT EXISTS idx_line_restock_product_id ON line_restock_lrs(product_id_lrs);
CREATE INDEX IF NOT EXISTS idx_productrestockprices_product ON productrestockprices_prr(product_ref_prr);
CREATE INDEX IF NOT EXISTS idx_productrestockprices_restock ON productrestockprices_prr(restock_id_prr);
CREATE INDEX IF NOT EXISTS idx_productrestockprices_date ON productrestockprices_prr(restock_date_prr DESC);

-- 6. Trigger pour calculer automatiquement le total_price_lrs dans line_restock
CREATE OR REPLACE FUNCTION trg_calculate_line_restock_total()
RETURNS TRIGGER AS $$
BEGIN
    NEW.total_price_lrs := NEW.quantity_lrs * NEW.unit_price_lrs;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_line_restock_total ON line_restock_lrs;

CREATE TRIGGER trg_line_restock_total
BEFORE INSERT OR UPDATE OF quantity_lrs, unit_price_lrs ON line_restock_lrs
FOR EACH ROW
EXECUTE FUNCTION trg_calculate_line_restock_total();

-- 7. Trigger pour calculer automatiquement le line_total_lor dans line_order
CREATE OR REPLACE FUNCTION trg_calculate_line_order_total()
RETURNS TRIGGER AS $$
BEGIN
    -- Si unit_price_lor est défini, calculer le total
    IF NEW.unit_price_lor IS NOT NULL THEN
        NEW.line_total_lor := NEW.quantity_lor * NEW.unit_price_lor;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_line_order_total ON line_order_lor;

CREATE TRIGGER trg_line_order_total
BEFORE INSERT OR UPDATE OF quantity_lor, unit_price_lor ON line_order_lor
FOR EACH ROW
EXECUTE FUNCTION trg_calculate_line_order_total();

-- 8. Trigger pour enregistrer automatiquement le prix d'achat dans l'historique lors d'un restock
-- NOTE: Ce trigger est désactivé pour permettre au seeder d'insérer des dates historiques variées
-- Pour l'utiliser en production, décommentez le CREATE TRIGGER ci-dessous
CREATE OR REPLACE FUNCTION trg_record_restock_price_history()
RETURNS TRIGGER AS $$
BEGIN
    -- Insérer un enregistrement dans productrestockprices_prr pour chaque ligne de restock
    INSERT INTO productrestockprices_prr (
        product_ref_prr,
        buying_price_prr,
        restock_id_prr,
        restock_date_prr
    )
    VALUES (
        NEW.product_id_lrs,
        NEW.unit_price_lrs,
        NEW.restock_id_lrs,
        NOW()
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_restock_price_history ON line_restock_lrs;

-- ATTENTION LES KHEYS : Trigger désactivé pour permettre des dates historiques variées lors du seeding
-- CREATE TRIGGER trg_restock_price_history
-- AFTER INSERT ON line_restock_lrs
-- FOR EACH ROW
-- EXECUTE FUNCTION trg_record_restock_price_history();

-- 9. Trigger pour appliquer le réapprovisionnement au stock du produit uniquement si status = 'received'
-- Ce trigger s'active lors de l'UPDATE du status du restock
CREATE OR REPLACE FUNCTION trg_apply_restock_to_product_on_received()
RETURNS TRIGGER AS $$
BEGIN
    -- Vérifier si le status passe à 'received'
    IF NEW.status_res = 'received' AND (OLD.status_res IS NULL OR OLD.status_res != 'received') THEN
        -- Mettre à jour le stock de tous les produits dans ce restock
        UPDATE products_pro p
        SET stock_quantity_pro = stock_quantity_pro + lr.quantity_lrs,
            date_last_reassor_pro = NEW.restock_date_res,
            updated_at = NOW()
        FROM line_restock_lrs lr
        WHERE lr.restock_id_lrs = NEW.id_res
          AND p.id_pro = lr.product_id_lrs;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_restock_update_stock_on_received ON restock_res;

CREATE TRIGGER trg_restock_update_stock_on_received
AFTER UPDATE OF status_res ON restock_res
FOR EACH ROW
EXECUTE FUNCTION trg_apply_restock_to_product_on_received();