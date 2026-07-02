use std::env;

use serde_json::json;

/// Envoie un email HTML via l'API HTTP SendGrid (même fournisseur que les rapports
/// de l'équipe). Utilise `reqwest` (déjà dépendance) — aucun SMTP, aucun bump de toolchain.
///
/// Variables d'environnement :
///   SENDGRID_API_KEY   — clé API SendGrid (si absente : envoi désactivé, renvoie une erreur claire)
///   TICKET_FROM_EMAIL  — adresse expéditeur (défaut : caisse@stock-s.fr)
///   TICKET_FROM_NAME   — nom expéditeur (défaut : Caisse StockS)
pub async fn send_email(to: &str, subject: &str, html_body: String) -> Result<(), String> {
    let api_key = env::var("SENDGRID_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| "Email non configuré (SENDGRID_API_KEY absente)".to_string())?;

    let from_email = env::var("TICKET_FROM_EMAIL").unwrap_or_else(|_| "caisse@stock-s.fr".to_string());
    let from_name = env::var("TICKET_FROM_NAME").unwrap_or_else(|_| "Caisse StockS".to_string());

    let payload = json!({
        "personalizations": [{ "to": [{ "email": to }] }],
        "from": { "email": from_email, "name": from_name },
        "subject": subject,
        "content": [{ "type": "text/html", "value": html_body }],
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.sendgrid.com/v3/mail/send")
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Requête SendGrid échouée : {e}"))?;

    let status = resp.status();
    if status.is_success() {
        Ok(())
    } else {
        let body = resp.text().await.unwrap_or_default();
        Err(format!("SendGrid a répondu {status} : {body}"))
    }
}
