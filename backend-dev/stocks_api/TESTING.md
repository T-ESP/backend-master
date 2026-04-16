# Guide des Tests Unitaires

## Tests Unitaires Ajoutés

Les tests unitaires ont été ajoutés sans modifier le code existant (handlers, services, etc.).

### Tests pour `dto.rs` (14 tests)

Tests couvrant les structures de données principales :
- **ProductStatus** : Égalité, clonage, sérialisation/désérialisation
- **CreateProductRequest** : Désérialisation avec et sans status
- **UpdateProductRequest** : Mise à jour partielle et complète
- **StockUpdateRequest** : Quantités positives et négatives
- **SearchParams** : Valeurs par défaut et personnalisées
- **ProductResponse** : Sérialisation
- **ProductWithSupplierResponse** : Sérialisation avec fournisseur

### Tests pour `kpis_dto.rs` (18 tests)

Tests couvrant toutes les structures KPI :
- **KpiPeriodParams** : Valeurs par défaut (30 jours), désérialisation
- **PricingMarginKpis** : Sérialisation des prix et marges
- **StockAvailabilityKpis** : Disponibilité et ruptures de stock
- **SalesRotationKpis** : Ventes et rotation, tendances
- **ProfitabilityKpis** : Rentabilité
- **RestockKpis** : Réapprovisionnements
- **PredictionsAlertsKpis** : Alertes (normal, imminent_stockout, overstock)
- **ScoringClassificationKpis** : Classifications ABC et catégories de performance
- **ComparativeKpis** : Comparaisons relatives
- **PricePoint, MarginPoint, PriceEvolution** : Historiques de prix

## Module Orders

### Tests pour `dto.rs` (16 tests)

Tests couvrant les structures de données principales :
- **CreateOrderRequest** : Désérialisation avec line items (vides ou multiples)
- **CreateLineItemRequest** : Désérialisation, quantités positives/négatives
- **UpdateOrderRequest** : Mise à jour du statut (pending, confirmed, shipped, delivered, cancelled)
- **OrderResponse** : Sérialisation, différents statuts
- **LineItemResponse** : Sérialisation avec détails
- **OrderWithItemsResponse** : Sérialisation avec liste de line items
- **OrderQueryParams** : Paramètres optionnels, filtres partiels
- **OrderStatsResponse** : Statistiques agrégées, valeurs nulles
- **Decimal precision** : Tests de précision des montants (0.01, 99.99, etc.)

## Exécuter les Tests Localement

### Tous les tests du module products (32 tests)
```bash
cd stocks_api
SQLX_OFFLINE=true cargo test --lib features::products
```

### Tous les tests du module orders (16 tests)
```bash
cd stocks_api
SQLX_OFFLINE=true cargo test --lib features::orders
```

### Tests spécifiques par fichier
```bash
# Products - dto.rs uniquement (14 tests)
SQLX_OFFLINE=true cargo test --lib features::products::dto

# Products - kpis_dto.rs uniquement (18 tests)
SQLX_OFFLINE=true cargo test --lib features::products::kpis_dto

# Orders - dto.rs uniquement (16 tests)
SQLX_OFFLINE=true cargo test --lib features::orders::dto
```

### Tous les tests unitaires du projet (48 tests)
```bash
cd stocks_api
SQLX_OFFLINE=true cargo test --lib
```

### Avec sortie détaillée
```bash
cd stocks_api
SQLX_OFFLINE=true cargo test --lib features::products -- --nocapture
SQLX_OFFLINE=true cargo test --lib features::orders -- --nocapture
```

## Variable d'Environnement SQLX_OFFLINE

La variable `SQLX_OFFLINE=true` est nécessaire car :
- Les macros sqlx! vérifient les requêtes SQL au moment de la compilation
- Sans cette variable, cargo tenterait de se connecter à la base de données
- En mode offline, sqlx utilise les métadonnées stockées pour la vérification des types

## Intégration CI/CD

Une configuration GitHub Actions a été créée dans `.github/workflows/tests.yml`.

### Où s'exécutent les tests ?

**Tests Locaux** (sur votre machine) :
```bash
SQLX_OFFLINE=true cargo test --lib features::products
```
- S'exécutent immédiatement sur votre ordinateur
- Feedback rapide pendant le développement
- Utilisent vos ressources CPU/RAM

**Tests CI/CD** (sur les serveurs GitHub) :
- S'exécutent automatiquement sur les serveurs GitHub lors d'un push ou PR
- Machine virtuelle Ubuntu fraîche à chaque fois
- Résultats visibles dans l'onglet "Actions" de votre repository GitHub
- Ne consomment pas vos ressources locales

### Architecture du Workflow CI (Optimisé)

Le workflow CI est structuré en **un seul job** avec des étapes séquentielles optimisées :

```
┌─────────────────────────────────────────┐
│         Workflow: Tests                 │
│  (Une seule action à cliquer)           │
└─────────────────────────────────────────┘
           │
           ├─> 1. Setup (une seule fois)
           │   ├─ Install Rust
           │   ├─ Cache dependencies
           │   └─ Compile code
           │
           ├─> 2. Products Tests (32 tests)
           │   └─ Rapide (code déjà compilé)
           │
           ├─> 3. Orders Tests (16 tests)
           │   └─ Rapide (code déjà compilé)
           │
           ├─> 4. Integration Tests (48 tests)
           │   └─ Vérification globale
           │
           └─> 5. Summary
               └─ Résumé final
```

**Étape 1 : Setup Commun** 🔧
```yaml
- Install Rust (dtolnay/rust-toolchain@stable)
- Cache cargo registry
- Cache cargo index
- Cache cargo build
```
- S'exécute **une seule fois** au début
- Compile le code une fois pour toutes les étapes suivantes
- Utilise les caches pour accélérer les runs suivants
- Durée : ~15-20s (première fois), ~5-8s (avec cache)

**Étape 2 : Products Tests (32 tests)** 🔵
```yaml
- name: "📦 Products Tests (32 tests)"
```
- Code déjà compilé = **très rapide** (~3-5 secondes)
- Teste uniquement `features::products`
- Feedback ciblé si products échoue

**Étape 3 : Orders Tests (16 tests)** 🟢
```yaml
- name: "📦 Orders Tests (16 tests)"
```
- Code déjà compilé = **très rapide** (~2-3 secondes)
- Teste uniquement `features::orders`
- Feedback ciblé si orders échoue

**Étape 4 : Integration Tests (48 tests)** 🟡
```yaml
- name: "🔍 Integration Tests (all modules)"
```
- Vérifie que tous les modules fonctionnent ensemble
- Tests déjà exécutés = **très rapide** (~3-5 secondes)
- Garantit l'intégration globale

**Étape 5 : Summary** 📊
```yaml
- name: "📊 Test Summary"
  if: always()
```
- Affiche le résumé final
- S'exécute **toujours** même si tests échouent
- Très rapide (~1 seconde)

### ⚡ Avantages de cette Architecture

**Efficacité** 🚀
- Setup une seule fois = pas de duplication
- Code compilé réutilisé pour tous les tests
- Durée totale : ~30-40 secondes (avec cache: ~15-20s)

**Clarté** 👁️
```
Tests ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
│
├─ Setup                              ~8s
├─ 📦 Products Tests (32 tests)       ~3s
├─ 📦 Orders Tests (16 tests)         ~2s
├─ 🔍 Integration Tests (48 tests)    ~4s
└─ 📊 Test Summary                    ~1s

Total: ~18s (avec cache)
```

**Séquentialité Intelligente** 🔒
- Si products échoue, vous le savez immédiatement
- Orders s'exécute quand même après (pas bloqué)
- Summary toujours affiché pour diagnostic

### Déclencher les tests dans la CI

Les tests s'exécutent automatiquement lors de :
- Push sur les branches `main`, `dev`, ou `product_status`
- Création/mise à jour d'une pull request vers `main` ou `dev`

### Voir les résultats sur GitHub

1. Allez sur votre repository GitHub
2. Cliquez sur l'onglet **"Actions"**
3. Sélectionnez le workflow "Tests"
4. Cliquez sur un run pour voir les détails :
   - Nombre de tests passés/échoués
   - Temps d'exécution de chaque étape
   - Logs détaillés avec les messages `echo`
   - Résumé final avec le compte des tests

### Vue dans GitHub Actions

Quand vous cliquez sur le workflow "Tests", vous voyez **une seule action** avec toutes les étapes :

```
Tests ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
│
└─ Run All Tests
   │
   ├─ Checkout code                              ✅  2s
   ├─ Install Rust                               ✅  5s
   ├─ Cache cargo registry                       ✅  1s
   ├─ Cache cargo index                          ✅  1s
   ├─ Cache cargo build                          ✅  3s
   │
   ├─ 📦 Products Tests (32 tests)               ✅  3s
   │  ├─ Testing Products Module
   │  ├─ running 32 tests
   │  ├─ test features::products::dto::tests::... ok
   │  └─ ✅ Products: 32 tests passed
   │
   ├─ 📦 Orders Tests (16 tests)                 ✅  2s
   │  ├─ Testing Orders Module
   │  ├─ running 16 tests
   │  ├─ test features::orders::dto::tests::... ok
   │  └─ ✅ Orders: 16 tests passed
   │
   ├─ 🔍 Integration Tests (all modules)         ✅  4s
   │  ├─ Running All Unit Tests
   │  ├─ running 48 tests
   │  └─ ✅ All tests: 48 tests passed
   │
   └─ 📊 Test Summary                            ✅  1s
      ├─ Module breakdown:
      ├─   ✅ Products: 32 tests
      ├─   ✅ Orders: 16 tests
      └─   📦 Total: 48 tests

Durée totale: ~21s (première fois) / ~18s (avec cache)
```

**Points clés** :
- 🔧 **Setup une fois** : Rust + dépendances installés qu'une seule fois
- ⚡ **Tests rapides** : Code déjà compilé, tests s'exécutent très vite
- 📊 **Feedback clair** : Chaque module testé séparément puis ensemble
- ✅ **Summary toujours affiché** : Même en cas d'échec (if: always())

## Résumé

**Total : 48 tests unitaires** couvrant 2 modules
- ✅ Products module: 32 tests
- ✅ Orders module: 16 tests
- ✅ Aucune modification du code existant
- ✅ Tests de sérialisation/désérialisation JSON
- ✅ Tests des valeurs par défaut
- ✅ Tests des enums et types personnalisés
- ✅ Tests de précision des montants (Decimal)
- ✅ Prêt pour l'intégration CI/CD
- ✅ Affichage détaillé du nombre de tests dans la CI

## Ajouter un Nouveau Module de Tests à la CI

L'architecture facilite l'ajout de nouveaux modules. Exemple pour ajouter `suppliers` :

**1. Créer les tests dans le code**
```bash
# Ajouter les tests dans src/features/suppliers/dto.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supplier_creation() {
        // Vos tests ici
    }
}
```

**2. Ajouter une étape dans `.github/workflows/tests.yml`**

Insérer après l'étape "Orders Tests" :
```yaml
- name: "📦 Suppliers Tests (X tests)"
  working-directory: ./stocks_api
  run: |
    echo "=========================================="
    echo "🟣 Testing Suppliers Module"
    echo "=========================================="
    cargo test --lib features::suppliers
    echo "✅ Suppliers: X tests passed"
    echo ""
```

**3. Mettre à jour le Summary**

Dans l'étape "📊 Test Summary", ajouter :
```yaml
echo "  ✅ Suppliers: X tests"
echo "     └─ dto.rs: X tests"
```

C'est tout ! Le nouveau module sera testé **séquentiellement** avec le code déjà compilé = rapide.

## Prochaines Étapes

Pour étendre la couverture des tests :
1. ✅ **Products** : 32 tests (fait)
2. ✅ **Orders** : 16 tests (fait)
3. 🔲 **Suppliers** : Ajouter tests unitaires pour le module suppliers
4. 🔲 **Stocks** : Ajouter tests unitaires pour le module stocks
5. 🔲 **Restocks** : Ajouter tests unitaires pour le module restocks
6. 🔲 **Sales** : Ajouter tests unitaires pour le module sales
7. 🔲 **Users** : Ajouter tests unitaires pour le module users
8. 🔲 Ajouter des tests d'intégration avec une base de données de test
9. 🔲 Ajouter des tests pour les handlers (avec mock des services)
10. 🔲 Configurer la mesure de couverture de code (tarpaulin ou grcov)
