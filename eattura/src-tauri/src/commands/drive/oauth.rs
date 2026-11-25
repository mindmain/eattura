use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Url};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::Duration;
use tauri_plugin_opener::OpenerExt;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    time::timeout,
};

const SCOPE: &str = "https://www.googleapis.com/auth/drive.file";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    scope: Option<String>,
}

pub(super) async fn authorize(
    app: &tauri::AppHandle,
    http: &Client,
    client_id: &str,
    client_secret: &SecretString,
) -> Result<(SecretString, SecretString), String> {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(|_| "Impossibile aprire il callback locale di Google.")?;
    let address = listener.local_addr().map_err(|error| error.to_string())?;
    let redirect = format!("http://{address}/oauth/drive");
    let state = uuid::Uuid::new_v4().to_string();
    let verifier = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut url = Url::parse("https://accounts.google.com/o/oauth2/v2/auth")
        .map_err(|error| error.to_string())?;
    url.query_pairs_mut().extend_pairs([
        ("client_id", client_id),
        ("redirect_uri", &redirect),
        ("scope", SCOPE),
        ("state", &state),
        ("response_type", "code"),
        ("access_type", "offline"),
        ("prompt", "consent"),
        ("code_challenge", &challenge),
        ("code_challenge_method", "S256"),
    ]);
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|_| "Impossibile aprire il browser di sistema.")?;
    let code = timeout(Duration::from_secs(180), callback(listener, &redirect, &state))
        .await
        .map_err(|_| "Tempo scaduto per il collegamento Google Drive.")??;
    let tokens = token_request(
        http,
        &[
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("client_secret", client_secret.expose_secret()),
            ("code", code.expose_secret()),
            ("redirect_uri", &redirect),
            ("code_verifier", &verifier),
        ],
    )
    .await?;
    if !tokens
        .scope
        .as_deref()
        .is_some_and(|scopes| scopes.split_whitespace().any(|scope| scope == SCOPE))
    {
        return Err("Google non ha concesso il permesso Drive richiesto.".into());
    }
    let refresh = tokens
        .refresh_token
        .filter(|token| !token.is_empty())
        .ok_or("Google non ha restituito un token per il backup automatico.")?;
    Ok((tokens.access_token.into(), refresh.into()))
}

pub(super) async fn refresh(
    http: &Client,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<(SecretString, Option<SecretString>), String> {
    let tokens = token_request(
        http,
        &[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", refresh_token),
        ],
    )
    .await?;
    Ok((
        tokens.access_token.into(),
        tokens.refresh_token.map(Into::into),
    ))
}

async fn token_request(http: &Client, fields: &[(&str, &str)]) -> Result<TokenResponse, String> {
    http.post(TOKEN_URL)
        .form(fields)
        .send()
        .await
        .map_err(|_| "Impossibile contattare Google per l’autenticazione Drive.")?
        .error_for_status()
        .map_err(|_| "Google ha rifiutato le credenziali Drive. Ricollega l’account.")?
        .json()
        .await
        .map_err(|_| "Risposta OAuth di Google non valida.".into())
}

async fn callback(
    listener: TcpListener,
    redirect: &str,
    state: &str,
) -> Result<SecretString, String> {
    loop {
        let (mut stream, _) = listener.accept().await.map_err(|error| error.to_string())?;
        let reader = BufReader::new(&mut stream);
        let mut request = Vec::new();
        let read = timeout(Duration::from_secs(3), async {
            use tokio::io::AsyncReadExt;
            reader.take(8192).read_until(b'\n', &mut request).await
        })
        .await;
        if !matches!(read, Ok(Ok(_))) {
            continue;
        }
        let Some(result) = callback_request(&request, redirect, state) else {
            let _ = stream
                .write_all(
                    b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
                )
                .await;
            continue;
        };
        let message = "Collegamento a Google Drive completato. Puoi tornare su Eattura.";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{message}",
            message.len()
        );
        let _ = stream.write_all(response.as_bytes()).await;
        return result;
    }
}

fn callback_request(request: &[u8], redirect: &str, state: &str) -> Option<Result<SecretString, String>> {
    let mut parts = std::str::from_utf8(request).ok()?.split_ascii_whitespace();
    if parts.next()? != "GET" {
        return None;
    }
    let query = parts.next()?.strip_prefix("/oauth/drive?")?;
    let url = Url::parse(&format!("{redirect}?{query}")).ok()?;
    let fields: std::collections::HashMap<_, _> = url.query_pairs().collect();
    if fields.get("state")?.as_ref() != state {
        return None;
    }
    if fields.contains_key("error") {
        return Some(Err("Collegamento Google Drive annullato o negato.".into()));
    }
    let code = fields.get("code").filter(|code| !code.is_empty());
    Some(match code {
        Some(code) => Ok(code.to_string().into()),
        None => Err("Codice di autorizzazione Google Drive mancante.".into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_requires_matching_state_and_authorization_code() {
        let redirect = "http://127.0.0.1:1234/oauth/drive";
        let request = b"GET /oauth/drive?state=expected&code=auth_code_123 HTTP/1.1\r\n";
        assert!(callback_request(request, redirect, "different").is_none());
        let code = callback_request(request, redirect, "expected")
            .unwrap()
            .unwrap();
        assert_eq!(code.expose_secret(), "auth_code_123");

        let cancelled = b"GET /oauth/drive?state=expected&error=access_denied HTTP/1.1\r\n";
        assert!(
            callback_request(cancelled, redirect, "expected")
                .unwrap()
                .is_err()
        );
    }
}
