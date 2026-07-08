# Déploiement cloud / VPS

Guide pour mettre le chatbot en production sur un serveur distant. Couvre
le VPS bare-metal (Hetzner, OVH, Scaleway), le cloud (AWS, GCP, Azure),
et les éléments à durcir avant l'ouverture publique.

## 1. Choix du serveur

### Configuration minimale recommandée

| Élément | Valeur |
|---|---|
| RAM | **4 GB** (l'inférence LLM est déléguée à Groq/Mistral) |
| CPU | 2 cœurs |
| Disque | **10 GB SSD libre** |
| Bande passante | 100 Mbps suffisent largement |
| OS | Ubuntu 22.04 / 24.04 LTS, Debian 12 |

### Fournisseurs avec un bon rapport qualité/prix (2026)

| Fournisseur | Offre exemple | Prix indicatif | Notes |
|---|---|---|---|
| **Hetzner Cloud** | CPX21 (3 vCPU AMD, 4 GB, 80 GB SSD) | ~6 €/mois | **Recommandé** |
| **OVH VPS** | VPS Value (2 vCPU, 4 GB, 80 GB) | ~7 €/mois | France |
| **Scaleway** | DEV1-M (3 vCPU, 4 GB, 40 GB SSD) | ~10 €/mois | France |
| **Contabo** | VPS S (4 vCPU, 8 GB, 200 GB) | ~6 €/mois | Bon rapport mais réseau parfois lent |

## 2. Préparation du serveur

### Mise à jour + outils de base

```sh
sudo apt update && sudo apt upgrade -y
sudo apt install -y curl git unzip ca-certificates ufw fail2ban jq
```

### Installer Docker (depuis le repo officiel)

```sh
# Installer Docker Engine + Compose plugin
curl -fsSL https://get.docker.com | sudo sh
sudo systemctl enable --now docker

# Autoriser ton user à utiliser docker sans sudo (relogue après)
sudo usermod -aG docker $USER
```

Vérifie :

```sh
docker --version
docker compose version
```

### Configurer le firewall (UFW)

```sh
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw allow 22/tcp           # SSH
sudo ufw allow 80/tcp           # HTTP (pour Let's Encrypt)
sudo ufw allow 443/tcp          # HTTPS
sudo ufw enable
```

**Ne JAMAIS ouvrir** les ports `5432` (Postgres), `8001` (ai-service), ou
`8090` (API Rust) directement sur Internet. Tout passe par le reverse
proxy en 443.

## 3. Récupération du code + configuration

```sh
git clone <votre-repo> stocks-api
cd stocks-api
git checkout chatbot       # ou main une fois mergé
cp .env.chatbot.example .env
```

Édite `.env` :

```env
# JWT (générer un secret long)
JWT_SECRET=<openssl rand -hex 32>

# Provider LLM (Groq primaire, Mistral en fallback)
LLM_PROVIDER=groq

# Clé Groq requise (Llama-3.3-70B, free tier)
GROQ_API_KEY=ta-clef-groq

# Optionnel : Mistral pour l'auto-fallback si Groq 429
MISTRAL_API_KEY=ta-clef-mistral

# Database
POSTGRES_PASSWORD=<openssl rand -hex 16>
```

> Dans `docker-compose.yml`, remplace les mots de passe hardcodés (`pass`,
> `zf2nrOa8...`) par des références `${POSTGRES_PASSWORD}` et
> `${JWT_SECRET}` pour qu'ils soient lus depuis `.env`. Voir section 7
> sur les secrets.

## 4. Premier démarrage

```sh
# Build des images (~2-5 min)
docker compose build

# Démarrer tout
docker compose up -d

# Suivre les logs jusqu'à voir "AI Service starting..."
docker compose logs -f ai-service
```

Au premier démarrage :

1. PostgreSQL initialise (~5 s)
2. Migrate applique V001 + V002 + V003 (~2 s)
3. Seed insère les données démo (~5-10 min — bcrypt-hash de 850 users)
4. Web démarre (instantané)
5. ai-service démarre → indexe le RAG (~30 s) → prêt à servir les requêtes
   (aucun modèle LLM à charger localement)
6. Stack prête

Vérification rapide :

```sh
curl http://localhost:8090/health      # → "OK"
curl http://localhost:8001/llm/health   # → JSON avec providers disponibles
```

## 5. Reverse proxy + HTTPS (Caddy ou Nginx)

L'API Rust écoute sur `0.0.0.0:8090` mais on **ne l'expose jamais
directement** en prod. Un reverse proxy gère HTTPS + termine TLS + ajoute
des headers de sécurité.

### Option A — Caddy (le plus simple, HTTPS automatique)

Installe Caddy :

```sh
sudo apt install -y debian-keyring debian-archive-keyring apt-transport-https
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list
sudo apt update && sudo apt install -y caddy
```

Édite `/etc/caddy/Caddyfile` :

```
api.tondomaine.fr {
    reverse_proxy localhost:8090

    # Headers de sécurité
    header {
        Strict-Transport-Security "max-age=31536000; includeSubDomains"
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
        Referrer-Policy "strict-origin-when-cross-origin"
    }
}
```

Reload :

```sh
sudo systemctl reload caddy
```

Caddy obtient automatiquement un certificat Let's Encrypt et le renouvelle.
Aucune autre action requise.

### Option B — Nginx + Certbot

```nginx
# /etc/nginx/sites-available/stocks-api
server {
    listen 443 ssl http2;
    server_name api.tondomaine.fr;

    ssl_certificate     /etc/letsencrypt/live/api.tondomaine.fr/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.tondomaine.fr/privkey.pem;

    location / {
        proxy_pass         http://localhost:8090;
        proxy_set_header   Host $host;
        proxy_set_header   X-Real-IP $remote_addr;
        proxy_set_header   X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Proto $scheme;
        # Important pour les longues réponses LLM
        proxy_read_timeout 300s;
    }
}

server {
    listen 80;
    server_name api.tondomaine.fr;
    return 301 https://$server_name$request_uri;
}
```

Certbot pour le certificat :

```sh
sudo apt install certbot python3-certbot-nginx
sudo certbot --nginx -d api.tondomaine.fr
```

## 6. Configurer CORS pour ton domaine frontend

Dans `stocks_api/src/bin/server.rs`, ajoute ton domaine de prod dans la
liste des origines autorisées :

```rust
let cors = CorsLayer::new()
    .allow_origin([
        "https://stock-s.fr".parse().unwrap(),
        "https://app.stock-s.fr".parse().unwrap(),  // ← ton frontend
    ])
    ...
```

Puis rebuilde l'image web :

```sh
docker compose build web && docker compose up -d --no-deps web
```

## 7. Gestion des secrets

**Ne JAMAIS commit `.env`** (déjà dans `.gitignore`).

Options pour les secrets en prod :

### Simple — fichier `.env` sur le serveur

- Permissions strictes : `chmod 600 .env`
- Ne pas le sauvegarder dans un backup non chiffré

### Mieux — Docker secrets

Pour Docker Swarm uniquement. Si tu veux scaler.

### Encore mieux — gestionnaire de secrets

- HashiCorp Vault
- AWS Secrets Manager / GCP Secret Manager
- Bitwarden Secrets Manager (gratuit jusqu'à un certain seuil)

Pour un projet seul de cette taille, le fichier `.env` avec permissions
strictes + backups chiffrés suffit largement.

## 8. Sauvegardes

### Postgres

```sh
# Backup quotidien dans /backups
docker exec backend-db-1 pg_dump -U user stocks | gzip > /backups/stocks-$(date +%F).sql.gz
```

À automatiser via cron :

```sh
sudo crontab -e
# Ajouter :
0 3 * * * docker exec backend-db-1 pg_dump -U user stocks | gzip > /backups/stocks-$(date +\%F).sql.gz
0 4 * * * find /backups -name "stocks-*.sql.gz" -mtime +14 -delete
```

### Volumes Docker (embeddings)

Pas besoin de backup : le cache HuggingFace se retéléchargera au premier boot.

## 9. Surveillance / observabilité

### Logs

`docker compose logs -f ai-service` en live. Pour archiver :

```sh
docker compose logs --no-log-prefix --since 24h ai-service > /var/log/ai-service-$(date +%F).log
```

### Métriques basiques

```sh
docker stats --no-stream
```

Pour une vue plus riche, déployer **Prometheus + Grafana + cAdvisor** dans
le même compose (1 GB de RAM en plus).

### Alertes

Si la RAM de ai-service dépasse 2 GB, le reranker est probablement mal
libéré ou un cache HuggingFace grossit trop. Configurer une alerte
mail / Slack sur `docker stats`.

## 10. Mises à jour applicatives

```sh
cd stocks-api
git pull
docker compose build
docker compose up -d
```

Les migrations s'appliquent automatiquement au démarrage de `migrate`.

Si une nouvelle migration ajoute une colonne `NOT NULL` sans `DEFAULT`,
**downtime potentiel**. Vérifier les SQL de migration avant un déploiement
en heure de pointe.

## 11. Mises à jour Docker / OS

```sh
# Updates de sécurité automatiques
sudo apt install unattended-upgrades
sudo dpkg-reconfigure unattended-upgrades

# Docker
sudo apt update && sudo apt upgrade docker-ce docker-ce-cli containerd.io
```

## 12. Sécurité — checklist avant l'ouverture publique

- [ ] `JWT_SECRET` long et aléatoire, **différent du dev**
- [ ] `POSTGRES_PASSWORD` long et aléatoire, **différent du dev**
- [ ] `GROQ_API_KEY` présente (requise) ; `MISTRAL_API_KEY` pour fallback
- [ ] UFW activé, seul `22/80/443` ouvert
- [ ] SSH par clé seulement, désactiver `PasswordAuthentication`
- [ ] `fail2ban` actif pour bloquer les brute-force SSH
- [ ] HTTPS forcé via reverse proxy (HSTS activé)
- [ ] CORS limité aux domaines frontend réels (pas de `*`)
- [ ] pgadmin **retiré** ou protégé derrière un VPN / IP whitelist
- [ ] Rate-limiting au niveau reverse proxy (par IP, par utilisateur)
- [ ] Logs Docker en rotation (`max-size` + `max-file` dans daemon.json)
- [ ] Backup Postgres quotidien testé (restauration validée au moins une
      fois)

## 13. Désactiver pgadmin en prod

Dans `docker-compose.yml`, soit **supprimer la section `pgadmin`**, soit
binder son port en localhost only :

```yaml
ports:
  - "127.0.0.1:5050:80"      # Au lieu de "5050:80"
```

Ainsi pgadmin n'est joignable qu'en SSH tunnel (`ssh -L 5050:localhost:5050 user@server`).

## 14. Désactiver les batch jobs au démarrage en prod

`RUN_ON_STARTUP=true` est utile en dev mais en prod, ça refait toute la
charge ML à chaque restart, ce qui peut prendre 15 minutes. À mettre à
`false` et laisser le cron quotidien (`CRON_SCHEDULE`) faire son travail.

## 15. Scaling — si la charge augmente

Le bot est conçu pour un usage interne (< 100 utilisateurs concurrents).
Si tu dépasses :

1. **Augmenter le nombre de workers Flask** : ajouter `gunicorn` devant
   Flask avec 2-4 workers. Permet de servir plusieurs requêtes en parallèle.
2. **Réplique horizontale** : 2-3 instances d'ai-service derrière un load
   balancer. Postgres reste partagée.
3. **Cache plus agressif** : baisser `CHAT_CACHE_THRESHOLD` pour absorber
   plus de variantes de questions.
4. **Modèle Groq plus rapide** : passer à `llama-3.1-8b-instant` pour
   absorber plus de RPM (plus rapide, moins de tokens/s consommés).
5. **Upgrader vers un plan Groq/Mistral payant** si les free tiers ne
   suffisent plus.

## 16. Coûts opérationnels indicatifs

Pour un déploiement type :

| Poste | Coût mensuel |
|---|---|
| VPS Hetzner CPX21 (4 GB) | ~6 € |
| Nom de domaine + DNS | ~1 € |
| Backups stockés ailleurs (S3 / B2) | ~1 € |
| Groq API (plan gratuit, 1M tok/jour Llama-3.3-70B) | 0 € |
| Mistral API (plan gratuit, fallback) | 0 € |
| **Total** | **~8 €/mois** |

Si la charge grimpe et qu'on dépasse le free tier Groq (~30 req/min,
1M tok/jour), on peut basculer sur le plan payant (~0.6 $/M tokens input,
~0.8 $/M output). Pour 100 utilisateurs actifs faisant 30 messages/jour de
~1500 tokens (in+out), on parle de ~135 M tokens/mois → ~85 €/mois.
