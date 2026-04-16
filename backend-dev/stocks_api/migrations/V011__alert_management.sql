-- ============================================================================
-- Migration V011: Alert Management System
-- Adds lifecycle management for notifications (new → acknowledged → in_progress → resolved / dismissed)
-- ============================================================================

-- 1) Create ENUM for notification status
CREATE TYPE notification_status_enum AS ENUM (
    'new',
    'acknowledged',
    'in_progress',
    'resolved',
    'dismissed'
);

-- 2) Make product_id nullable (supplier-level alerts have no product)
ALTER TABLE notifications
ALTER COLUMN product_id DROP NOT NULL;

-- 3) Convert status column from VARCHAR to the new ENUM
-- First drop the old default
ALTER TABLE notifications
ALTER COLUMN status DROP DEFAULT;

-- Convert existing values: map 'unresolved' → 'new', anything else → 'new'
UPDATE notifications SET status = 'new' WHERE status IS NOT NULL;
UPDATE notifications SET status = 'new' WHERE status IS NULL;

-- Change column type
ALTER TABLE notifications
ALTER COLUMN status TYPE notification_status_enum
USING status::notification_status_enum;

-- Set the new default
ALTER TABLE notifications
ALTER COLUMN status SET DEFAULT 'new'::notification_status_enum;

-- Make status NOT NULL
ALTER TABLE notifications
ALTER COLUMN status SET NOT NULL;

-- 4) Add updated_at column
ALTER TABLE notifications
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ DEFAULT NOW();

-- 5) Add indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_notifications_status ON notifications(status);
CREATE INDEX IF NOT EXISTS idx_notifications_severity ON notifications(severity);
CREATE INDEX IF NOT EXISTS idx_notifications_model_type ON notifications(model_type);
CREATE INDEX IF NOT EXISTS idx_notifications_product ON notifications(product_id);
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at DESC);

-- 6) Composite index for common filter combinations
CREATE INDEX IF NOT EXISTS idx_notifications_status_severity ON notifications(status, severity);
