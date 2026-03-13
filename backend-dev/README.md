# StockS API

API RESTful pour la gestion de stocks, de produits, de commandes et l'analyse de données (KPIs).

## Table des matières
- [Technologies](#technologies)
- [Structure du projet](#structure-du-projet)
- [Démarrage rapide](#démarrage-rapide)
- [Documentation de l'API](#documentation-de-lapi)
- [Collection Postman](#collection-postman)
- [Administration de la base de données](#administration-de-la-base-de-données)
- [Commandes utiles](#commandes-utiles)

## Technologies

- **Langage :** [Rust](https://www.rust-lang.org/)
- **Framework Web :** [Axum](https://github.com/tokio-rs/axum)
- **Base de données :** [PostgreSQL](https://www.postgresql.org/)
- **Accès base de données :** [SQLx](https://github.com/launchbadge/sqlx)
- **Migrations :** [Refinery](https://github.com/rust-db/refinery) (utilisé via un binaire custom) et `sqlx-cli` pour la vérification compile-time.
- **Documentation API :** [OpenAPI (Swagger)](https://www.openapis.org/) générée avec [Utoipa](https://github.com/juhaku/utoipa)
- **Containerisation :** [Docker](https://www.docker.com/) & [Docker Compose](https://docs.docker.com/compose/)

## Structure du projet

Le projet suit une architecture modulaire et basée sur les fonctionnalités ("feature-based").

```
stocks_api/
├── Cargo.toml      # Dépendances Rust
├── migrations/     # Migrations SQL (format sqlx-cli)
├── src/
│   ├── main.rs     # Point d'entrée (non utilisé pour le serveur)
│   ├── openapi.rs  # Configuration de la documentation OpenAPI/Swagger
│   ├── bin/        # Points d'entrée des binaires
│   │   ├── migrate.rs  # Binaire pour lancer les migrations
│   │   ├── seed.rs     # Binaire pour peupler la base de données
│   │   └── server.rs   # Point d'entrée principal de l'API
│   ├── common/     # Modules partagés (erreurs, sécurité, etc.)
│   └── features/   # Cœur de l'application, un module par fonctionnalité
│       ├── auth/
│       ├── products/
│       └── ...
└── ...
```

Chaque module dans `features` contient généralement :
- `handlers.rs`: Logique des points d'entrée de l'API (controllers).
- `services.rs`: Logique métier.
- `dto.rs`: Objets de transfert de données (Data Transfer Objects).
- `router.rs`: Définition des routes pour la fonctionnalité.
- `mod.rs`: Déclaration des modules.

## Démarrage rapide

### Prérequis

- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/)

### Lancement

1.  Clonez le dépôt.
2.  À la racine du dossier `backend`, lancez l'application avec la commande suivante :

    ```bash
    make up
    ```
    Cette commande va :
    - Construire les images Docker.
    - Démarrer les conteneurs (API, base de données, pgAdmin).
    - Appliquer les migrations de la base de données.

3.  L'API sera accessible à l'adresse `http://localhost:8090`.

## Documentation de l'API

Une fois l'application lancée, la documentation interactive Swagger UI est disponible à l'adresse suivante :

**[http://localhost:8090/swagger-ui](http://localhost:8090/swagger-ui)**

Cette interface vous permet d'explorer tous les points d'entrée de l'API, de voir les modèles de données et de tester les requêtes directement depuis votre navigateur.

## Collection Postman

Une collection Postman est disponible à la racine du projet pour tester l'API :
`Stock_API_Postman_Collection.json`

Vous pouvez l'importer dans [Postman](https://www.postman.com/) pour avoir une configuration prête à l'emploi.

## Administration de la base de données

[pgAdmin](https://www.pgadmin.org/), un outil d'administration pour PostgreSQL, est également déployé et accessible à :

**[http://localhost:5050](http://localhost:5050)**

Identifiants par défaut (configurés dans `docker-compose.yml`) :
- **Email :** `admin@example.com`
- **Mot de passe :** `admin`

## Commandes utiles

### Lancement complet (avec rebuild)

**Linux/macOS :**

```bash
sudo docker compose down -v && sudo docker compose up --build
```

**Windows (PowerShell) :**

```powershell
docker compose down -v; docker compose up --build
```

**Windows (CMD) :**

```cmd
docker compose down -v && docker compose up --build
```
