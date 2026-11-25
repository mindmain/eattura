//! Automatic single-ZIP archive backup and synchronization over Google Drive.
pub(crate) mod backup;
mod oauth;

use std::{collections::BTreeMap, time::Duration};

use reqwest::{Client, RequestBuilder, Url, redirect::Policy};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::Emitter;

use crate::{
    app_log::{AppLog, LogLevel},
    state::TauriState,
    storage::FileStore,
};
use backup::{ArchiveSnapshot, merge_snapshots};

const KEYRING_SERVICE: &str = "eattura-google-drive";
const DRIVE_API: &str = "https://www.googleapis.com/drive/v3/";
const DRIVE_UPLOAD_API: &str = "https://www.googleapis.com/upload/drive/v3/";
const FOLDER_MIME: &str = "application/vnd.google-apps.folder";
const ZIP_MIME: &str = "application/zip";
const BACKUP_FOLDER_NAME: &str = "Eattura";
const BACKUP_FILE_NAME: &str = "eattura-backup.zip";
const STATE_FILE: &str = "drive-backup.json";
const MAX_BACKUP_BYTES: usize = 128 * 1024 * 1024;
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
    account_id: String,
    #[serde(default)]
    account_email: Option<String>,
    #[serde(default)]
    folder_id: Option<String>,
    #[serde(default)]
    backup_file_id: Option<String>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        let _secret = SecretString::from(std::mem::take(&mut self.client_secret));
        let _token = SecretString::from(std::mem::take(&mut self.refresh_token));
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalSyncState {
    account_id: String,
    backup_file_id: Option<String>,
    last_synced_sha256: Option<String>,
    last_remote_version: Option<String>,
    last_backup_at: Option<String>,
    last_checked_at: Option<i64>,
    last_action: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveStatus {
    pub connected: bool,
    pub account_email: Option<String>,
    pub backup_name: Option<String>,
    pub last_backup_at: Option<String>,
    pub last_checked_at: Option<i64>,
    pub last_action: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub action: String,
    pub modified_time: Option<String>,
    pub invoices: usize,
    pub clients: usize,
    pub notifications: usize,
}

#[derive(Deserialize)]
struct About {
    user: DriveUser,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveUser {
    permission_id: String,
    #[serde(default)]
    email_address: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
}

#[derive(Deserialize)]
struct FileList {
    files: Vec<DriveFile>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DriveFile {
    id: String,
    #[allow(dead_code)]
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    modified_time: Option<String>,
    #[serde(default)]
    trashed: bool,
    #[serde(default)]
    app_properties: BTreeMap<String, String>,
}

fn valid_id(id: &str) -> bool {
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

fn load_local_state(store: &FileStore, account_id: &str) -> LocalSyncState {
    let path = store.data_dir().join(STATE_FILE);
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice::<LocalSyncState>(&bytes)
            .ok()
            .filter(|state| state.account_id == account_id)
            .unwrap_or_else(|| LocalSyncState {
                account_id: account_id.to_owned(),
                ..LocalSyncState::default()
            }),
        Err(_) => LocalSyncState {
            account_id: account_id.to_owned(),
            ..LocalSyncState::default()
        },
    }
}

fn save_local_state(store: &FileStore, state: &LocalSyncState) -> Result<(), String> {
    crate::storage::atomic_json(&store.data_dir().join(STATE_FILE), state)
}

fn remove_local_state(store: &FileStore) {
    let _ = std::fs::remove_file(store.data_dir().join(STATE_FILE));
    let _ = std::fs::remove_file(store.data_dir().join("drive-import.json"));
}

fn build_status(store: &FileStore, connection: Option<&Connection>) -> DriveStatus {
    let Some(connection) = connection else {
        return DriveStatus {
            connected: false,
            account_email: None,
            backup_name: None,
            last_backup_at: None,
            last_checked_at: None,
            last_action: None,
        };
    };
    let state = load_local_state(store, &connection.account_id);
    DriveStatus {
        connected: true,
        account_email: connection.account_email.clone(),
        backup_name: Some(format!("{BACKUP_FOLDER_NAME} / {BACKUP_FILE_NAME}")),
        last_backup_at: state.last_backup_at,
        last_checked_at: state.last_checked_at,
        last_action: state.last_action,
    }
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .redirect(Policy::none())
        .build()
        .map_err(|_| "Impossibile inizializzare il client Google Drive.".into())
}

#[tauri::command]
pub fn get_drive_status(state: tauri::State<'_, TauriState>) -> Result<DriveStatus, String> {
    let connection = load_connection()?;
    Ok(build_status(&state.store, connection.as_ref()))
}

#[tauri::command]
pub async fn connect_drive(
    app: tauri::AppHandle,
    state: tauri::State<'_, TauriState>,
    log: tauri::State<'_, AppLog>,
    credentials_json: String,
) -> Result<DriveStatus, String> {
    let _guard = DRIVE_LOCK.lock().await;
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
    let (access_token, refresh_token) =
        oauth::authorize(&app, &http, &client_id, &client_secret).await?;
    let drive = Drive {
        http,
        token: access_token,
        api_base: DRIVE_API,
        upload_base: DRIVE_UPLOAD_API,
    };
    let user = drive.current_user().await?;
    let mut connection = Connection {
        client_id,
        client_secret: client_secret.expose_secret().to_owned(),
        refresh_token: refresh_token.expose_secret().to_owned(),
        account_id: user.permission_id,
        account_email: user.email_address.or(user.display_name),
        folder_id: None,
        backup_file_id: None,
    };
    save_connection(&connection)?;
    let report = sync_with_drive(&state.store, &drive, &mut connection).await?;
    save_connection(&connection)?;
    log.record(
        LogLevel::Info,
        "drive",
        &format!(
            "Google Drive connected ({}) — initial sync action: {}",
            connection.account_email.as_deref().unwrap_or("account"),
            report.action
        ),
    );
    let _ = app.emit("drive-backup-synced", &report);
    Ok(build_status(&state.store, Some(&connection)))
}

#[tauri::command]
pub async fn disconnect_drive(
    state: tauri::State<'_, TauriState>,
    log: tauri::State<'_, AppLog>,
) -> Result<DriveStatus, String> {
    let _guard = DRIVE_LOCK.lock().await;
    remove_local_state(&state.store);
    match keyring()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {
            log.record(LogLevel::Info, "drive", "Google Drive disconnected");
            Ok(build_status(&state.store, None))
        }
        Err(_) => Err("Impossibile rimuovere il collegamento Drive dal portachiavi.".into()),
    }
}

#[tauri::command]
pub async fn sync_drive(
    app: tauri::AppHandle,
    state: tauri::State<'_, TauriState>,
    log: tauri::State<'_, AppLog>,
) -> Result<Option<SyncReport>, String> {
    let report = run_sync_locked(&state.store, &log).await?;
    if let Some(ref payload) = report {
        let _ = app.emit("drive-backup-synced", payload);
    }
    Ok(report)
}

pub(crate) async fn run_background_sync(
    store: &FileStore,
    log: &AppLog,
    app: &tauri::AppHandle,
) {
    match run_sync_locked(store, log).await {
        Ok(Some(report)) => {
            let _ = app.emit("drive-backup-synced", &report);
        }
        Ok(None) => {}
        Err(error) => {
            log.record(
                LogLevel::Warn,
                "drive",
                &format!("Automatic Google Drive backup check failed: {error}"),
            );
        }
    }
}

async fn run_sync_locked(store: &FileStore, log: &AppLog) -> Result<Option<SyncReport>, String> {
    let _guard = DRIVE_LOCK.lock().await;
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
        api_base: DRIVE_API,
        upload_base: DRIVE_UPLOAD_API,
    };
    let prev_folder = connection.folder_id.clone();
    let prev_file = connection.backup_file_id.clone();
    let report = sync_with_drive(store, &drive, &mut connection).await?;
    if connection.folder_id != prev_folder || connection.backup_file_id != prev_file {
        save_connection(&connection)?;
    }
    if report.action != "up_to_date" {
        log.record(
            LogLevel::Info,
            "drive",
            &format!(
                "Google Drive backup {}: {} invoice(s), {} client(s)",
                report.action, report.invoices, report.clients
            ),
        );
    }
    Ok(Some(report))
}

async fn sync_with_drive(
    store: &FileStore,
    drive: &Drive<'_>,
    connection: &mut Connection,
) -> Result<SyncReport, String> {
    let mut local_state = load_local_state(store, &connection.account_id);
    let local_snapshot = ArchiveSnapshot::capture(store).await?;
    let remote_file = drive
        .find_backup_file(
            connection
                .backup_file_id
                .as_deref()
                .or(local_state.backup_file_id.as_deref()),
        )
        .await?;
    let now_ms = chrono::Utc::now().timestamp_millis();

    let (action, final_file, counts, synced_sha) = match remote_file {
        None => {
            // First backup on this Google Drive account: upload clean archive ZIP.
            let folder_id = drive
                .ensure_backup_folder(connection.folder_id.as_deref())
                .await?;
            connection.folder_id = Some(folder_id.clone());
            let now_rfc3339 = chrono::Utc::now().to_rfc3339();
            let zip_bytes = local_snapshot.encode_zip(&now_rfc3339)?;
            let created = drive
                .upload_backup(None, Some(&folder_id), &local_snapshot.sha256, &zip_bytes)
                .await?;
            (
                "uploaded".to_owned(),
                created,
                local_snapshot.counts(),
                local_snapshot.sha256,
            )
        }
        Some(remote) => {
            let remote_sha = remote.app_properties.get("snapshotSha256").cloned();
            let remote_unchanged = match (&remote_sha, &local_state.last_synced_sha256) {
                (Some(r_sha), Some(last_sha)) => r_sha == last_sha,
                (None, _) => {
                    !remote.version.is_empty()
                        && local_state.last_remote_version.as_deref() == Some(remote.version.as_str())
                }
                _ => false,
            };

            if remote_sha.as_deref() == Some(local_snapshot.sha256.as_str()) {
                (
                    "up_to_date".to_owned(),
                    remote,
                    local_snapshot.counts(),
                    local_snapshot.sha256,
                )
            } else if remote_unchanged {
                // Remote did not change since our last sync, and local has new changes -> update single ZIP in-place.
                let now_rfc3339 = chrono::Utc::now().to_rfc3339();
                let zip_bytes = local_snapshot.encode_zip(&now_rfc3339)?;
                let updated = drive
                    .upload_backup(Some(&remote.id), None, &local_snapshot.sha256, &zip_bytes)
                    .await?;
                (
                    "uploaded".to_owned(),
                    updated,
                    local_snapshot.counts(),
                    local_snapshot.sha256,
                )
            } else {
                // Remote has a newer backup -> download and inspect.
                let zip_bytes = drive.download(&remote.id).await?;
                let (_, remote_snapshot) = ArchiveSnapshot::decode_zip(&zip_bytes)?;

                if remote_snapshot.sha256 == local_snapshot.sha256 {
                    (
                        "up_to_date".to_owned(),
                        remote,
                        local_snapshot.counts(),
                        local_snapshot.sha256,
                    )
                } else if local_snapshot.is_empty()
                    || local_state.last_synced_sha256.as_deref()
                        == Some(local_snapshot.sha256.as_str())
                {
                    // Local had no unsynced edits (2nd device opening or 5-min poll) -> apply remote snapshot directly.
                    remote_snapshot.apply(store).await?;
                    (
                        "downloaded".to_owned(),
                        remote,
                        remote_snapshot.counts(),
                        remote_snapshot.sha256,
                    )
                } else {
                    // Both devices modified data concurrently -> merge without losing records and update Drive.
                    let merged = merge_snapshots(&local_snapshot, &remote_snapshot)?;
                    merged.apply(store).await?;
                    let updated = if merged.sha256 != remote_snapshot.sha256 {
                        let now_rfc3339 = chrono::Utc::now().to_rfc3339();
                        let merged_zip = merged.encode_zip(&now_rfc3339)?;
                        drive
                            .upload_backup(Some(&remote.id), None, &merged.sha256, &merged_zip)
                            .await?
                    } else {
                        remote
                    };
                    ("merged".to_owned(), updated, merged.counts(), merged.sha256)
                }
            }
        }
    };

    connection.backup_file_id = Some(final_file.id.clone());
    local_state.backup_file_id = Some(final_file.id);
    local_state.last_synced_sha256 = Some(synced_sha);
    local_state.last_remote_version = Some(final_file.version);
    if final_file.modified_time.is_some() {
        local_state.last_backup_at.clone_from(&final_file.modified_time);
    }
    local_state.last_checked_at = Some(now_ms);
    local_state.last_action = Some(action.clone());
    save_local_state(store, &local_state)?;

    Ok(SyncReport {
        action,
        modified_time: final_file.modified_time,
        invoices: counts.invoices,
        clients: counts.clients,
        notifications: counts.notifications,
    })
}

struct Drive<'a> {
    http: Client,
    token: SecretString,
    api_base: &'a str,
    upload_base: &'a str,
}

impl Drive<'_> {
    fn get(&self, path: &str, query: &[(&str, &str)]) -> Result<RequestBuilder, String> {
        let mut url = Url::parse(&format!("{}{path}", self.api_base))
            .map_err(|_| "URL Google Drive non valido.")?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        Ok(self.http.get(url).bearer_auth(self.token.expose_secret()))
    }

    async fn current_user(&self) -> Result<DriveUser, String> {
        let about: About = response_json(self.get(
            "about",
            &[("fields", "user(permissionId,emailAddress,displayName)")],
        )?)
        .await?;
        Ok(about.user)
    }

    async fn get_file_if_active(&self, id: &str) -> Result<Option<DriveFile>, String> {
        if !valid_id(id) {
            return Ok(None);
        }
        let resp = self
            .get(
                &format!("files/{id}"),
                &[(
                    "fields",
                    "id,name,version,modifiedTime,trashed,appProperties",
                )],
            )?
            .send()
            .await
            .map_err(|_| "Google Drive non raggiungibile.")?;
        if resp.status().as_u16() == 404 {
            return Ok(None);
        }
        if !resp.status().is_success() {
            return Err(format!(
                "Google Drive ha risposto HTTP {}.",
                resp.status().as_u16()
            ));
        }
        let file: DriveFile = resp
            .json()
            .await
            .map_err(|_| "Risposta Drive non valida.".to_string())?;
        Ok((!file.trashed).then_some(file))
    }

    async fn find_backup_file(&self, known_id: Option<&str>) -> Result<Option<DriveFile>, String> {
        if let Some(id) = known_id
            && let Some(file) = self.get_file_if_active(id).await?
        {
            return Ok(Some(file));
        }
        let query = format!(
            "(appProperties has {{ key='eatturaRole' and value='backup' }} or name = '{BACKUP_FILE_NAME}') and mimeType != '{FOLDER_MIME}' and trashed = false"
        );
        let list: FileList = response_json(self.get(
            "files",
            &[
                ("q", query.as_str()),
                ("orderBy", "modifiedTime desc"),
                ("pageSize", "5"),
                (
                    "fields",
                    "files(id,name,version,modifiedTime,trashed,appProperties)",
                ),
            ],
        )?)
        .await?;
        Ok(list.files.into_iter().find(|file| !file.trashed))
    }

    async fn ensure_backup_folder(&self, known_id: Option<&str>) -> Result<String, String> {
        if let Some(id) = known_id
            && let Some(folder) = self.get_file_if_active(id).await?
        {
            return Ok(folder.id);
        }
        let query = format!(
            "appProperties has {{ key='eatturaRole' and value='backup-folder' }} and mimeType = '{FOLDER_MIME}' and trashed = false"
        );
        let list: FileList = response_json(self.get(
            "files",
            &[
                ("q", query.as_str()),
                ("pageSize", "1"),
                ("fields", "files(id,name,trashed)"),
            ],
        )?)
        .await?;
        if let Some(existing) = list.files.into_iter().find(|f| !f.trashed) {
            return Ok(existing.id);
        }
        let mut url = Url::parse(&format!("{}files", self.api_base))
            .map_err(|_| "URL Google Drive non valido.")?;
        url.query_pairs_mut().append_pair("fields", "id,name");
        let created: DriveFile = response_json(
            self.http
                .post(url)
                .bearer_auth(self.token.expose_secret())
                .json(&json!({
                    "name": BACKUP_FOLDER_NAME,
                    "mimeType": FOLDER_MIME,
                    "appProperties": {
                        "eatturaRole": "backup-folder"
                    }
                })),
        )
        .await?;
        Ok(created.id)
    }

    async fn upload_backup(
        &self,
        existing_file_id: Option<&str>,
        parent_folder_id: Option<&str>,
        snapshot_sha256: &str,
        zip_bytes: &[u8],
    ) -> Result<DriveFile, String> {
        if zip_bytes.len() > MAX_BACKUP_BYTES {
            return Err("Il backup supera il limite massimo di 128 MiB.".into());
        }
        let mut metadata = json!({
            "name": BACKUP_FILE_NAME,
            "mimeType": ZIP_MIME,
            "appProperties": {
                "eatturaRole": "backup",
                "schemaVersion": "1",
                "snapshotSha256": snapshot_sha256,
            }
        });
        if existing_file_id.is_none()
            && let Some(folder_id) = parent_folder_id
        {
            metadata["parents"] = json!([folder_id]);
        }

        let boundary = format!("eattura-{}", uuid::Uuid::new_v4().simple());
        let meta_bytes = serde_json::to_vec(&metadata).map_err(|e| e.to_string())?;
        let mut body = Vec::with_capacity(meta_bytes.len() + zip_bytes.len() + 256);
        body.extend_from_slice(format!("--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n").as_bytes());
        body.extend_from_slice(&meta_bytes);
        body.extend_from_slice(format!("\r\n--{boundary}\r\nContent-Type: {ZIP_MIME}\r\n\r\n").as_bytes());
        body.extend_from_slice(zip_bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let path = match existing_file_id {
            Some(id) => {
                if !valid_id(id) {
                    return Err("Identificativo del backup Drive non valido.".into());
                }
                format!("{}files/{id}", self.upload_base)
            }
            None => format!("{}files", self.upload_base),
        };
        let mut url = Url::parse(&path).map_err(|_| "URL upload Google Drive non valido.")?;
        url.query_pairs_mut().extend_pairs([
            ("uploadType", "multipart"),
            ("fields", "id,name,version,modifiedTime,appProperties"),
        ]);

        let builder = if existing_file_id.is_some() {
            self.http.patch(url)
        } else {
            self.http.post(url)
        };
        response_json(
            builder
                .bearer_auth(self.token.expose_secret())
                .header(
                    reqwest::header::CONTENT_TYPE,
                    format!("multipart/related; boundary={boundary}"),
                )
                .body(body),
        )
        .await
    }

    async fn download(&self, id: &str) -> Result<Vec<u8>, String> {
        if !valid_id(id) {
            return Err("Identificativo del backup Drive non valido.".into());
        }
        let mut response = response(self.get(&format!("files/{id}"), &[("alt", "media")])?).await?;
        if response
            .content_length()
            .is_some_and(|size| size > MAX_BACKUP_BYTES as u64)
        {
            return Err("Il backup Drive supera il limite di 128 MiB.".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Scaricamento del backup Drive interrotto.")?
        {
            if bytes.len() + chunk.len() > MAX_BACKUP_BYTES {
                return Err("Il backup Drive supera il limite di 128 MiB.".into());
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
        .map_err(|_| "Risposta Google Drive non valida.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    #[derive(Default)]
    struct FakeDriveState {
        zip_bytes: Option<Vec<u8>>,
        sha256: Option<String>,
        version: u64,
        uploads: usize,
        patches: usize,
    }

    async fn spawn_fake_drive(state: Arc<Mutex<FakeDriveState>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            while let Ok((mut socket, _)) = listener.accept().await {
                let mut raw = Vec::new();
                let mut header_end = None;
                let mut buf = [0_u8; 4096];
                loop {
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    raw.extend_from_slice(&buf[..n]);
                    if header_end.is_none()
                        && let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n")
                    {
                        header_end = Some(pos + 4);
                    }
                    if let Some(end) = header_end {
                        let headers = String::from_utf8_lossy(&raw[..end]);
                        let content_len = headers
                            .lines()
                            .find_map(|line| {
                                let (k, v) = line.split_once(':')?;
                                k.eq_ignore_ascii_case("content-length")
                                    .then(|| v.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0);
                        if raw.len() >= end + content_len {
                            break;
                        }
                    }
                }
                let Some(end) = header_end else {
                    continue;
                };
                let head = String::from_utf8_lossy(&raw[..end]).into_owned();
                let body = &raw[end..];
                let first_line = head.lines().next().unwrap_or("");
                let mut parts = first_line.split_whitespace();
                let method = parts.next().unwrap_or("");
                let target = parts.next().unwrap_or("");

                let (content_type, resp_body): (&str, Vec<u8>) = if method == "GET"
                    && target.contains("alt=media")
                {
                    let guard = state.lock().unwrap();
                    ("application/zip", guard.zip_bytes.clone().unwrap_or_default())
                } else if method == "GET" && target.starts_with("/drive/v3/files/backup_1") {
                    let existing = {
                        let guard = state.lock().unwrap();
                        guard.sha256.clone().map(|sha| (sha, guard.version))
                    };
                    if let Some((sha, version)) = existing {
                        (
                            "application/json",
                            serde_json::to_vec(&json!({
                                "id": "backup_1",
                                "name": "eattura-backup.zip",
                                "version": version.to_string(),
                                "modifiedTime": format!("2026-11-25T11:0{version}:00Z"),
                                "trashed": false,
                                "appProperties": { "eatturaRole": "backup", "snapshotSha256": sha }
                            }))
                            .unwrap(),
                        )
                    } else {
                        let _ = socket
                            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                            .await;
                        continue;
                    }
                } else if method == "GET" && target.starts_with("/drive/v3/files?") {
                    let guard = state.lock().unwrap();
                    if target.contains("backup-folder") {
                        (
                            "application/json",
                            serde_json::to_vec(&json!({ "files": [] })).unwrap(),
                        )
                    } else if let Some(ref sha) = guard.sha256 {
                        (
                            "application/json",
                            serde_json::to_vec(&json!({
                                "files": [{
                                    "id": "backup_1",
                                    "name": "eattura-backup.zip",
                                    "version": guard.version.to_string(),
                                    "modifiedTime": format!("2026-11-25T11:0{}:00Z", guard.version),
                                    "trashed": false,
                                    "appProperties": { "eatturaRole": "backup", "snapshotSha256": sha }
                                }]
                            }))
                            .unwrap(),
                        )
                    } else {
                        (
                            "application/json",
                            serde_json::to_vec(&json!({ "files": [] })).unwrap(),
                        )
                    }
                } else if method == "POST" && target.starts_with("/drive/v3/files?") {
                    (
                        "application/json",
                        serde_json::to_vec(&json!({ "id": "folder_1", "name": "Eattura" })).unwrap(),
                    )
                } else if (method == "POST" || method == "PATCH")
                    && target.starts_with("/upload/drive/v3/files")
                {
                    // Extract ZIP part from multipart/related payload (starts at PK\x03\x04)
                    let zip_start = body.windows(4).position(|w| w == b"PK\x03\x04").unwrap();
                    let end_marker = body.windows(4).rposition(|w| w == b"\r\n--").unwrap();
                    let zip_slice = body[zip_start..end_marker].to_vec();
                    let (manifest, _) = ArchiveSnapshot::decode_zip(&zip_slice).unwrap();

                    let mut guard = state.lock().unwrap();
                    guard.version += 1;
                    if method == "POST" {
                        guard.uploads += 1;
                    } else {
                        guard.patches += 1;
                    }
                    guard.sha256 = Some(manifest.snapshot_sha256.clone());
                    guard.zip_bytes = Some(zip_slice);
                    (
                        "application/json",
                        serde_json::to_vec(&json!({
                            "id": "backup_1",
                            "name": "eattura-backup.zip",
                            "version": guard.version.to_string(),
                            "modifiedTime": format!("2026-11-25T11:0{}:00Z", guard.version),
                            "appProperties": {
                                "eatturaRole": "backup",
                                "snapshotSha256": manifest.snapshot_sha256
                            }
                        }))
                        .unwrap(),
                    )
                } else {
                    ("application/json", b"{}".to_vec())
                };

                let header = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    resp_body.len()
                );
                let _ = socket.write_all(header.as_bytes()).await;
                let _ = socket.write_all(&resp_body).await;
            }
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn single_backup_syncs_across_two_eattura_instances_via_metadata() {
        let fake_state = Arc::new(Mutex::new(FakeDriveState::default()));
        let base = spawn_fake_drive(fake_state.clone()).await;
        let api_base = format!("{base}/drive/v3/");
        let upload_base = format!("{base}/upload/drive/v3/");

        // Instance 1 creates a client + draft invoice and uploads the initial single backup.
        let dir1 = tempfile::tempdir().unwrap();
        let store1 = FileStore::open(dir1.path().join("eattura.json")).unwrap();
        store1
            .insert(
                "clients",
                json!({
                    "id": "c1",
                    "id_paese": "IT",
                    "id_codice": "12345678903",
                    "denominazione": "Alfa SRL",
                    "indirizzo": "Via Torino 5",
                    "cap": "20100",
                    "comune": "Milano",
                    "nazione": "IT"
                }),
            )
            .await
            .unwrap();
        store1
            .insert(
                "invoices",
                json!({
                    "id": "inv1",
                    "numero": "1",
                    "data": "2026-11-01",
                    "tipo_documento": "TD01",
                    "divisa": "EUR",
                    "stato": "draft",
                    "cedente_id": "c1",
                    "cessionario_id": "c1"
                }),
            )
            .await
            .unwrap();

        let drive = Drive {
            http: http_client().unwrap(),
            token: "token".into(),
            api_base: &api_base,
            upload_base: &upload_base,
        };
        let mut conn1 = Connection {
            client_id: "id".into(),
            client_secret: "secret".into(),
            refresh_token: "refresh".into(),
            account_id: "acct-1".into(),
            account_email: Some("utente@example.it".into()),
            folder_id: None,
            backup_file_id: None,
        };

        let first = sync_with_drive(&store1, &drive, &mut conn1).await.unwrap();
        assert_eq!(first.action, "uploaded");
        assert_eq!(first.invoices, 1);

        // Polling again with no changes uses metadata only (up_to_date, no extra upload).
        let noop = sync_with_drive(&store1, &drive, &mut conn1).await.unwrap();
        assert_eq!(noop.action, "up_to_date");
        assert_eq!(fake_state.lock().unwrap().uploads, 1);
        assert_eq!(fake_state.lock().unwrap().patches, 0);

        // Instance 2 opens empty with the same Google account and automatically downloads the backup!
        let dir2 = tempfile::tempdir().unwrap();
        let store2 = FileStore::open(dir2.path().join("eattura.json")).unwrap();
        let mut conn2 = Connection {
            client_id: "id".into(),
            client_secret: "secret".into(),
            refresh_token: "refresh".into(),
            account_id: "acct-1".into(),
            account_email: Some("utente@example.it".into()),
            folder_id: None,
            backup_file_id: None,
        };
        let pulled = sync_with_drive(&store2, &drive, &mut conn2).await.unwrap();
        assert_eq!(pulled.action, "downloaded");
        assert_eq!(pulled.invoices, 1);

        // Instance 2 adds a second invoice -> updates the SAME backup file in-place via PATCH!
        store2
            .insert(
                "invoices",
                json!({
                    "id": "inv2",
                    "numero": "2",
                    "data": "2026-11-02",
                    "tipo_documento": "TD01",
                    "divisa": "EUR",
                    "stato": "sent",
                    "cedente_id": "c1",
                    "cessionario_id": "c1"
                }),
            )
            .await
            .unwrap();
        let pushed = sync_with_drive(&store2, &drive, &mut conn2).await.unwrap();
        assert_eq!(pushed.action, "uploaded");
        assert_eq!(fake_state.lock().unwrap().uploads, 1);
        assert_eq!(fake_state.lock().unwrap().patches, 1);

        // Instance 1 runs its 5-minute check -> sees newer metadata and downloads the 2nd invoice!
        let refreshed = sync_with_drive(&store1, &drive, &mut conn1).await.unwrap();
        assert_eq!(refreshed.action, "downloaded");
        assert_eq!(refreshed.invoices, 2);
        let list = crate::commands::invoices::list_invoices_store(&store1)
            .await
            .unwrap();
        assert_eq!(list.len(), 2);
    }
}
