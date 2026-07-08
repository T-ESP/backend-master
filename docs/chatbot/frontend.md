# Guide d'intégration frontend — Chatbot StockS

Documentation complète pour connecter une UI au chatbot. Indépendante du
framework (React, Vue, Svelte, vanilla JS). L'API est du REST classique + SSE.

---

## 1. Architecture de principe

```
Navigateur / App
      │  HTTP + JWT
      ▼
stocks_api (Rust, :8090)   ← tu parles uniquement à ça
      │  HTTP interne
      ▼
ai-service (Python, :8001) ← invisible pour le front
      │
      ├─ LLM Groq (primaire) / Mistral (fallback)
      ├─ RAG (pgvector)
      └─ 33 outils → PostgreSQL / API interne
```

Le frontend ne connaît que l'API Rust. Toute la logique IA est transparente.

---

## 2. Authentification

Le **même JWT** que le reste de l'application. Toutes les routes `/chat/…`
nécessitent :

```http
Authorization: Bearer <token>
```

JWT valide 24 h. Si `401` → renvoyer vers le login (comportement standard).

---

## 3. Surface API complète

| Méthode | Route | Description |
|---|---|---|
| `GET`    | `/chat/briefing`                          | Briefing proactif à l'ouverture |
| `POST`   | `/chat/sessions`                          | Créer une session |
| `GET`    | `/chat/sessions`                          | Lister les sessions |
| `GET`    | `/chat/sessions/:id`                      | Charger session + messages |
| `DELETE` | `/chat/sessions/:id`                      | Supprimer une session |
| `POST`   | `/chat/sessions/:id/messages`             | Envoyer un message (réponse JSON) |
| `POST`   | `/chat/sessions/:id/messages/stream`      | Envoyer un message (réponse SSE) |
| `POST`   | `/chat/sessions/:id/confirm-action`       | Confirmer ou annuler une action |
| `GET`    | `/chat/sessions/:id/export`               | Exporter en markdown ou JSON |
| `GET`    | `/chat/provider-health`                   | État des fournisseurs LLM |

> Toute la doc Swagger est disponible sur `http://localhost:8090/swagger-ui`
> (onglet **chat**). Tu peux jouer chaque endpoint avec « Try it out » avant
> de coder le moindre composant.

---

## 4. Flux complet — étape par étape

### 4.1 Ouverture de l'interface chat

```
1. GET /chat/briefing         → résumé proactif de l'état du magasin
2. GET /chat/sessions         → liste des conversations existantes
```

**Briefing** — à afficher comme premier message système à l'ouverture (avant
que l'utilisateur ne pose une question) :

```jsonc
// GET /chat/briefing — réponse
{
  "success": true,
  "data": {
    "summary": "Bonjour 👋 Voici l'état de votre magasin :\n- 🔴 12 alerte(s) critique(s)\n- 📦 8 produit(s) à réapprovisionner en urgence\n- ⚠️ 68 produit(s) en stock bas",
    "suggested_questions": [
      "Quelles sont les alertes critiques ?",
      "Quels produits dois-je réapprovisionner en urgence ?",
      "Comment vont nos ventes ce mois ?"
    ]
  }
}
```

Affiche `data.summary` comme bulle « assistant » au lancement. Les
`suggested_questions` peuvent devenir des boutons de démarrage rapide.

### 4.2 Créer une nouvelle session

```http
POST /chat/sessions
Content-Type: application/json

{ "title": null, "provider": "auto" }
```

`provider` : `"auto"` | `"groq"` | `"mistral"`. Avec `"auto"`,
le serveur choisit le meilleur disponible (Groq d'abord, Mistral si indispo).
Garde le `session_id` retourné — tu en as besoin pour tous les appels suivants.

```jsonc
{
  "success": true,
  "data": {
    "session_id": "8c20f4a3-...",
    "title": null,
    "provider": "auto",
    "created_at": "2026-05-22T10:00:00Z",
    "updated_at": "2026-05-22T10:00:00Z"
  }
}
```

### 4.3 Envoyer un message — deux modes

#### Mode REST (simple, recommandé pour démarrer)

```http
POST /chat/sessions/:id/messages
Content-Type: application/json

{ "content": "Quel est le produit le plus vendu cette semaine ?" }
```

Attend la réponse complète (~0,5 s pour les réponses déterministes,
~1-2 s via Groq, ~2-4 s si fallback Mistral).

#### Mode SSE (recommandé en production)

```http
POST /chat/sessions/:id/messages/stream
Content-Type: application/json

{ "content": "Quels sont mes stocks critiques ?" }
```

La réponse est un flux d'événements SSE. Le frontend reçoit des
notifications au fil de la génération — plus fluide pour l'utilisateur.

---

## 5. Format complet de la réponse REST

```jsonc
{
  "success": true,
  "data": {
    // ── Le message de l'assistant ──────────────────────────────
    "assistant_message": {
      "message_id": 42,
      "session_id": "8c20f4a3-...",
      "role": "assistant",
      "content": "**Terreau universel 20L** — produit le plus vendu...",
      "provider": "deterministic",
      "tokens_in": 0,
      "tokens_out": 0,
      "latency_ms": 850,
      "created_at": "2026-05-22T10:01:00Z"
    },

    // ── Action en attente (null si réponse normale) ────────────
    "pending_action": null,
    // ou :
    "pending_action": {
      "action_id": "550e8400-...",
      "session_id": "8c20f4a3-...",
      "tool_name": "create_restock",
      "tool_args": { "product_id": 8, "quantity": 50 },
      "status": "pending",
      "created_at": "2026-05-22T10:01:00Z"
    },

    // ── Suggestions de suivi (nouveau) ─────────────────────────
    "suggestions": [
      {
        "text": "Jusqu'à quand le stock de Terreau universel 20L va durer ?",
        "tool": "get_forecast",
        "args": { "product_id": 8 }
      },
      {
        "text": "CA de Terreau universel 20L sur les 30 derniers jours",
        "tool": "get_product_sales",
        "args": { "product_id": 8, "period_days": 30 }
      },
      {
        "text": "Classification ABC-XYZ de Terreau universel 20L",
        "tool": "get_classification",
        "args": { "product_id": 8 }
      }
    ],

    // ── Métadonnées ────────────────────────────────────────────
    "provider_used": "deterministic",   // groq | mistral | deterministic | cache
    "intent": "data",                   // doc | data | action | chitchat
    "cached": false,
    "shortcut_used": "get_top_product_full",  // ou null
    "numbers_verified": true,           // false = réponse suspecte (chiffres non vérifiés)
    "citations": [],                    // sources RAG — non vide pour intent=doc

    "usage": {
      "tokens_in": 0,
      "tokens_out": 0,
      "latency_ms": 850
    }
  }
}
```

---

## 6. Les suggestions — comment les utiliser

C'est la feature principale pour une UX fluide. Après chaque réponse,
`suggestions[]` contient 2–4 questions de suivi **pré-résolues** : le tool
et les arguments sont déjà remplis. L'utilisateur clique → réponse
instantanée, zéro appel LLM.

### Option A — Boutons qui envoient le texte en chat

Le plus simple : au clic, soumettre `suggestion.text` comme nouveau message
utilisateur via `POST /chat/sessions/:id/messages`. L'infrastructure de
shortcuts le capturera et répondra vite.

```js
function onSuggestionClick(suggestion) {
  sendMessage(suggestion.text);
}
```

### Option B — Appel direct d'outil (réponse instantanée garantie)

Le clic appelle directement `POST /chat/sessions/:id/confirm-action` avec
l'outil pré-résolu. Aucun passage par le LLM, réponse en ~200 ms.

```js
async function onSuggestionClick(suggestion) {
  // Affiche la question comme message utilisateur
  appendUserBubble(suggestion.text);

  // Appel direct d'outil — contourne entièrement le LLM
  const res = await fetch(`/chat/sessions/${sessionId}/confirm-action`, {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${jwt}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({
      action_id: null,       // null = pas une action d'écriture
      decision: 'execute',
      tool_name: suggestion.tool,
      tool_args: suggestion.args
    })
  });
  const data = await res.json();
  appendAssistantBubble(data.data.result_text);
}
```

> Note : l'option B nécessite un endpoint dédié côté backend si tu veux le
> résultat formaté. Pour l'instant l'option A est suffisante — les shortcuts
> se chargeront du cache et de la rapidité.

### Affichage suggéré

```
┌─────────────────────────────────────────────────────┐
│ 🤖  Terreau universel 20L — produit le plus vendu…  │
│     ...                                             │
│                                                     │
│  💡 Questions suivantes :                           │
│  [ Jusqu'à quand dure le stock ? ]                  │
│  [ CA sur 30 jours            ]                     │
│  [ Classification ABC-XYZ     ]                     │
└─────────────────────────────────────────────────────┘
```

---

## 7. Gestion des actions d'écriture (pending_action)

Quand l'utilisateur dit « crée un réappro de 50 unités du produit 8 », la
réponse contient `pending_action` non nul. **L'action n'est PAS encore
exécutée** — le bot attend ta confirmation.

```jsonc
"pending_action": {
  "action_id": "550e8400-e29b-41d4-a716-446655440000",
  "tool_name": "create_restock",
  "tool_args": { "product_id": 8, "quantity": 50 },
  "status": "pending"
}
```

**Affiche deux boutons** sous le message du bot :

```
[ ✅ Confirmer ]   [ ❌ Annuler ]
```

```http
POST /chat/sessions/:id/confirm-action
Content-Type: application/json

{ "action_id": "550e8400-...", "decision": "confirm" }
// ou "decision": "cancel"
```

Réponse si confirmée :
```jsonc
{
  "data": {
    "action_id": "550e8400-...",
    "status": "confirmed",
    "result": { "restock_cree": { "id": 42, ... } },
    "message": "Action exécutée avec succès"
  }
}
```

**Règle d'UX** : désactive les boutons après le premier clic (évite le
double-submit). Affiche un toast « Action exécutée » ou « Action annulée ».

---

## 8. SSE — format des événements

Tous les événements suivent le format SSE standard :

```
data: {"event": "ping", "data": {}}

data: {"event": "intent", "data": {"intent": "data"}}

data: {"event": "shortcut", "data": {"tool": "get_top_product_full"}}

data: {"event": "tool_call", "data": {"tool": "get_product_sales", "args": {...}}}

data: {"event": "cached", "data": {"provider": "cache"}}

data: {"event": "delta", "data": {"content": "**Terreau universel 20L** — ..."}}

data: {"event": "pending_action", "data": {"action_id": "...", "tool_name": "..."}}

data: {"event": "done", "data": {
  "provider_used": "deterministic",
  "intent": "data",
  "numbers_verified": true,
  "shortcut_used": "get_top_product_full",
  "suggestions": [...],
  "citations": [],
  "usage": {"tokens_in": 0, "tokens_out": 0, "latency_ms": 850}
}}
```

### Consommer le SSE en JavaScript

```js
async function sendMessageStream(sessionId, content, jwt) {
  const res = await fetch(`/chat/sessions/${sessionId}/messages/stream`, {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${jwt}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ content })
  });

  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split('\n');
    buffer = lines.pop(); // ligne incomplète

    for (const line of lines) {
      if (!line.startsWith('data: ')) continue;
      const payload = JSON.parse(line.slice(6));

      switch (payload.event) {
        case 'intent':
          showIntentIndicator(payload.data.intent); // icône dans la bulle
          break;
        case 'shortcut':
          showStatus('Réponse directe…');
          break;
        case 'tool_call':
          showStatus(`Consultation de ${payload.data.tool}…`);
          break;
        case 'delta':
          appendToCurrentBubble(payload.data.content); // texte en continu
          break;
        case 'pending_action':
          showConfirmButtons(payload.data);
          break;
        case 'done':
          finalizeBubble(payload.data);            // suggestions, citations…
          break;
      }
    }
  }
}
```

---

## 9. Les 5 cas d'état à gérer

### 9.1 Chargement normal

Dès l'envoi du message, affiche un spinner « L'assistant réfléchit… ».
Avec SSE, remplace par les labels d'événements (`intent`, `tool_call`…).
Désactive le champ de saisie jusqu'à réception de l'événement `done`.

### 9.2 Action en attente

Voir section 7. Afficher les boutons Confirmer / Annuler.

### 9.3 Réponse suspecte — `numbers_verified: false`

Le garde-fou a détecté que des chiffres dans la réponse ne correspondent
pas aux données. Ajoute un avertissement discret :

```
⚠️ Certains chiffres de cette réponse n'ont pas pu être vérifiés.
```

Ne supprime pas la réponse — informe seulement.

### 9.4 Erreur réseau / ai-service down

`502 Bad Gateway` → toast « Service IA temporairement indisponible » +
bouton **Réessayer** qui re-soumet le même message.

---

## 10. Métadonnées à exploiter

### `provider_used`

| Valeur | Affichage suggéré |
|---|---|
| `"deterministic"` | ⚡ Réponse directe |
| `"groq"` | ☁️ Via Groq |
| `"mistral"` | ☁️ Via Mistral (fallback) |
| `"cache"` | 💾 Réponse mise en cache |

### `intent`

| Valeur | Icône |
|---|---|
| `"data"` | 📊 |
| `"doc"` | 📖 |
| `"action"` | ⚙️ |
| `"chitchat"` | 💬 |

### `citations`

Uniquement pour `intent = "doc"` (questions conceptuelles). Affiche les
sources sous le message :

```
📎 Sources : docs/chatbot/comment-ca-marche.md — Classification ABC-XYZ
```

### `numbers_verified`

`false` → badge d'avertissement (voir 9.4).

### `shortcut_used`

`non-null` → la réponse vient d'un outil direct, pas du LLM. Tu peux
afficher « Réponse directe » ou une icône éclair.

---

## 11. Export d'une conversation

```http
GET /chat/sessions/:id/export?format=markdown
// ou ?format=json
```

Le format `markdown` déclenche un `Content-Disposition: attachment` — le
navigateur propose le téléchargement directement. Parfait pour un bouton
« Exporter ».

```js
function exportSession(sessionId) {
  window.location.href = `/chat/sessions/${sessionId}/export?format=markdown`;
}
```

---

## 12. Vérifier l'état des LLM

```http
GET /chat/provider-health
```

```jsonc
{
  "data": {
    "default": "groq",
    "providers": [
      { "name": "groq",    "available": true,  "error": null },
      { "name": "mistral", "available": false, "error": null }
    ]
  }
}
```

Utile pour afficher un indicateur « LLM en ligne » dans l'interface.

---

## 13. CORS

Le serveur autorise par défaut :
- `http://localhost:5173` (Vite)
- `http://localhost:5174`
- `https://stock-s.fr`

Si ton frontend tourne sur un autre port, ajoute-le dans
[`stocks_api/src/bin/server.rs`](../../stocks_api/src/bin/server.rs) (lignes
CORS). Sinon le navigateur bloque silencieusement (erreur console :
`blocked by CORS policy`).

---

## 14. Skeleton de composant React

Squelette minimaliste pour avoir quelque chose qui marche :

```tsx
// ChatWidget.tsx
import { useState, useRef, useEffect } from 'react';

const API = 'http://localhost:8090';

export function ChatWidget({ jwt }: { jwt: string }) {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [messages, setMessages] = useState<any[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [pendingAction, setPendingAction] = useState<any>(null);
  const [suggestions, setSuggestions] = useState<any[]>([]);
  const bottomRef = useRef<HTMLDivElement>(null);

  const headers = {
    'Authorization': `Bearer ${jwt}`,
    'Content-Type': 'application/json'
  };

  // Créer la session au montage
  useEffect(() => {
    fetch(`${API}/chat/sessions`, {
      method: 'POST', headers,
      body: JSON.stringify({ provider: 'auto' })
    })
      .then(r => r.json())
      .then(r => setSessionId(r.data.session_id));

    // Afficher le briefing proactif
    fetch(`${API}/chat/briefing`, { headers })
      .then(r => r.json())
      .then(r => {
        if (r.data?.summary) {
          setMessages([{ role: 'assistant', content: r.data.summary }]);
          if (r.data.suggested_questions) {
            setSuggestions(r.data.suggested_questions.map(
              (q: string) => ({ text: q, tool: null, args: null })
            ));
          }
        }
      });
  }, []);

  // Auto-scroll
  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  async function sendMessage(content: string) {
    if (!sessionId || !content.trim()) return;
    setLoading(true);
    setSuggestions([]);
    setMessages(m => [...m, { role: 'user', content }]);
    setInput('');

    const res = await fetch(`${API}/chat/sessions/${sessionId}/messages`, {
      method: 'POST', headers,
      body: JSON.stringify({ content })
    });
    const json = await res.json();
    const data = json.data;

    setMessages(m => [...m, {
      role: 'assistant',
      content: data.assistant_message.content,
      intent: data.intent,
      provider: data.provider_used,
      numbers_verified: data.numbers_verified,
      citations: data.citations,
    }]);

    setPendingAction(data.pending_action || null);
    setSuggestions(data.suggestions || []);
    setLoading(false);
  }

  async function confirmAction(decision: 'confirm' | 'cancel') {
    if (!pendingAction || !sessionId) return;
    const res = await fetch(`${API}/chat/sessions/${sessionId}/confirm-action`, {
      method: 'POST', headers,
      body: JSON.stringify({ action_id: pendingAction.action_id, decision })
    });
    const json = await res.json();
    setMessages(m => [...m, {
      role: 'system',
      content: decision === 'confirm' ? '✅ Action exécutée.' : '❌ Action annulée.'
    }]);
    setPendingAction(null);
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      {/* Messages */}
      <div style={{ flex: 1, overflow: 'auto', padding: 16 }}>
        {messages.map((m, i) => (
          <div key={i} style={{ marginBottom: 12, textAlign: m.role === 'user' ? 'right' : 'left' }}>
            <div style={{
              display: 'inline-block', maxWidth: '80%', padding: '8px 12px',
              background: m.role === 'user' ? '#0070f3' : '#f0f0f0',
              color: m.role === 'user' ? '#fff' : '#000',
              borderRadius: 12
            }}>
              {/* Rendu markdown simple — utilise react-markdown en prod */}
              <pre style={{ whiteSpace: 'pre-wrap', fontFamily: 'inherit', margin: 0 }}>
                {m.content}
              </pre>
              {m.numbers_verified === false && (
                <div style={{ color: 'orange', fontSize: 11, marginTop: 4 }}>
                  ⚠️ Chiffres non vérifiés
                </div>
              )}
              {m.citations?.length > 0 && (
                <div style={{ fontSize: 11, marginTop: 4, color: '#666' }}>
                  📎 {m.citations.map((c: any) => c.source_path).join(', ')}
                </div>
              )}
            </div>
            {m.provider && (
              <div style={{ fontSize: 10, color: '#aaa', marginTop: 2 }}>
                {m.provider === 'deterministic' ? '⚡' : '🤖'} {m.provider}
              </div>
            )}
          </div>
        ))}

        {/* Spinner */}
        {loading && (
          <div style={{ textAlign: 'left', color: '#aaa', fontSize: 14 }}>
            L'assistant réfléchit…
          </div>
        )}

        {/* Boutons de confirmation */}
        {pendingAction && (
          <div style={{ display: 'flex', gap: 8, marginTop: 8 }}>
            <button onClick={() => confirmAction('confirm')}
              style={{ padding: '8px 16px', background: '#22c55e', color: '#fff', border: 'none', borderRadius: 8, cursor: 'pointer' }}>
              ✅ Confirmer
            </button>
            <button onClick={() => confirmAction('cancel')}
              style={{ padding: '8px 16px', background: '#ef4444', color: '#fff', border: 'none', borderRadius: 8, cursor: 'pointer' }}>
              ❌ Annuler
            </button>
          </div>
        )}

        {/* Suggestions */}
        {suggestions.length > 0 && !loading && (
          <div style={{ marginTop: 12 }}>
            <div style={{ fontSize: 12, color: '#888', marginBottom: 6 }}>💡 Questions suivantes :</div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
              {suggestions.map((s: any, i: number) => (
                <button key={i}
                  onClick={() => sendMessage(s.text)}
                  style={{
                    padding: '6px 12px', fontSize: 13,
                    background: '#f5f5f5', border: '1px solid #ddd',
                    borderRadius: 20, cursor: 'pointer'
                  }}>
                  {s.text}
                </button>
              ))}
            </div>
          </div>
        )}

        <div ref={bottomRef} />
      </div>

      {/* Saisie */}
      <div style={{ display: 'flex', gap: 8, padding: 16, borderTop: '1px solid #eee' }}>
        <input
          value={input}
          onChange={e => setInput(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && !loading && sendMessage(input)}
          placeholder="Posez votre question…"
          disabled={loading}
          style={{ flex: 1, padding: '8px 12px', borderRadius: 8, border: '1px solid #ddd' }}
        />
        <button
          onClick={() => sendMessage(input)}
          disabled={loading || !input.trim()}
          style={{ padding: '8px 16px', background: '#0070f3', color: '#fff', border: 'none', borderRadius: 8, cursor: 'pointer' }}>
          Envoyer
        </button>
      </div>
    </div>
  );
}
```

Ce composant couvre :
- Briefing proactif à l'ouverture
- Envoi de messages
- Suggestions cliquables
- Confirmation d'actions d'écriture
- Indicateur de chargement
- Badge provider + vérification des nombres
- Citations RAG

---

## 15. Checklist de complétion

```
Infrastructure
  [x] Créer une session à l'ouverture
  [x] Appeler le briefing et l'afficher
  [x] Envoyer des messages et afficher les réponses

Messagerie
  [x] Historique scrollable
  [x] Auto-scroll vers le bas
  [x] Indicateur de chargement (spinner ou SSE labels)
  [x] Champ désactivé pendant l'envoi (anti-spam)
  [x] Envoi sur Entrée

Actions d'écriture
  [x] Détecter pending_action non nul
  [x] Afficher boutons Confirmer / Annuler
  [x] Désactiver après premier clic

Suggestions
  [x] Afficher les boutons de suivi après chaque réponse
  [x] Les envoyer comme message au clic

Gestion des erreurs
  [x] 401 → redirection login
  [x] 502 → toast + bouton Réessayer
  [x] numbers_verified=false → avertissement discret

Enrichissements
  [ ] Afficher le provider (badge)
  [ ] Icône selon intent (data/doc/action/chitchat)
  [ ] Citations sous les réponses doc
  [ ] Sélecteur de provider (dropdown)
  [ ] Bouton Supprimer la conversation
  [ ] Bouton Exporter en markdown
  [ ] Indicateur état des LLM (GET /chat/provider-health)
```

---

## 16. Sécurité

- **Ne jamais stocker le JWT en localStorage.** Préfère `sessionStorage` ou
  `httpOnly cookie`.
- **Sanitize le contenu du bot** avant rendu HTML. Le LLM peut retourner du
  markdown avec du HTML. Utilise `DOMPurify` ou un renderer markdown
  sécurisé (`react-markdown` avec `rehype-sanitize`).
- **Debounce le bouton Envoyer** — désactive-le pendant qu'une requête est
  en vol. Sinon l'utilisateur peut soumettre 10 messages en cliquant 10 fois.
