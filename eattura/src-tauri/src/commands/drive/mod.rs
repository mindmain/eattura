//! Import invoice files from a user-selected Google Drive folder.
mod oauth;

use std::{collections::BTreeMap, time::Duration};

use reqwest::{Client, RequestBuilder, Url, redirect::Policy};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::{commands::invoices, state::TauriState, storage::FileStore};

const KEYRING_SERVICE: &str = "eattura-google-drive";
const DRIVE_API: &str = "https://www.googleapis.com/drive/v3/";
const FOLDER_TYPE: &str = "application/vnd.google-apps.folder";
const MAX_FILE_BYTES: usize = 32 * 1024 * 1024;
const MAX_PAGES: usize = 1000;
static DRIVE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Deserialize)]
struct CredentialFile {
    installed: DesktopClient,
}

#[derive(Deserialize)]
struct DesktopClient {
    client_id: String,
    client_secret: String,
}

#[derive(Serialize, Deserialize)]
struct Connection {
    client_id: String,
    client_secret: String,
    refresh_token: String,
    folder_id: String,
    folder_name: String,
    account_id: String,
}

impl Drop for Connection {
    fn drop(&mut self) {
        // Move the stored secrets into zeroizing containers before dropping them.
        let _secret = SecretString::from(std::mem::take(&mut self.client_secret));
        let _token = SecretString::from(std::mem::take(&mut self.refresh_token));
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveStatus {
    folder_name: Option<String>,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    imported: usize,
    unchanged: usize,
    ignored: usize,
    failed: usize,
    errors: Vec<String>,
}

#[derive(Default, Serialize, Deserialize)]
struct SyncCache {
    folder_id: String,
    versions: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Folder {
    name: String,
    mime_type: String,
    drive_id: Option<String>,
    #[serde(default)]
    trashed: bool,
}

#[derive(Deserialize)]
struct About {
    user: DriveUser,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveUser {
    permission_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page {
    files: Vec<DriveFile>,
    next_page_token: Option<String>,
    #[serde(default)]
    incomplete_search: bool,
}

#[derive(Deserialize)]
struct DriveFile {
    id: String,
    name: String,
    version: String,
}

pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 256
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn keyring() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, "desktop")
        .map_err(|_| "Portachiavi di sistema non disponibile.".into())
}

fn load_connection() -> Result<Option<Connection>, String> {
    match keyring()?.get_password() {
        Ok(value) => {
            let value: SecretString = value.into();
            serde_json::from_str(value.expose_secret())
                .map(Some)
                .map_err(|_| "Collegamento Drive non leggibile nel portachiavi.".into())
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("Impossibile leggere il collegamento Drive dal portachiavi.".into()),
    }
}

fn save_connection(connection: &Connection) -> Result<(), String> {
    let value: SecretString = serde_json::to_string(connection)
        .map_err(|error| error.to_string())?
        .into();
    keyring()?
        .set_password(value.expose_secret())
        .map_err(|_| "Impossibile salvare il collegamento Drive nel portachiavi.".into())
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(Policy::none())
        .build()
        .map_err(|_| "Impossibile inizializzare il client Drive.".into())
}

#[tauri::command]
pub fn get_drive_status() -> Result<DriveStatus, String> {
    Ok(DriveStatus {
        folder_name: load_connection()?
            .as_ref()
            .map(|connection| connection.folder_name.clone()),
    })
}

#[tauri::command]
pub async fn connect_drive(
    app: tauri::AppHandle,
    credentials_json: String,
) -> Result<DriveStatus, String> {
    let _guard = DRIVE_LOCK
        .try_lock()
        .map_err(|_| "È già in corso un’operazione Drive.")?;
    let credentials_json: SecretString = credentials_json.into();
    if credentials_json.expose_secret().len() > 16 * 1024 {
        return Err("Il file delle credenziali Google è troppo grande.".into());
    }
    let credentials: CredentialFile = serde_json::from_str(credentials_json.expose_secret())
        .map_err(|_| "Seleziona il JSON OAuth di un client Google di tipo App desktop.")?;
    let DesktopClient {
        client_id,
        client_secret,
    } = credentials.installed;
    if !client_id.ends_with(".apps.googleusercontent.com") || client_secret.trim().is_empty() {
        return Err("Le credenziali OAuth desktop di Google non sono valide.".into());
    }
    let client_secret: SecretString = client_secret.into();
    let http = http_client()?;
    let (mut ids, access_token, refresh_token) =
        oauth::authorize(&app, &http, &client_id, &client_secret, oauth::Pick::Folder).await?;
    let folder_id = ids.pop().ok_or("Seleziona una cartella Google Drive.")?;
    let drive = Drive {
        http,
        token: access_token,
        base: DRIVE_API,
    };
    let folder = drive.folder(&folder_id).await?;
    let account_id = drive.account_id().await?;
    save_connection(&Connection {
        client_id,
        client_secret: client_secret.expose_secret().to_owned(),
        refresh_token: refresh_token.expose_secret().to_owned(),
        folder_id,
        folder_name: folder.name.clone(),
        account_id,
    })?;
    Ok(DriveStatus {
        folder_name: Some(folder.name),
    })
}

#[tauri::command]
pub async fn authorize_drive_files(
    app: tauri::AppHandle,
    state: tauri::State<'_, TauriState>,
) -> Result<SyncReport, String> {
    let _guard = DRIVE_LOCK
        .try_lock()
        .map_err(|_| "È già in corso un’operazione Drive.")?;
    let mut connection = load_connection()?.ok_or("Collega prima una cartella Drive.")?;
    let http = http_client()?;
    let secret = SecretString::from(std::mem::take(&mut connection.client_secret));
    let (ids, token, refresh) = oauth::authorize(
        &app,
        &http,
        &connection.client_id,
        &secret,
        oauth::Pick::Files,
    )
    .await?;
    let drive = Drive {
        http,
        token,
        base: DRIVE_API,
    };
    if drive.account_id().await? != connection.account_id {
        return Err("Seleziona lo stesso account Google usato per collegare la cartella.".into());
    }
    let folder = drive.folder(&connection.folder_id).await?;
    let files = drive
        .list(&connection.folder_id, folder.drive_id.as_deref())
        .await?;
    if !files.iter().any(|file| ids.contains(&file.id)) {
        return Err("Seleziona almeno una fattura XML o P7M nella cartella collegata.".into());
    }
    connection.client_secret = secret.expose_secret().to_owned();
    let _old: SecretString = std::mem::replace(
        &mut connection.refresh_token,
        refresh.expose_secret().to_owned(),
    )
    .into();
    connection.folder_name = folder.name;
    save_connection(&connection)?;
    sync_files(&state.store, &drive, &connection.folder_id, files).await
}

#[tauri::command]
pub fn disconnect_drive() -> Result<DriveStatus, String> {
    let _guard = DRIVE_LOCK
        .try_lock()
        .map_err(|_| "È già in corso un’operazione Drive.")?;
    match keyring()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(DriveStatus { folder_name: None }),
        Err(_) => Err("Impossibile rimuovere il collegamento Drive dal portachiavi.".into()),
    }
}

#[tauri::command]
pub async fn sync_drive(state: tauri::State<'_, TauriState>) -> Result<Option<SyncReport>, String> {
    let _guard = DRIVE_LOCK
        .try_lock()
        .map_err(|_| "È già in corso un’operazione Drive.")?;
    let Some(mut connection) = load_connection()? else {
        return Ok(None);
    };
    let http = http_client()?;
    let (token, refresh) = oauth::refresh(
        &http,
        &connection.client_id,
        &connection.client_secret,
        &connection.refresh_token,
    )
    .await?;
    if let Some(refresh) = refresh {
        let _old: SecretString = std::mem::replace(
            &mut connection.refresh_token,
            refresh.expose_secret().to_owned(),
        )
        .into();
        save_connection(&connection)?;
    }
    let drive = Drive {
        http,
        token,
        base: DRIVE_API,
    };
    let folder = drive.folder(&connection.folder_id).await?;
    let files = drive
        .list(&connection.folder_id, folder.drive_id.as_deref())
        .await?;
    sync_files(&state.store, &drive, &connection.folder_id, files)
        .await
        .map(Some)
}

struct Drive<'a> {
    http: Client,
    token: SecretString,
    base: &'a str,
}

impl Drive<'_> {
    fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<RequestBuilder, String> {
        let mut url =
            Url::parse(&format!("{}{path}", self.base)).map_err(|_| "URL Drive non valido.")?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        Ok(self.http.get(url).bearer_auth(self.token.expose_secret()))
    }

    async fn account_id(&self) -> Result<String, String> {
        let about: About =
            response_json(self.get("about", &[("fields", "user(permissionId)")])?).await?;
        Ok(about.user.permission_id)
    }

    async fn folder(&self, id: &str) -> Result<Folder, String> {
        if !valid_id(id) {
            return Err("Identificativo della cartella Drive non valido.".into());
        }
        let response = self.get(
            &format!("files/{id}"),
            &[
                ("supportsAllDrives", "true"),
                ("fields", "name,mimeType,driveId,trashed"),
            ],
        )?;
        let folder: Folder = response_json(response).await?;
        if folder.mime_type != FOLDER_TYPE || folder.trashed {
            return Err("Seleziona una cartella Drive accessibile e fuori dal cestino.".into());
        }
        Ok(folder)
    }

    async fn list(
        &self,
        folder_id: &str,
        drive_id: Option<&str>,
    ) -> Result<Vec<DriveFile>, String> {
        let query = format!("'{folder_id}' in parents and trashed = false");
        let mut parameters = vec![
            ("q", query.as_str()),
            ("supportsAllDrives", "true"),
            ("includeItemsFromAllDrives", "true"),
            ("pageSize", "100"),
            (
                "fields",
                "nextPageToken,incompleteSearch,files(id,name,version)",
            ),
        ];
        if let Some(id) = drive_id {
            parameters.extend([("corpora", "drive"), ("driveId", id)]);
        }
        let mut files = Vec::new();
        let mut next: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let mut request = parameters.clone();
            if let Some(token) = &next {
                request.push(("pageToken", token));
            }
            let page: Page = response_json(self.get("files", &request)?).await?;
            if page.incomplete_search {
                return Err("Google Drive ha restituito un elenco incompleto.".into());
            }
            files.extend(page.files.into_iter().filter(|file| supported(&file.name)));
            next = page.next_page_token;
            if next.is_none() {
                return Ok(files);
            }
        }
        Err("La cartella Drive contiene troppi elementi per la sincronizzazione.".into())
    }

    async fn download(&self, id: &str) -> Result<Vec<u8>, String> {
        if !valid_id(id) {
            return Err("Identificativo del file Drive non valido.".into());
        }
        let mut response = response(self.get(
            &format!("files/{id}"),
            &[("alt", "media"), ("supportsAllDrives", "true")],
        )?)
        .await?;
        if response
            .content_length()
            .is_some_and(|size| size > MAX_FILE_BYTES as u64)
        {
            return Err("Documento oltre il limite di 32 MiB.".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Scaricamento Drive interrotto.")?
        {
            if bytes.len() + chunk.len() > MAX_FILE_BYTES {
                return Err("Documento oltre il limite di 32 MiB.".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}

async fn response(request: RequestBuilder) -> Result<reqwest::Response, String> {
    let response = request
        .send()
        .await
        .map_err(|_| "Google Drive non raggiungibile.")?;
    if !response.status().is_success() {
        return Err(format!(
            "Google Drive ha risposto HTTP {}.",
            response.status().as_u16()
        ));
    }
    Ok(response)
}

async fn response_json<T: serde::de::DeserializeOwned>(
    request: RequestBuilder,
) -> Result<T, String> {
    response(request)
        .await?
        .json()
        .await
        .map_err(|_| "Risposta Drive non valida.".into())
}

fn supported(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, extension)| {
        extension.eq_ignore_ascii_case("xml") || extension.eq_ignore_ascii_case("p7m")
    })
}

async fn sync_files(
    store: &FileStore,
    drive: &Drive<'_>,
    folder_id: &str,
    files: Vec<DriveFile>,
) -> Result<SyncReport, String> {
    let cache_path = store.data_dir().join("drive-import.json");
    let mut cache: SyncCache = match std::fs::read(&cache_path) {
        Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or_default(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => SyncCache::default(),
        Err(error) => return Err(format!("Impossibile leggere lo stato Drive: {error}")),
    };
    if cache.folder_id != folder_id {
        cache = SyncCache {
            folder_id: folder_id.to_owned(),
            versions: BTreeMap::new(),
        };
    }
    let mut report = SyncReport::default();
    let mut versions = BTreeMap::new();
    for file in files {
        if cache.versions.get(&file.id) == Some(&file.version) {
            report.unchanged += 1;
            versions.insert(file.id, file.version);
            continue;
        }
        let result = match drive.download(&file.id).await {
            Ok(bytes) => import_file(store, &file.name, &bytes).await,
            Err(error) => Err(error),
        };
        match result {
            Ok(imported) => {
                if imported {
                    report.imported += 1;
                } else {
                    report.ignored += 1;
                }
                versions.insert(file.id, file.version);
            }
            Err(error) => {
                report.failed += 1;
                if report.errors.len() < 10 {
                    report.errors.push(format!("{}: {error}", file.name));
                }
            }
        }
    }
    cache.versions = versions;
    crate::storage::atomic_json(&cache_path, &cache)?;
    Ok(report)
}

async fn import_file(store: &FileStore, name: &str, bytes: &[u8]) -> Result<bool, String> {
    let (payload, signer) = if name.to_ascii_lowercase().ends_with(".p7m") {
        let (xml, signer) = common::p7m::inspect_p7m(bytes).map_err(|error| error.to_string())?;
        (
            std::borrow::Cow::Owned(xml),
            signer.is_signed.then_some(signer),
        )
    } else {
        (std::borrow::Cow::Borrowed(bytes), None)
    };
    let xml = std::str::from_utf8(&payload).map_err(|_| "XML con codifica non valida.")?;
    if let Ok(receipt) = common::pec::parse_notification(xml)
        && !matches!(
            receipt.message_type,
            common::pec::SdiMessageType::Unknown(_)
        )
    {
        return Ok(false);
    }
    invoices::import_xml_content_with_signer(store, xml, signer).await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    const INVOICE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<FatturaElettronica versione="FPR12">
<FatturaElettronicaHeader>
<DatiTrasmissione><IdTrasmittente><IdPaese>IT</IdPaese><IdCodice>01234567890</IdCodice></IdTrasmittente><ProgressivoInvio>00001</ProgressivoInvio><FormatoTrasmissione>FPR12</FormatoTrasmissione><CodiceDestinatario>0000000</CodiceDestinatario></DatiTrasmissione>
<CedentePrestatore><DatiAnagrafici><IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>01234567890</IdCodice></IdFiscaleIVA><Anagrafica><Denominazione>Example Issuer</Denominazione></Anagrafica><RegimeFiscale>RF01</RegimeFiscale></DatiAnagrafici><Sede><Indirizzo>Via Roma 1</Indirizzo><CAP>00100</CAP><Comune>Roma</Comune><Nazione>IT</Nazione></Sede></CedentePrestatore>
<CessionarioCommittente><DatiAnagrafici><IdFiscaleIVA><IdPaese>IT</IdPaese><IdCodice>09876543210</IdCodice></IdFiscaleIVA><Anagrafica><Denominazione>Example Customer</Denominazione></Anagrafica></DatiAnagrafici><Sede><Indirizzo>Via Roma 2</Indirizzo><CAP>00100</CAP><Comune>Roma</Comune><Nazione>IT</Nazione></Sede></CessionarioCommittente>
</FatturaElettronicaHeader>
<FatturaElettronicaBody><DatiGenerali><DatiGeneraliDocumento><TipoDocumento>TD01</TipoDocumento><Divisa>EUR</Divisa><Data>2026-01-01</Data><Numero>DRIVE-1</Numero></DatiGeneraliDocumento></DatiGenerali><DatiBeniServizi><DettaglioLinee><NumeroLinea>1</NumeroLinea><Descrizione>Services</Descrizione><PrezzoUnitario>100</PrezzoUnitario><PrezzoTotale>100</PrezzoTotale><AliquotaIVA>22</AliquotaIVA></DettaglioLinee><DatiRiepilogo><AliquotaIVA>22</AliquotaIVA><ImponibileImporto>100</ImponibileImporto><Imposta>22</Imposta></DatiRiepilogo></DatiBeniServizi></FatturaElettronicaBody>
</FatturaElettronica>"#;

    #[tokio::test]
    async fn sync_paginates_a_shared_folder_and_skips_cached_versions() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/drive/v3/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            for (path, expected_query, body) in [
                (
                    "/drive/v3/files/folder",
                    "supportsAllDrives=true",
                    r#"{"name":"Invoices","mimeType":"application/vnd.google-apps.folder","driveId":"team"}"#,
                ),
                (
                    "/drive/v3/files?",
                    "corpora=drive",
                    r#"{"nextPageToken":"second","files":[{"id":"invoice","name":"invoice.xml","version":"1"}]}"#,
                ),
                (
                    "/drive/v3/files?",
                    "pageToken=second",
                    r#"{"files":[{"id":"text","name":"notes.txt","version":"1"}]}"#,
                ),
                ("/drive/v3/files/invoice?", "alt=media", INVOICE),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    request.push(socket.read_u8().await.unwrap());
                    assert!(request.len() < 8192);
                }
                let request = String::from_utf8(request).unwrap();
                assert!(request.starts_with(&format!("GET {path}")));
                assert!(request.lines().next().unwrap().contains(expected_query));
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains("authorization: bearer fixture")
                );
                socket.write_all(format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()
                ).as_bytes()).await.unwrap();
            }
        });

        let archive = tempfile::tempdir().unwrap();
        let store_path = archive.path().join("eattura.json");
        let store = FileStore::open(store_path.clone()).unwrap();
        let drive = Drive {
            http: http_client().unwrap(),
            token: "fixture".into(),
            base: &base,
        };
        let folder = drive.folder("folder").await.unwrap();
        let files = drive
            .list("folder", folder.drive_id.as_deref())
            .await
            .unwrap();
        assert_eq!(files.len(), 1);
        let report = sync_files(&store, &drive, "folder", files).await.unwrap();
        assert_eq!(report.imported, 1, "{:?}", report.errors);
        server.await.unwrap();

        let reopened = FileStore::open(store_path).unwrap();
        let cached = DriveFile {
            id: "invoice".into(),
            name: "invoice.xml".into(),
            version: "1".into(),
        };
        let report = sync_files(&reopened, &drive, "folder", vec![cached])
            .await
            .unwrap();
        assert_eq!(report.unchanged, 1);
        let invoices = invoices::list_invoices_store(&reopened).await.unwrap();
        assert_eq!(invoices.len(), 1);
        assert_eq!(invoices[0].numero, "DRIVE-1");
        assert_eq!(invoices[0].stato, "imported");
        assert!(!import_file(&reopened, "receipt.xml", b"<RicevutaConsegna><NomeFile>IT01234567890_00001.xml</NomeFile></RicevutaConsegna>").await.unwrap());
    }
}
