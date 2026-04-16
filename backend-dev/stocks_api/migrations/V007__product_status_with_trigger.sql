-- ============================================================================
-- Migration V007: Convert product status from VARCHAR to ENUM type
-- ============================================================================

-- ============================================================================
-- 1) CREATE ENUM TYPE for product status
-- ============================================================================
CREATE TYPE product_status_enum AS ENUM (
    'in_stock',
    'out_of_stock',
    'discontinued',
    'ordered'
);

-- ============================================================================
-- 2) DROP old trigger and constraint from V004
-- ============================================================================
DROP TRIGGER IF EXISTS trg_products_stock_status ON products_pro;
DROP FUNCTION IF EXISTS trg_update_product_status_from_stock();

ALTER TABLE products_pro
DROP CONSTRAINT IF EXISTS chk_products_status;

-- ============================================================================
-- 3) CONVERT status_pro column from VARCHAR to ENUM
-- ============================================================================
-- First, drop the existing DEFAULT (it's a VARCHAR default)
ALTER TABLE products_pro
ALTER COLUMN status_pro DROP DEFAULT;

-- Then convert the column type using USING clause to cast existing values
ALTER TABLE products_pro
ALTER COLUMN status_pro TYPE product_status_enum
USING status_pro::product_status_enum;

-- Finally, set the new DEFAULT with the correct enum type
ALTER TABLE products_pro
ALTER COLUMN status_pro SET DEFAULT 'in_stock'::product_status_enum;

-- ============================================================================
-- 4) UPDATE existing products to set default if NULL
-- ============================================================================
UPDATE products_pro
SET status_pro = 'in_stock'::product_status_enum
WHERE status_pro IS NULL;

-- ============================================================================
-- 5) CREATE FUNCTION to handle automatic status updates on stock changes
-- ============================================================================
CREATE OR REPLACE FUNCTION update_product_status_on_stock_change()
RETURNS TRIGGER AS $$
BEGIN
    -- Only automatically update if status is 'in_stock' or 'out_of_stock'
    -- 'discontinued' and 'ordered' are manual and should not be auto-updated

    IF NEW.status_pro IN ('in_stock', 'out_of_stock') THEN
        IF NEW.stock_quantity_pro = 0 THEN
            -- Stock hit zero -> out_of_stock (only if currently in_stock)
            IF NEW.status_pro = 'in_stock' THEN
                NEW.status_pro := 'out_of_stock'::product_status_enum;
            END IF;
        ELSIF NEW.stock_quantity_pro > 0 THEN
            -- Stock increased from zero -> in_stock (only if currently out_of_stock)
            IF NEW.status_pro = 'out_of_stock' THEN
                NEW.status_pro := 'in_stock'::product_status_enum;
            END IF;
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- 6) CREATE TRIGGER to call the function on INSERT/UPDATE
-- ============================================================================
DROP TRIGGER IF EXISTS trg_update_product_status ON products_pro;

CREATE TRIGGER trg_update_product_status
BEFORE INSERT OR UPDATE ON products_pro
FOR EACH ROW
EXECUTE FUNCTION update_product_status_on_stock_change();

-- ============================================================================
-- 7) CREATE INDEX on status_pro for better query performance
-- ============================================================================
CREATE INDEX IF NOT EXISTS idx_products_status ON products_pro(status_pro);
