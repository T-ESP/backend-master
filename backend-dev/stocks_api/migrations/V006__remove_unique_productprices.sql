-- Migration V006: Retirer la contrainte UNIQUE sur productprices_prp pour permettre l'historisation des prix de vente

-- 1. Retirer la contrainte UNIQUE sur product_ref_prp
ALTER TABLE productprices_prp
DROP CONSTRAINT IF EXISTS productprices_prp_product_ref_prp_key;

-- 2. Ajouter un index pour optimiser les requêtes (sans contrainte UNIQUE)
CREATE INDEX IF NOT EXISTS idx_productprices_product_ref ON productprices_prp(product_ref_prp);

-- 3. Ajouter un index sur created_at pour faciliter les requêtes temporelles
CREATE INDEX IF NOT EXISTS idx_productprices_created_at ON productprices_prp(created_at DESC);

-- 4. Ajouter un index composite pour récupérer rapidement le dernier prix d'un produit
CREATE INDEX IF NOT EXISTS idx_productprices_product_date ON productprices_prp(product_ref_prp, created_at DESC);

-- 5. Ajouter des commentaires pour documenter la table
COMMENT ON TABLE productprices_prp IS 'Historique des prix de vente des produits. Chaque ligne représente un changement de prix.';
COMMENT ON COLUMN productprices_prp.product_ref_prp IS 'Référence du produit (peut avoir plusieurs entrées pour l''historique)';
COMMENT ON COLUMN productprices_prp.price_prp IS 'Prix de vente à cette date';
COMMENT ON COLUMN productprices_prp.created_at IS 'Date d''entrée en vigueur du prix';
