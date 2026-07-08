# Changer de modèle LLM

Le code est conçu pour que swapper le modèle soit **une simple modification
d'environnement** sans aucun changement de Python. Il suffit de pointer
`GROQ_MODEL` ou `MISTRAL_MODEL` sur un autre identifiant supporté par le
provider correspondant.

## Groq — changer de modèle

Par défaut : `llama-3.3-70b-versatile`. Alternatives utiles :

| Modèle | Tokens/s | Contexte | Quand l'utiliser |
|---|---|---|---|
| `llama-3.3-70b-versatile` (défaut) | ~500 | 128 k | Meilleur compromis qualité/vitesse |
| `llama-3.1-8b-instant` | ~750 | 128 k | Rapide, économiser le rate-limit du 70B |
| `mixtral-8x7b-32768` | ~600 | 32 k | Alternatives européennes |
| `gemma2-9b-it` | ~500 | 8 k | Compact, français correct |

Pour switcher, dans `docker-compose.yml` (ou `.env`) :

```yaml
GROQ_MODEL: "llama-3.1-8b-instant"
```

Puis :

```sh
docker compose up -d --no-deps ai-service
```

Aucun téléchargement, aucune migration — le modèle est côté Groq.

## Mistral — changer de modèle

Par défaut : `mistral-small-latest`. Alternatives :

| Modèle | Latence | Quand l'utiliser |
|---|---|---|
| `mistral-small-latest` (défaut) | 1–3 s | Fallback standard |
| `ministral-3b-latest` | < 1 s | Ultra rapide, français correct |
| `mistral-large-latest` | 2–5 s | Qualité max (payant) |
| `open-mistral-7b` | 1–2 s | Open-weight |

```yaml
MISTRAL_MODEL: "ministral-3b-latest"
```

## Changer le provider primaire

Défaut : Groq en primaire, Mistral en fallback.

Pour inverser (par ex. pour économiser la quota Groq) :

```yaml
LLM_PROVIDER: "mistral"
```

L'auto-fallback continuera de basculer vers l'autre provider en cas de 429.

## Mesurer après switch

```sh
curl -s http://localhost:8001/llm/health | jq
```

Cela renvoie la liste des providers, celui par défaut, et si les clés sont
détectées.

Puis re-lancer l'eval pour mesurer :

```sh
python scripts/chat_eval_panel.py --provider groq
python scripts/chat_eval_panel.py --provider mistral
```

## Ajouter un nouveau provider

Voir [modeles.md § 9](modeles.md#9-ajouter-un-nouveau-provider) — compter
~120 lignes calquées sur `groq_provider.py`.
