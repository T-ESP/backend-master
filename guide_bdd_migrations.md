# 📚 Guide express — Base de données & migrations

Stack : **PostgreSQL 16** (service `db`), binaire Rust **`migrate`** (service one-shot), **API `web`**, **pgAdmin**.  
Un `docker compose up --build` suffit pour démarrer, appliquer les migrations et lancer l’API.

---

## 🚀 Démarrage

```bash
docker compose up --build
```

Logs attendus :
- `✅ Migrations terminées` (service **migrate**)
- `🌐 API disponible ... 8080` (service **web**)

Services :
- **API** : http://localhost:8090
- **pgAdmin** : http://localhost:5050 (admin@example.com / admin)
- **Postgres** : localhost:5432 (réseau Compose: host = `db`)

---

## 🔗 Connexion

- Paramètres DB (par défaut) :
```
POSTGRES_USER=user
POSTGRES_PASSWORD=pass
POSTGRES_DB=stocks
```

> En équipe/prod : utilisez un `.env` et **ne commitez pas** de secrets.

---

## 📁 Migrations — où les mettre ?

Placez les fichiers SQL dans :
```
stocks_api/migrations/
```

**Convention** :
```
YYYYMMDD_HHMM__description.sql
# ex. 20250818_1410__create_users.sql
```

> Les migrations sont **embarquées** dans le binaire `migrate` au build.  
> Après ajout/modification : **rebuild obligatoire** → `docker compose up --build`.

---

## ➕ Ajouter une migration (3 étapes)

1) Créez le fichier :
```
stocks_api/migrations/20250818_1410__add_orders_table.sql
```

2) Écrivez un SQL idempotent (ex.) :
```sql
CREATE TABLE IF NOT EXISTS orders (
  id          BIGSERIAL PRIMARY KEY,
  user_id     BIGINT NOT NULL,
  total_cents INTEGER NOT NULL,
  created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

3) Appliquez :
```bash
docker compose up --build
# ou uniquement rejouer les migrations :
docker compose run --rm migrate
```

---

## ✅ Vérifier que tout est en place

### 🔧 Configuration d’un serveur dans pgAdmin

1. Ouvre ton navigateur et va sur [http://localhost:5050](http://localhost:5050)
2. Connecte-toi avec :
   - **Email** : `admin@example.com`
   - **Mot de passe** : `admin`
3. Clique sur `Add New Server`
4. Dans l'onglet **General** :
   - **Name** : `Local DB` (ou un nom de ton choix)
5. Dans l'onglet **Connection** :
   - **Host name / address** : `db`
   - **Port** : `5432`
   - **Maintenance database** : `stocks`
   - **Username** : `user`
   - **Password** : `pass`
6. Clique sur **Save**
7. Tu peux maintenant naviguer dans :
   ```
   Servers > Local DB > Databases > stocks > Schemas > public > Tables
   ```


### Ligne de commande
```bash
# Lister les tables non système
docker compose exec db psql -U user -d stocks -c "\dt"

---

## 🔁 Commandes utiles

```bash
# Rejouer les migrations seulement
docker compose run --rm migrate

# Réexécuter le seeder
docker compose run --rm seed

# Logs en live
docker compose logs -f db
docker compose logs -f migrate
docker compose logs -f seed
docker compose logs -f web

# Reset complet (⚠️ supprime les données locales)
docker compose down -v && docker compose up --build
```

---
