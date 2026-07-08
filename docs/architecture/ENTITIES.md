# Entités et champs de création — API StockS

Ce document liste toutes les entités pouvant être créées via l'API, leurs champs,
et précise lesquels sont obligatoires ou optionnels lors de la création.

**Légende** :
- ✅ **Obligatoire** — la requête échoue si le champ est absent
- ⚪ **Optionnel** — peut être omis ; une valeur par défaut est appliquée si applicable
- 🔒 **Auto-généré** — ne pas fournir ; géré par le backend

---

## 1. Platform Admin

Créer un compte admin plateforme (rôle super-utilisateur pour gérer les tenants).

**Endpoint** : `POST /auth/register`
**Authentification** : aucune

| Champ       | Type     | Statut         | Description                         |
|-------------|----------|----------------|-------------------------------------|
| `email`     | string   | ✅ Obligatoire | Email unique                        |
| `password`  | string   | ✅ Obligatoire | Mot de passe en clair (sera hashé)  |

**Exemple** :
```json
{
  "email": "admin@example.com",
  "password": "adminpass"
}
```

---

## 2. Tenant (Commerce)

Créer un nouveau commerce. Chaque tenant a sa propre base de données isolée.

**Endpoint** : `POST /admin/tenants`
**Authentification** : JWT + platform admin

| Champ     | Type   | Statut         | Description                                   |
|-----------|--------|----------------|-----------------------------------------------|
| `name`    | string | ✅ Obligatoire | Nom affiché du commerce                       |
| `slug`    | string | ✅ Obligatoire | Identifiant URL unique (ex: `"taynas"`)       |
| `email`   | string | ✅ Obligatoire | Email de contact unique                       |
| `phone`   | string | ⚪ Optionnel   | Téléphone                                     |
| `address` | string | ⚪ Optionnel   | Adresse postale                               |
| `siret`   | string | ⚪ Optionnel   | Numéro SIRET                                  |
| `id`      | UUID   | 🔒 Auto        | Généré par le serveur                         |
| `db_name` | string | 🔒 Auto        | Nom de la base tenant créée automatiquement   |
| `status`  | string | 🔒 Auto        | Initialisé à `"active"`                       |

**Exemple** :
```json
{
  "name": "tayna store",
  "slug": "taynas",
  "email": "contact@tayna.fr",
  "phone": "0612345678",
  "address": "14 rue de la Paix, 13005 Paris",
  "siret": "12345678301234"
}
```

---

## 3. User (Client d'un commerce)

Créer un utilisateur dans un tenant. Un code de fidélité unique (`FID-XXXXXXXX`)
est généré automatiquement.

**Endpoint** : `POST /api/:commerce_id/users`
**Authentification** : JWT

| Champ           | Type   | Statut         | Description                                           |
|-----------------|--------|----------------|-------------------------------------------------------|
| `email`         | string | ✅ Obligatoire | Email unique dans le tenant                           |
| `firstname`     | string | ✅ Obligatoire | Prénom                                                |
| `lastname`      | string | ✅ Obligatoire | Nom                                                   |
| `password`      | string | ✅ Obligatoire | Mot de passe                                          |
| `phone`         | string | ⚪ Optionnel   | Modifiable plus tard via `PUT`                        |
| `fidelity_code` | string | 🔒 Auto        | Format `FID-XXXXXXXX` (8 chiffres) retourné dans la réponse |
| `status`        | string | 🔒 Auto        | Initialisé à `"active"`                               |

**Exemple** :
```json
{
  "email": "testuser@example.com",
  "firstname": "Alice",
  "lastname": "Martin",
  "password": "password123"
}
```

**Réponse** : inclut `fidelity_code` (à utiliser comme code-barre côté front).

---

## 4. Supplier (Fournisseur)

**Endpoint** : `POST /api/:commerce_id/suppliers`
**Authentification** : JWT

| Champ         | Type   | Statut         | Contraintes          |
|---------------|--------|----------------|----------------------|
| `name_sup`    | string | ✅ Obligatoire | 1 à 255 caractères   |
| `email_sup`   | string | ✅ Obligatoire | Format email valide  |
| `phone_sup`   | string | ✅ Obligatoire | 1 à 20 caractères    |
| `address_sup` | string | ✅ Obligatoire | 1 à 500 caractères   |

**Exemple** :
```json
{
  "name_sup": "Test Supplier SARL",
  "email_sup": "contact@testsupplier.com",
  "phone_sup": "+33612345678",
  "address_sup": "12 rue de la Paix, 75001 Paris"
}
```

---

## 5. Product (Produit)

**Endpoint** : `POST /api/:commerce_id/products`
**Authentification** : JWT

| Champ             | Type               | Statut         | Description                                    |
|-------------------|--------------------|----------------|------------------------------------------------|
| `name`            | string             | ✅ Obligatoire | Nom unique                                     |
| `category`        | string             | ✅ Obligatoire | Catégorie libre                                |
| `reference`       | string             | ✅ Obligatoire | Référence unique                               |
| `supplier_id`     | int                | ✅ Obligatoire | ID d'un supplier existant                      |
| `stock_quantity`  | int                | ✅ Obligatoire | Stock initial (≥ 0)                            |
| `buying_price`    | number (decimal)   | ✅ Obligatoire | Prix d'achat                                   |
| `status`          | enum               | ⚪ Optionnel   | Défaut: `"InStock"`                            |
| `date_last_reassor` | datetime         | 🔒 Auto        | Initialisée à la création                      |

**Valeurs de `status`** : `"InStock"` · `"OutOfStock"` · `"Discontinued"` · `"Ordered"`

**Exemple** :
```json
{
  "name": "Test Product API",
  "category": "Électronique",
  "reference": "TEST-API-001",
  "supplier_id": 1,
  "stock_quantity": 100,
  "buying_price": 15.99,
  "status": "InStock"
}
```

---

## 6. Order (Commande)

Crée une commande + ses lignes en une seule requête. Le montant est calculé
automatiquement et le stock des produits est décrémenté. Des points de fidélité
sont attribués automatiquement après création.

**Endpoint** : `POST /api/:commerce_id/orders`
**Authentification** : JWT

| Champ          | Type   | Statut         | Description                                              |
|----------------|--------|----------------|----------------------------------------------------------|
| `user_id`      | int    | ✅ Obligatoire | ID d'un user existant                                    |
| `status`       | string | ✅ Obligatoire | `pending` · `confirmed` · `shipped` · `delivered` · `cancelled` |
| `line_items`   | array  | ✅ Obligatoire | Au moins une ligne                                       |
| `amount`       | decimal | 🔒 Auto        | Calculé à partir des lignes                              |
| `order_date`   | datetime | 🔒 Auto      | Définie à la création                                    |

### Champs d'une ligne (`line_items[]`)

| Champ         | Type | Statut         | Description                |
|---------------|------|----------------|----------------------------|
| `product_id`  | int  | ✅ Obligatoire | Doit exister et avoir du stock |
| `quantity`    | int  | ✅ Obligatoire | > 0                        |
| `unit_price`  | decimal | 🔒 Auto     | Lu depuis le produit       |
| `line_total`  | decimal | 🔒 Auto     | `quantity × unit_price`    |

**Exemple** :
```json
{
  "user_id": 1,
  "status": "pending",
  "line_items": [
    { "product_id": 1, "quantity": 2 },
    { "product_id": 2, "quantity": 1 }
  ]
}
```

**Effets secondaires** :
- Décrémente `stock_quantity` des produits concernés
- Attribue des points de fidélité à l'utilisateur : `floor(amount / euros_per_point)`

---

## 7. Restock (Réapprovisionnement)

Crée un ordre de réapprovisionnement auprès d'un fournisseur. Quand son statut
passe à `received`, le stock des produits est automatiquement incrémenté.

**Endpoint** : `POST /api/:commerce_id/restocks`
**Authentification** : JWT

| Champ          | Type     | Statut         | Description                                      |
|----------------|----------|----------------|--------------------------------------------------|
| `supplier_id`  | int      | ⚪ Optionnel   | Peut être `null` si fournisseur inconnu          |
| `lines`        | array    | ✅ Obligatoire | Au moins une ligne                               |
| `status`       | enum     | ⚪ Optionnel   | Défaut: `"pending"`                              |
| `restock_date` | datetime | ⚪ Optionnel   | Défaut: date/heure actuelle                      |

**Valeurs de `status`** : `"pending"` · `"in_transit"` · `"received"` · `"cancelled"`

### Champs d'une ligne (`lines[]`)

| Champ         | Type    | Statut         | Contraintes                      |
|---------------|---------|----------------|----------------------------------|
| `product_id`  | int     | ✅ Obligatoire | Doit exister                     |
| `quantity`    | int     | ✅ Obligatoire | > 0                              |
| `unit_price`  | decimal | ✅ Obligatoire | ≥ 0 (prix d'achat auprès du fournisseur) |
| `total_price` | decimal | 🔒 Auto        | `quantity × unit_price`          |

**Exemple** :
```json
{
  "supplier_id": 1,
  "status": "pending",
  "lines": [
    { "product_id": 1, "quantity": 50, "unit_price": "12.50" },
    { "product_id": 2, "quantity": 30, "unit_price": "8.00" }
  ]
}
```

---

## 8. Loyalty Transaction (Ajustement manuel de points)

Ajoute ou retire manuellement des points à un utilisateur (cadeau, correction,
geste commercial). Les points gagnés via les commandes sont attribués
automatiquement — cet endpoint est réservé aux ajustements hors commande.

**Endpoint** : `POST /api/:commerce_id/loyalty/users/:user_id/points`
**Authentification** : JWT

| Champ     | Type         | Statut         | Description                                                   |
|-----------|--------------|----------------|---------------------------------------------------------------|
| `points`  | int (signé)  | ✅ Obligatoire | `> 0` = ajout, `< 0` = retrait. **`0` est refusé.**           |
| `reason`  | string       | ⚪ Optionnel   | Motif libre, conservé pour audit (ex: "Cadeau anniversaire")  |

**Règles** :
- `points = 0` → 422 `INVALID_VALUE`
- Retrait qui ferait descendre le solde sous 0 → 422 `INSUFFICIENT_POINTS`
- L'ajustement est persisté comme une transaction dans `loyalty_points_lpo` (sans `order_id`)

**Exemple — ajouter** :
```json
{
  "points": 100,
  "reason": "Cadeau d'anniversaire"
}
```

**Exemple — retirer** :
```json
{
  "points": -50,
  "reason": "Correction manuelle"
}
```

**Réponse** (201) :
```json
{
  "data": {
    "user_id": 1,
    "adjustment": 100,
    "reason": "Cadeau d'anniversaire",
    "new_total_points": 350,
    "transaction": { "id": 42, "order_id": null, "points": 100, "reason": "...", "created_at": "..." }
  }
}
```

---

## Singletons (configuration, pas de création manuelle)

### Loyalty Config

**Endpoint de modification** : `PUT /api/:commerce_id/loyalty/config`
La config est auto-créée avec les valeurs par défaut à la première lecture ;
on ne peut que la modifier.

| Champ              | Type    | Défaut | Contraintes               | Rôle                                            |
|--------------------|---------|--------|---------------------------|-------------------------------------------------|
| `euros_per_point`  | decimal | 2.00   | > 0                       | Nb d'€ dépensés pour 1 point gagné              |
| `points_required`  | int     | 100    | > 0                       | Nb de points requis pour une tranche de réduction |
| `discount_percent` | decimal | 5.00   | > 0 et ≤ 100              | % de réduction par tranche                      |

Tous les champs sont optionnels lors d'un `PUT` ; les champs omis ne sont pas modifiés.

**Exemple** :
```json
{
  "euros_per_point": 2.00,
  "points_required": 100,
  "discount_percent": 5.00
}
```

---

## Notes globales

- **JWT** : toutes les routes sauf `/auth/*` et les health checks requièrent un header `Authorization: Bearer <token>`.
- **commerce_id** : c'est l'UUID du tenant, récupérable via `GET /admin/tenants` ou renvoyé à la connexion. Toutes les routes tenant le requièrent dans l'URL.
- **Unicité** : les contraintes d'unicité (email user, reference produit, slug tenant, etc.) sont vérifiées en base — une requête en conflit renvoie une erreur 500 ou 422 selon le cas.
- **Dates** : les formats attendus sont ISO 8601 UTC (ex: `"2026-04-24T14:30:00Z"`).
