# Chatbot StockS — Documentation

Cette section regroupe **toute la documentation du chatbot conversationnel**
intégré à la plateforme StockS.

## Table des matières

| Document | Pour qui | Sujet |
|---|---|---|
| [que-puis-je-demander.md](que-puis-je-demander.md) | Tout le monde | **Catalogue complet** de tout ce qu'on peut demander au bot — 33 outils par thème, avec exemples de questions, et ce qu'il ne fait pas |
| [comment-ca-marche.md](comment-ca-marche.md) | Tout le monde | Fonctionnement interne (flux, RAG, agent, outils) — comment une question devient une réponse |
| [modeles.md](modeles.md) | Tech / ops | LLM utilisés (Mistral, Groq, local), critères de choix, alternatives |
| [ressources.md](ressources.md) | Ops / déploiement | Budget RAM/CPU/disque, dimensionnement, dégradation gracieuse |
| [frontend.md](frontend.md) | Devs front | Comment connecter une UI au backend (REST, JWT, états à gérer) |
| [cloud-deploiement.md](cloud-deploiement.md) | Ops | Déploiement sur VPS / cloud, variables d'env, secrets, sécurité |
| [quickstart.md](quickstart.md) | Tout le monde | Démarrage rapide (docker compose + smoke test) |
| [status.md](status.md) | Tout le monde | État actuel + vérifications live + caveats connus |
| [improvements-2026-05-09.md](improvements-2026-05-09.md) | Tout le monde | Améliorations apportées le 2026-05-09 (raccourcis, cache, reranker, etc.) |
| [evaluation-2026-05-15.md](evaluation-2026-05-15.md) | Tech | Résultats de la campagne d'évaluation 2026-05-15 (baseline 64 % → 91 % après fixes ciblés), bugs trouvés, performance observée |
| [evaluation-panel.md](evaluation-panel.md) | Tech | **Panel utilisateur 100 %** (15/15) — questions concrètes : "prix du produit le plus vendu", "presque en rupture", "stock trop élevé", "livraisons en attente", etc. |
| [changer-de-modele.md](changer-de-modele.md) | Ops | Comment switcher de modèle local (1.5B → 3B → 7B → Phi-3.5 → API) avec budget RAM 5 GB |
| [ameliorations-2026-05-21.md](ameliorations-2026-05-21.md) | Tech | Round d'optimisations : prompt caching, shortcuts sémantiques, mémoire d'entités, pré-résumé des payloads (KV quant testé puis abandonné) |
| [ameliorations-2026-05-22.md](ameliorations-2026-05-22.md) | Tech | Briefing proactif, comparaison temporelle, garde-fou anti-hallucination, streaming SSE |

## Lecture conseillée selon ton rôle

- **Je veux savoir ce que je peux demander au bot** → [que-puis-je-demander.md](que-puis-je-demander.md)
- **Je découvre le projet** → [comment-ca-marche.md](comment-ca-marche.md) puis [quickstart.md](quickstart.md)
- **Je veux brancher un frontend** → [frontend.md](frontend.md)
- **Je dois le mettre en production** → [ressources.md](ressources.md) puis [cloud-deploiement.md](cloud-deploiement.md)
- **Je veux changer le modèle local** → [modeles.md](modeles.md)
- **Je veux comprendre ce qui a changé récemment** → [improvements-2026-05-09.md](improvements-2026-05-09.md)
