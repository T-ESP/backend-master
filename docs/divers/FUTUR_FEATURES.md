# FUTUR_FEATURES — StockS

Liste d'idées de fonctionnalités à implémenter pour faire passer **StockS** d'un MVP bien architecturé à un projet de niveau professionnel.

Organisé par catégorie, avec une priorisation finale en bas de document.

---

## Sommaire

- [Features métier](#features-métier)
  - [Stock & produits](#stock--produits)
  - [Commandes & ventes](#commandes--ventes)
  - [Fidélité & clients](#fidélité--clients)
- [Features techniques](#features-techniques)
  - [Tests](#tests-priorité-absolue)
  - [Observabilité & qualité](#observabilité--qualité)
  - [Sécurité](#sécurité)
  - [CI/CD](#cicd)
- [Features UX / Frontend](#features-ux--frontend)
- [Features "wow"](#features-wow)
- [Top 5 priorités](#top-5-priorités-si-peu-de-temps)

---

## Features métier

Les plus visibles côté démo et jury — elles donnent l'impression d'un produit fini.

### Stock & produits

- [ ] **Variantes de produits** (taille, couleur, format) — un même produit avec plusieurs SKU
- [ ] **Bundles / packs** (vendre plusieurs produits ensemble à prix dédié)
- [ ] **Code-barres / QR code** : génération + scan via webcam pour ajouter au stock ou à une commande
- [ ] **Historique de prix complet** (pas juste le dernier prix) avec graphique d'évolution
- [ ] **Réservations de stock** (panier qui réserve avant la commande pour éviter le surbooking)
- [ ] **Inventaire physique** : module pour faire un comptage et réconcilier les écarts
- [ ] **Multi-entrepôts** : un produit peut être stocké à plusieurs endroits, transferts entre stocks

### Commandes & ventes

- [ ] **Gestion des retours / remboursements** (au lieu de `DELETE` une commande)
- [ ] **Pré-commandes / backorders** sur produits en rupture
- [ ] **Devis (quotes)** convertibles en commande
- [ ] **Facturation PDF** générée automatiquement (logo, mentions légales, TVA)
- [ ] **Multi-devises + conversion** automatique (utile en multi-tenant)
- [ ] **TVA / taxes** configurables par produit / catégorie

### Fidélité & clients

- [ ] **Coupons / codes promo** avec règles (montant min, période, usage unique)
- [ ] **Tiers de fidélité** (Bronze / Argent / Or) avec avantages différents
- [ ] **Programme de parrainage** (un client invite un ami, les deux gagnent des points)
- [ ] **Emails / SMS automatiques** : confirmation commande, alerte stock, anniversaire client

---

## Features techniques

Celles qui impressionnent un dev senior ou un prof tech.

### Tests (priorité absolue)

C'est le plus gros manque actuel : **0 test** dans le repo.

- [ ] **Tests unitaires backend** Rust (`cargo test`) sur la logique métier (loyalty, orders, stock)
- [ ] **Tests d'intégration** avec une vraie DB de test via `testcontainers-rs`
- [ ] **Tests E2E Playwright** (déjà installé, jamais utilisé) sur les parcours critiques
- [ ] **Coverage report** dans la CI + badge dans le README

### Observabilité & qualité

- [ ] **Logging structuré** avec `tracing` (remplacer les ~135 `println!` / `eprintln!`)
- [ ] **Sentry** ou équivalent pour l'error tracking côté front + back
- [ ] **Healthcheck endpoint** `/health` détaillé (DB, dépendances, version)
- [ ] **Métriques Prometheus** + dashboard Grafana (latence, erreurs, requêtes)
- [ ] **Audit log** : table qui trace qui a modifié quoi et quand

### Sécurité

Plusieurs points faibles actuels à corriger en priorité.

- [ ] **Rate limiting** (`tower-governor`) sur auth et endpoints publics
- [ ] **Secrets via variables d'env** — le JWT secret hardcodé dans `docker-compose.yml` est un drapeau rouge
- [ ] **2FA** (TOTP via app authenticator)
- [ ] **Reset password** par email avec token expirant
- [ ] **CSRF protection** + headers sécurité (équivalent Helmet)
- [ ] **Soft delete** partout au lieu de `DELETE` physique
- [ ] **RGPD** : endpoint d'export et de suppression des données utilisateur

### CI/CD

- [ ] **GitHub Actions** : ajouter un job tests + lint (`clippy` + `eslint`) avant le deploy
- [ ] **Dependabot** ou **Renovate** pour les updates de dépendances
- [ ] **Environnements** staging + prod séparés
- [ ] **Preview deployments** automatiques sur chaque PR

---

## Features UX / Frontend

- [ ] **Internationalisation complète** (`i18next` est setup mais vide) — FR / EN / ES
- [ ] **Mode sombre** propre et persistant
- [ ] **Notifications temps réel** via WebSocket / SSE (nouvelle commande, alerte stock)
- [ ] **Export CSV / Excel** sur toutes les listes (produits, commandes, clients)
- [ ] **Import CSV** pour bulk-créer des produits
- [ ] **Drag & drop** pour réorganiser ou uploader (images produits)
- [ ] **Storybook** pour documenter les composants UI
- [ ] **PWA** : installable sur mobile, mode offline basique
- [ ] **Recherche globale** type `Cmd+K` (Algolia-like, raccourci clavier)
- [ ] **Filtres avancés sauvegardés** sur les listes
- [ ] **Skeleton loaders** partout au lieu de spinners génériques
- [ ] **Empty states** dessinés (illustrations quand il n'y a rien)

---

## Features "wow"

Pour démarquer le projet d'un MVP standard.

- [ ] **Assistant IA conversationnel** (Mistral est déjà intégré) : *"quels produits dois-je recommander ce mois-ci ?"* → réponse en langage naturel
- [ ] **Dashboard customisable** : drag & drop des widgets KPI
- [ ] **Alertes intelligentes prédictives** : *"tu vas être en rupture dans 6 jours sur X"*
- [ ] **Suggestions de prix dynamiques** basées sur historique + saisonnalité
- [ ] **API publique documentée** + génération de clés API + page développeurs
- [ ] **Webhooks sortants** configurables (Zapier-friendly)
- [ ] **App mobile** React Native partageant la logique métier
- [ ] **Marketplace de plugins** ou intégrations (Shopify, WooCommerce import)

---

## Top 5 priorités (si peu de temps)

Si tu dois choisir, voici les features avec le meilleur ratio impact / effort :

1. **Tests** (au moins quelques-uns + CI qui les run) — immédiat ×10 sur la crédibilité
2. **Logging structuré + Sentry** — montre que t'as pensé prod
3. **Sécurité** : virer le JWT hardcodé, ajouter rate limiting + reset password
4. **i18n complet + dark mode** — visible direct à la démo, peu d'effort
5. **Export CSV + facturation PDF** — features métier toujours demandées, gros effet "produit fini"

---

## État actuel du projet (rappel)

**Stack :** Rust (Axum + SQLx) + React 19 + PostgreSQL + Mistral AI
**Architecture :** 14 modules backend, 15 modules frontend, multi-tenant, ~15k lignes Rust
**Points forts :** archi propre, business logic riche (loyalty, KPIs, AI insights), Docker + CD GitHub Actions
**Points faibles principaux :** 0 test, logging non-structuré, secrets hardcodés, i18n vide, pas d'audit trail
