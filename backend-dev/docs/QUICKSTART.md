# Quick Start - Testing Guide

Get everything running and see AI results in under 5 minutes.

---

## 1. Start Everything

```bash
docker compose down -v          # Clean slate (removes old data)
docker compose up --build -d    # Build & start all services
```

Wait ~30 seconds for migrations, seeding, and services to start. Check status:

```bash
docker compose ps
```

You should see `db`, `web`, `ai-service`, and `pgadmin` running. `migrate` and `seed` will have exited (that's normal).

Check the AI service logs to see the models running:

```bash
docker compose logs -f ai-service
```

Wait until you see "All handlers completed" (~2 minutes). Press Ctrl+C to stop following logs.

---

## 2. Login & Get a Token

```bash
curl -s -X POST http://localhost:8090/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@example.com","password":"adminpass"}'
```

Copy the token from the response. For convenience, save it:

```bash
# Linux/Mac:
TOKEN=$(curl -s -X POST http://localhost:8090/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"admin@example.com","password":"adminpass"}' | python3 -c "import sys,json; print(json.loads(sys.stdin.read())['data']['token'])")

# Windows PowerShell:
$response = Invoke-RestMethod -Uri "http://localhost:8090/auth/login" -Method POST -ContentType "application/json" -Body '{"email":"admin@example.com","password":"adminpass"}'
$TOKEN = $response.data.token

# Windows CMD:
# Run the curl command manually and copy-paste the token value
```

---

## 3. Explore the API

### Swagger UI (interactive)
Open in your browser: **http://localhost:8090/swagger-ui**

Click "Authorize" at the top, paste your token, and you can test every endpoint interactively.

### pgAdmin (database browser)
Open: **http://localhost:5050**
- Email: `admin@admin.com`
- Password: `admin`
- Add server: host=`db`, port=`5432`, user=`user`, password=`pass`, database=`stocks`

---

## 4. See AI Results

### Alert Dashboard
```bash
# Summary: how many alerts by status and severity
curl -s http://localhost:8090/alerts/summary \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# List critical alerts
curl -s "http://localhost:8090/alerts?severity=critical&limit=5" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# List all unread alerts
curl -s "http://localhost:8090/alerts?status=new&limit=10" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
```

### Demand Forecasts
```bash
# All forecasts
curl -s "http://localhost:8090/ai/forecasts?limit=5" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# Forecast for product 1
curl -s http://localhost:8090/ai/forecasts/1 \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# Products that need restocking urgently
curl -s http://localhost:8090/ai/urgent-restocks \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
```

### Product Classifications (ABC-XYZ)
```bash
# All classifications
curl -s "http://localhost:8090/ai/classifications?limit=10" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# Classification for product 1
curl -s http://localhost:8090/ai/classifications/1 \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
```

### Product Clusters
```bash
curl -s "http://localhost:8090/ai/clusters?limit=10" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
```

### Supplier Scores
```bash
# All supplier scores
curl -s http://localhost:8090/ai/supplier-scores \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# Score for supplier 1
curl -s http://localhost:8090/ai/supplier-scores/1 \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
```

### Price Suggestions
```bash
curl -s "http://localhost:8090/ai/price-suggestions?limit=10" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
```

### Price Anomalies (only flagged ones)
```bash
curl -s "http://localhost:8090/ai/price-anomalies?only_anomalies=true&limit=10" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
```

### Sales Anomalies (only flagged ones)
```bash
curl -s "http://localhost:8090/ai/sales-anomalies?only_anomalies=true&limit=10" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
```

---

## 5. Manage Alerts

### Acknowledge an alert
```bash
curl -s -X PUT http://localhost:8090/alerts/1/status \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"status":"acknowledged"}' | python3 -m json.tool
```

### Bulk acknowledge multiple alerts
```bash
curl -s -X PUT http://localhost:8090/alerts/bulk-status \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"ids":[1,2,3,4,5],"status":"acknowledged"}' | python3 -m json.tool
```

### Resolve an alert
```bash
curl -s -X PUT http://localhost:8090/alerts/1/status \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"status":"resolved"}' | python3 -m json.tool
```

### Clean up old resolved/dismissed alerts
```bash
curl -s -X DELETE "http://localhost:8090/alerts/cleanup?older_than_days=7" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
```

---

## 6. See Product KPIs (with AI enrichment)

```bash
# Predictions & alerts KPI (includes AI forecast data)
curl -s "http://localhost:8090/products/1/kpis/predictions-alerts" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# Scoring & classification KPI (includes AI classification + cluster)
curl -s "http://localhost:8090/products/1/kpis/scoring-classification" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# Price evolution chart data
curl -s "http://localhost:8090/products/1/kpis/price-evolution" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
```

---

## 7. Trigger AI Models Manually

```bash
# Re-run all AI models right now
curl -s -X POST http://localhost:8001/ai/run | python3 -m json.tool

# Check AI service health
curl -s http://localhost:8001/ai/health | python3 -m json.tool

# See last run metrics (timing, success/failure counts)
curl -s http://localhost:8001/ai/status | python3 -m json.tool
```

---

## 8. Global Dashboard Data

```bash
# Overall performance
curl -s "http://localhost:8090/kpis/global-performance?start_date=2024-01-01&end_date=2026-12-31" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# Top and flop products
curl -s "http://localhost:8090/kpis/top-flop?start_date=2024-01-01&end_date=2026-12-31" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# ABC distribution
curl -s "http://localhost:8090/kpis/abc-distribution?start_date=2024-01-01&end_date=2026-12-31" \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool

# Stock status summary
curl -s http://localhost:8090/stocks/summary \
  -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
```

---

## 9. View Logs

```bash
# Rust API logs
docker compose logs web

# AI service logs (model execution details)
docker compose logs ai-service

# Database logs
docker compose logs db

# Follow all logs in real-time
docker compose logs -f
```

---

## 10. Stop & Clean Up

```bash
# Stop everything (keep data)
docker compose down

# Stop and delete all data (clean restart)
docker compose down -v
```

---

## Windows-Specific Notes

If you're on Windows CMD (not PowerShell or Git Bash), replace:
- Single quotes `'` with double quotes `"` in curl commands
- Escape inner double quotes: `\"` instead of `"`
- Replace `python3` with `python`
- Replace `$TOKEN` with `%TOKEN%`

Example for Windows CMD:
```cmd
curl -s -X POST http://localhost:8090/auth/login -H "Content-Type: application/json" -d "{\"email\":\"admin@example.com\",\"password\":\"adminpass\"}"
```

Or just use the Swagger UI at http://localhost:8090/swagger-ui - it works the same everywhere.
