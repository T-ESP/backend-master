-- Migration n°2 : Conversion de toutes les colonnes TIMESTAMP en TIMESTAMPTZ

ALTER TABLE role_rol
    ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'Europe/Paris',
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ USING updated_at AT TIME ZONE 'Europe/Paris';

ALTER TABLE users_usr
    ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'Europe/Paris',
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ USING updated_at AT TIME ZONE 'Europe/Paris';

ALTER TABLE role_user_rus
    ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'Europe/Paris',
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ USING updated_at AT TIME ZONE 'Europe/Paris';

ALTER TABLE supplier_sup
    ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'Europe/Paris',
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ USING updated_at AT TIME ZONE 'Europe/Paris';

ALTER TABLE products_pro
    ALTER COLUMN date_last_reassor_pro TYPE TIMESTAMPTZ USING date_last_reassor_pro AT TIME ZONE 'Europe/Paris',
    ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'Europe/Paris',
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ USING updated_at AT TIME ZONE 'Europe/Paris';

ALTER TABLE productprices_prp
    ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'Europe/Paris',
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ USING updated_at AT TIME ZONE 'Europe/Paris';

ALTER TABLE order_ord
    ALTER COLUMN order_date_ord TYPE TIMESTAMPTZ USING order_date_ord AT TIME ZONE 'Europe/Paris',
    ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'Europe/Paris',
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ USING updated_at AT TIME ZONE 'Europe/Paris';

ALTER TABLE line_order_lor
    ALTER COLUMN created_at TYPE TIMESTAMPTZ USING created_at AT TIME ZONE 'Europe/Paris',
    ALTER COLUMN updated_at TYPE TIMESTAMPTZ USING updated_at AT TIME ZONE 'Europe/Paris';