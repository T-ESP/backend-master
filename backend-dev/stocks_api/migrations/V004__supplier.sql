-- Migration n°3 : gestion avancée du stock et du réapprovisionnement

-- Ajout d'une colonne status sur la table products_pro
ALTER TABLE products_pro
ADD COLUMN IF NOT EXISTS status_pro VARCHAR NOT NULL DEFAULT 'in_stock';

-- Ajout d'une contrainte pour limiter les valeurs possibles
ALTER TABLE products_pro
ADD CONSTRAINT chk_products_status
CHECK (status_pro IN ('in_stock', 'out_of_stock','discontinued'));

-- Fonction de trigger pour mettre à jour le status_pro selon le stock
CREATE OR REPLACE FUNCTION trg_update_product_status_from_stock()
RETURNS TRIGGER AS $$
BEGIN
    -- Si le stock est négatif ou nul, on force à 0 et on passe en out_of_stock
    IF NEW.stock_quantity_pro <= 0 THEN
        NEW.stock_quantity_pro := 0;
        
        -- On ne touche pas aux produits arrêtés (discontinued)
        IF NEW.status_pro <> 'discontinued' THEN
            NEW.status_pro := 'out_of_stock';
        END IF;
    
    -- Si le stock est > 0 et qu'on était en out_of_stock, on repasse en in_stock
    ELSE
        IF NEW.status_pro = 'out_of_stock' THEN
            NEW.status_pro := 'in_stock';
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger avant INSERT/UPDATE du stock produit
DROP TRIGGER IF EXISTS trg_products_stock_status ON products_pro;

CREATE TRIGGER trg_products_stock_status
BEFORE INSERT OR UPDATE OF stock_quantity_pro ON products_pro
FOR EACH ROW
EXECUTE FUNCTION trg_update_product_status_from_stock();

-- Table de réapprovisionnement
CREATE TABLE IF NOT EXISTS restock_res (
    id_res            SERIAL PRIMARY KEY,
    quantity_res      INTEGER NOT NULL CHECK (quantity_res > 0),
    restock_date_res  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at        TIMESTAMPTZ DEFAULT NOW(),
    updated_at        TIMESTAMPTZ DEFAULT NOW()
);

-- NOTE: Le trigger pour appliquer le restock au stock produit a été déplacé vers line_restock_lrs
-- car un restock peut maintenant contenir plusieurs produits (voir migration V005)

ALTER TABLE supplier_sup
ADD COLUMN IF NOT EXISTS phone_sup VARCHAR;