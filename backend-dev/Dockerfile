###############################################################################
# -------- build stage --------------------------------------------------------
###############################################################################
FROM rust:1.82 AS builder

# 1) on place la racine du repo
WORKDIR /build

# 2) on copie TOUT le contexte (pas seulement stocks_api/)
#    ainsi le chemin reste identique entre host et conteneur
COPY . .

# 3) on passe dans le crate Rust
WORKDIR /build/stocks_api

# 4) dépendances système
RUN apt-get update && apt-get install -y pkg-config libpq-dev

# 5) compilation des trois exécutables
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin migrate --bin server --bin seed

###############################################################################
# -------- runtime stage ------------------------------------------------------
###############################################################################
FROM debian:bookworm-slim

# Dépendances runtime pour tokio-postgres
RUN apt-get update && apt-get install -y libpq5 ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Dossier d’exécution
WORKDIR /app

# Copie des binaires compilés
COPY --from=builder /build/stocks_api/target/release/migrate /app/migrate
COPY --from=builder /build/stocks_api/target/release/server  /app/server
COPY --from=builder /build/stocks_api/target/release/seed    /app/seed

# Port exposé pour le serveur web
EXPOSE 8080

# Pas d’ENTRYPOINT ici car la commande est définie dans docker-compose
