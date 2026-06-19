# 🤖 Route API pour l'IA - Documentation

## 📋 Vue d'ensemble

J'ai créé une nouvelle route API dédiée qui agrège toutes les données pertinentes de votre système de gestion de stock en **un seul appel**. Cette route est optimisée pour alimenter votre assistant IA avec des informations complètes et structurées.

## 🚀 Endpoint

```
GET /ai/insights
```

### Authentification
⚠️ **Route protégée** - Nécessite un token JWT dans le header :
```
Authorization: Bearer <votre-token-jwt>
```

## 📊 Paramètres (optionnels)

| Paramètre | Type | Description | Défaut |
|-----------|------|-------------|---------|
| `start_date` | Date (YYYY-MM-DD) | Date de début d'analyse | 30 jours avant aujourd'hui |
| `end_date` | Date (YYYY-MM-DD) | Date de fin d'analyse | Aujourd'hui |

### Exemple d'utilisation

```bash
# Sans paramètres (30 derniers jours par défaut)
curl -H "Authorization: Bearer YOUR_TOKEN" \
  http://localhost:8080/ai/insights

# Avec période personnalisée
curl -H "Authorization: Bearer YOUR_TOKEN" \
  "http://localhost:8080/ai/insights?start_date=2026-01-01&end_date=2026-01-23"
```

## 📦 Structure de la réponse

La réponse contient **8 sections principales** :

```json
{
  "status": "success",
  "message": "Données pour analyse IA récupérées avec succès",
  "data": {
    "global_performance": { /* Performance globale */ },
    "top_flop": { /* Top et flop produits */ },
    "stock_alerts": [ /* Alertes stocks */ ],
    "stock_summary": { /* Résumé des stocks */ },
    "trends": { /* Tendances */ },
    "category_analysis": { /* Analyse par catégorie */ },
    "forecast": { /* Prévisions */ },
    "catalog_health": { /* Santé du catalogue */ },
    "period": { /* Info sur la période */ }
  }
}
```

## 🔍 Détail des données

### 1. **global_performance** - Performance Globale
```json
{
  "total_revenue": 125000.50,
  "total_profit": 45000.25,
  "avg_margin_rate": 36.5,
  "total_products_count": 250,
  "active_products_count": 220,
  "inactive_products_count": 25,
  "discontinued_products_count": 5,
  "total_orders_count": 1500,
  "global_avg_basket_value": 83.33,
  "total_stock_value_cost": 75000.00,
  "total_stock_value_potential": 150000.00
}
```

### 2. **top_flop** - Produits les Plus/Moins Vendus
```json
{
  "top_10_by_revenue": [
    {
      "product_id": 42,
      "product_name": "Laptop Pro X",
      "category": "Electronics",
      "value": 25000.00
    }
  ],
  "top_10_by_profit": [...],
  "top_10_by_volume": [...],
  "top_10_by_turnover": [...],
  "flop_10_by_sales": [...],
  "flop_10_by_profit": [...],
  "at_risk_products": [...]
}
```

### 3. **stock_alerts** - Alertes Stocks
```json
[
  {
    "product": {
      "id": 15,
      "name": "Mouse Wireless",
      "category": "Accessories",
      "reference": "MW-001",
      "stock_quantity": 3,
      "buying_price": 15.50
    },
    "alert_type": "critical_low",
    "severity": "high",
    "message": "Stock critically low - immediate attention needed"
  }
]
```

### 4. **stock_summary** - Résumé des Stocks
```json
{
  "total_products": 250,
  "out_of_stock_count": 12,
  "low_stock_count": 35,
  "overstock_count": 8,
  "total_stock_value": 75000.00,
  "categories_affected": ["Electronics", "Accessories", "Furniture"]
}
```

### 5. **trends** - Tendances
```json
{
  "revenue_growth_percent": 15.5,
  "profit_growth_percent": 12.3,
  "orders_growth_percent": 8.7,
  "basket_value_growth_percent": 6.2,
  "global_trend": "increasing",
  "seasonality_detected": false
}
```

### 6. **category_analysis** - Analyse par Catégorie
```json
{
  "by_category": [
    {
      "category": "Electronics",
      "revenue": 85000.00,
      "profit": 32000.00,
      "avg_margin_rate": 37.6,
      "products_count": 85,
      "avg_turnover_rate": 2.5,
      "stock_distribution_percent": 45.2
    }
  ],
  "top_5_by_revenue": [...],
  "top_5_by_profit": [...],
  "top_5_by_volume": [...]
}
```

### 7. **forecast** - Prévisions
```json
{
  "forecasted_revenue_next_month": 42000.00,
  "forecasted_revenue_next_3_months": 126000.00,
  "cash_needed_for_restocks": 15000.00,
  "predicted_stockouts_count": 8,
  "optimization_opportunities_count": 15
}
```

### 8. **catalog_health** - Santé du Catalogue
```json
{
  "availability_rate": 88.5,
  "stockout_products_count": 12,
  "discontinued_products_count": 5,
  "catalog_renewal_rate": 8.5,
  "low_rotation_products_percent": 15.2,
  "obsolete_products_percent": 6.8,
  "overstocked_products_percent": 3.2
}
```

### 9. **period** - Informations sur la Période
```json
{
  "start_date": "2025-12-24",
  "end_date": "2026-01-23",
  "days_count": 30
}
```

## 💡 Exemples d'utilisation avec l'IA

### JavaScript/TypeScript
```typescript
async function getAIInsights() {
  const response = await fetch('http://localhost:8080/ai/insights', {
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json'
    }
  });
  
  const { data } = await response.json();
  
  // Envoyer à votre IA
  const aiPrompt = `
    Analyse ces données de gestion de stock et donne-moi des insights :
    
    Performance globale : CA de ${data.global_performance.total_revenue}€, 
    avec ${data.global_performance.total_orders_count} commandes.
    
    Tendances : ${data.trends.global_trend} avec une croissance de 
    ${data.trends.revenue_growth_percent}%
    
    Alertes : ${data.stock_alerts.length} produits nécessitent une attention
    
    Top produits : ${JSON.stringify(data.top_flop.top_10_by_revenue.slice(0, 3))}
    
    Que recommandes-tu pour optimiser mes ventes et mon stock ?
  `;
  
  return aiPrompt;
}
```

### Python
```python
import requests
import json

def get_ai_insights(token: str, start_date: str = None, end_date: str = None):
    url = "http://localhost:8080/ai/insights"
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json"
    }
    
    params = {}
    if start_date:
        params['start_date'] = start_date
    if end_date:
        params['end_date'] = end_date
    
    response = requests.get(url, headers=headers, params=params)
    data = response.json()['data']
    
    # Préparer pour l'IA
    ai_context = {
        "revenue": data['global_performance']['total_revenue'],
        "top_products": data['top_flop']['top_10_by_revenue'][:5],
        "alerts": len(data['stock_alerts']),
        "trend": data['trends']['global_trend'],
        "growth": data['trends']['revenue_growth_percent']
    }
    
    return ai_context
```

## 🎯 Cas d'usage pour l'IA

### 1. Analyse de performance
```
"Voici les données de mon entreprise : [data]. 
Identifie les forces et faiblesses de ma gestion de stock."
```

### 2. Recommandations d'achat
```
"Avec ces alertes stocks [stock_alerts] et ces prévisions [forecast],
quels produits dois-je commander en priorité ?"
```

### 3. Optimisation des prix
```
"Analyse mes marges [category_analysis] et mes top/flop [top_flop].
Quels produits devrais-je promouvoir ou réduire ?"
```

### 4. Détection d'anomalies
```
"Regarde ces données [all data]. Y a-t-il des anomalies ou 
des tendances inquiétantes ?"
```

### 5. Rapport automatique
```
"Génère un rapport exécutif basé sur ces KPIs pour la période 
du [period.start_date] au [period.end_date]."
```

## ⚡ Optimisations

### Performance
- Toutes les requêtes SQL sont exécutées **en parallèle** (`tokio::try_join!`)
- Temps de réponse typique : **< 2 secondes**
- Données fraîches et en temps réel

### Sécurité
- Route protégée par JWT
- Validation automatique des paramètres
- Gestion d'erreurs robuste

## 🔧 Intégration avec d'autres routes

Si vous avez besoin de données plus spécifiques, voici les routes complémentaires :

### Routes KPIs détaillées
```
GET /kpis/global-performance
GET /kpis/top-flop
GET /kpis/trends
GET /kpis/category-analysis
GET /kpis/supplier-analysis
GET /kpis/catalog-health
GET /kpis/abc-distribution
GET /kpis/operational-efficiency
GET /kpis/price-analysis
GET /kpis/forecast
GET /kpis/time-series
```

### Routes Stocks
```
GET /stocks/out-of-stock
GET /stocks/low-stock
GET /stocks/soon-out-of-stock
GET /stocks/overstock
GET /stocks/alerts
GET /stocks/summary
```

### Routes Produits
```
GET /products
GET /products/:id
GET /products/:id/kpis/pricing-margin
GET /products/:id/kpis/sales-rotation
GET /products/:id/kpis/profitability
```

## 📝 Notes importantes

1. **Période par défaut** : 30 derniers jours si non spécifiée
2. **Format des dates** : YYYY-MM-DD (ISO 8601)
3. **Token JWT** : Obligatoire pour toutes les requêtes
4. **Limites** : 
   - 100 alertes stocks maximum
   - Top/Flop limités à 10 produits chacun
5. **Documentation Swagger** : Disponible sur `http://localhost:8080/swagger-ui`

## 🚀 Prochaines étapes

Pour utiliser cette route avec votre IA :

1. **Obtenir un token JWT** via `/auth/login`
2. **Faire un appel à** `/ai/insights`
3. **Formater les données** pour votre prompt IA
4. **Envoyer à votre modèle** (GPT-4, Claude, etc.)
5. **Présenter les insights** à l'utilisateur

## 💬 Exemple de prompt complet pour l'IA

```
Tu es un expert en gestion de stock et analyse business. 
Voici les données complètes de mon entreprise :

PÉRIODE ANALYSÉE : Du {period.start_date} au {period.end_date} ({period.days_count} jours)

PERFORMANCE GLOBALE :
- Chiffre d'affaires : {global_performance.total_revenue}€
- Profit : {global_performance.total_profit}€
- Marge moyenne : {global_performance.avg_margin_rate}%
- Commandes : {global_performance.total_orders_count}
- Panier moyen : {global_performance.global_avg_basket_value}€

TENDANCES :
- Croissance CA : {trends.revenue_growth_percent}%
- Tendance : {trends.global_trend}

TOP PRODUITS (par CA) :
{top_flop.top_10_by_revenue (top 5)}

ALERTES STOCKS :
{stock_alerts.length} produits nécessitent une attention
- Ruptures : {stock_summary.out_of_stock_count}
- Stock bas : {stock_summary.low_stock_count}
- Surstock : {stock_summary.overstock_count}

PRÉVISIONS :
- CA prévu mois prochain : {forecast.forecasted_revenue_next_month}€
- Ruptures prévues : {forecast.predicted_stockouts_count}
- Opportunités d'optimisation : {forecast.optimization_opportunities_count}

QUESTIONS :
1. Quelle est ta analyse générale de la santé de mon entreprise ?
2. Quels sont les 3 points d'attention prioritaires ?
3. Quelles actions concrètes recommandes-tu ?
4. Y a-t-il des opportunités de croissance que je devrais saisir ?
```

---

**Créé par** : Assistant IA  
**Date** : 23 janvier 2026  
**Version API** : 1.0.0
