###############################################################################
# -------- build stage --------------------------------------------------------
#############################################################################
FROM rust:1.82 AS builder

WORKDIR /build

# Copier tout le projet
COPY . .

# Aller dans le vrai crate Rust
WORKDIR /build/stocks_api_master

# Dépendances système
RUN apt-get update && apt-get install -y pkg-config libpq-dev curl

# Pré-télécharger Swagger UI pour éviter l'échec réseau pendant cargo build
RUN curl -L --retry 5 --retry-delay 3 \
    -o /tmp/v5.17.12.zip \
    https://github.com/swagger-api/swagger-ui/archive/refs/tags/v5.17.12.zip

ENV SWAGGER_UI_DOWNLOAD_URL=file:///tmp/v5.17.12.zip

# Compilation des binaires
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin migrate --bin server --bin seed

###############################################################################
# -------- runtime stage ------------------------------------------------------
###############################################################################
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y libpq5 ca-certificates \
      python3 python3-pip \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copier les binaires compilés
COPY --from=builder /build/stocks_api_master/target/release/migrate /app/migrate
COPY --from=builder /build/stocks_api_master/target/release/server  /app/server
COPY --from=builder /build/stocks_api_master/target/release/seed    /app/seed

# Seeder Python (seed.py) — utilisé par POST /admin/tenants/:id/seed
COPY scripts/seed.py           /app/seed.py
COPY scripts/requirements.txt  /app/requirements.txt
RUN pip3 install --no-cache-dir --break-system-packages -r /app/requirements.txt

EXPOSE 8080
