-- Enable UUIDs or extensions si besoin
-- CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- employyes --------------------------------------------------------

--  commerce ----------------------------------------------------------------
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE commerces (
                           id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                           name TEXT NOT NULL,
                           slug TEXT UNIQUE NOT NULL,
                           email TEXT UNIQUE NOT NULL,
                           password_hash TEXT NOT NULL,
                           phone TEXT,
                           address TEXT,
                           siret TEXT,
                           db_name TEXT NOT NULL,
                           status TEXT NOT NULL DEFAULT 'active',
                           created_at TIMESTAMP NOT NULL DEFAULT NOW()
);



-- 8. line items table ------------------------------------------------------------
CREATE TABLE platform_admins (
                                 id SERIAL PRIMARY KEY,
                                 email TEXT UNIQUE NOT NULL,
                                 password_hash TEXT NOT NULL,
                                 created_at TIMESTAMP NOT NULL DEFAULT NOW()
);
