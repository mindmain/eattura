use std::io;

use hmac::{Hmac, Mac};
use secrecy::{ExposeSecret, SecretString};
use sha2::Sha256;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[cfg(unix)]
pub(super) type Stream = tokio::net::UnixStream;
#[cfg(windows)]
pub(super) type Stream = tokio::net::windows::named_pipe::NamedPipeClient;

fn proof(
    token: &SecretString,
    role: &[u8],
    client: &[u8],
    server: &[u8],
) -> io::Result<Hmac<Sha256>> {
    let mut mac = Hmac::<Sha256>::new_from_slice(token.expose_secret().as_bytes())
        .map_err(|_| io::Error::other("Invalid IPC credential"))?;
    mac.update(b"eattura-mcp-ipc-v1");
    mac.update(role);
    mac.update(client);
    mac.update(server);
    Ok(mac)
}

fn nonce() -> [u8; 32] {
    let mut bytes = [0; 32];
    bytes[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    bytes
}

pub(super) async fn authenticate_server<Stream: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut Stream,
    token: &SecretString,
) -> io::Result<()> {
    let mut client = [0; 32];
    stream.read_exact(&mut client).await?;
    let server = nonce();
    stream.write_all(&server).await?;
    stream
        .write_all(
            &proof(token, b"server", &client, &server)?
                .finalize()
                .into_bytes(),
        )
        .await?;
    stream.flush().await?;
    let mut received = [0; 32];
    stream.read_exact(&mut received).await?;
    proof(token, b"client", &client, &server)?
        .verify_slice(&received)
        .map_err(|_| {
            io::Error::new(io::ErrorKind::PermissionDenied, "IPC authentication failed")
        })?;
    stream.write_all(&[1]).await?;
    stream.flush().await
}

pub(super) async fn authenticate_client<Stream: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut Stream,
    token: &SecretString,
) -> io::Result<()> {
    let client = nonce();
    stream.write_all(&client).await?;
    stream.flush().await?;
    let mut server = [0; 32];
    let mut received = [0; 32];
    stream.read_exact(&mut server).await?;
    stream.read_exact(&mut received).await?;
    proof(token, b"server", &client, &server)?
        .verify_slice(&received)
        .map_err(|_| {
            io::Error::new(io::ErrorKind::PermissionDenied, "IPC authentication failed")
        })?;
    stream
        .write_all(
            &proof(token, b"client", &client, &server)?
                .finalize()
                .into_bytes(),
        )
        .await?;
    stream.flush().await?;
    let mut accepted = [0];
    stream.read_exact(&mut accepted).await?;
    if accepted != [1] {
        return Err(io::Error::other("IPC authentication failed"));
    }
    Ok(())
}

#[cfg(unix)]
pub(super) struct Listener {
    listener: tokio::net::UnixListener,
    directory: std::path::PathBuf,
    endpoint: String,
}

#[cfg(unix)]
impl Listener {
    pub(super) fn bind() -> io::Result<Self> {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        let temporary = std::env::temp_dir();
        let base = if temporary.as_os_str().len() > 40 {
            std::path::Path::new("/tmp")
        } else {
            &temporary
        };
        let directory = base.join(format!("eattura-mcp-{}", uuid::Uuid::new_v4().simple()));
        std::fs::DirBuilder::new().mode(0o700).create(&directory)?;
        let path = directory.join("app.sock");
        let result = (|| {
            let listener = tokio::net::UnixListener::bind(&path)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            let endpoint = path
                .to_str()
                .ok_or_else(|| io::Error::other("IPC path must be UTF-8"))?
                .into();
            Ok(Self {
                listener,
                directory: directory.clone(),
                endpoint,
            })
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&path);
            let _ = std::fs::remove_dir(&directory);
        }
        result
    }

    pub(super) fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub(super) async fn accept(&mut self) -> io::Result<tokio::net::UnixStream> {
        self.listener.accept().await.map(|(stream, _)| stream)
    }
}

#[cfg(unix)]
impl Drop for Listener {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.endpoint);
        let _ = std::fs::remove_dir(&self.directory);
    }
}

#[cfg(unix)]
pub(super) async fn connect(endpoint: &str) -> io::Result<tokio::net::UnixStream> {
    if !std::path::Path::new(endpoint).is_absolute() {
        return Err(io::Error::other("Invalid local IPC endpoint"));
    }
    tokio::net::UnixStream::connect(endpoint).await
}

#[cfg(windows)]
pub(super) struct Listener {
    pending: tokio::net::windows::named_pipe::NamedPipeServer,
    endpoint: String,
}

#[cfg(windows)]
impl Listener {
    pub(super) fn bind() -> io::Result<Self> {
        let endpoint = format!(r"\\.\pipe\eattura-mcp-{}", uuid::Uuid::new_v4().simple());
        let pending = tokio::net::windows::named_pipe::ServerOptions::new()
            .first_pipe_instance(true)
            .reject_remote_clients(true)
            .create(&endpoint)?;
        Ok(Self { pending, endpoint })
    }

    pub(super) fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub(super) async fn accept(
        &mut self,
    ) -> io::Result<tokio::net::windows::named_pipe::NamedPipeServer> {
        self.pending.connect().await?;
        let next = tokio::net::windows::named_pipe::ServerOptions::new()
            .reject_remote_clients(true)
            .create(&self.endpoint)?;
        Ok(std::mem::replace(&mut self.pending, next))
    }
}

#[cfg(windows)]
pub(super) async fn connect(
    endpoint: &str,
) -> io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    if !endpoint.starts_with(r"\\.\pipe\eattura-mcp-") {
        return Err(io::Error::other("Invalid local IPC endpoint"));
    }
    loop {
        match tokio::net::windows::named_pipe::ClientOptions::new().open(endpoint) {
            Err(error) if error.raw_os_error() == Some(231) => {
                tokio::time::sleep(std::time::Duration::from_millis(25)).await
            }
            result => return result,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mutual_authentication_rejects_another_users_token_in_both_directions() {
        for same_token in [true, false] {
            let (mut client, mut server) = tokio::io::duplex(1024);
            let server_task = tokio::spawn(async move {
                authenticate_server(&mut server, &"a".repeat(64).into()).await
            });
            let token = if same_token { "a" } else { "b" }.repeat(64).into();
            let result = authenticate_client(&mut client, &token).await;
            assert_eq!(result.is_ok(), same_token);
            drop(client);
            assert_eq!(server_task.await.unwrap().is_ok(), same_token);
        }
    }

    #[test]
    fn proofs_are_role_bound_and_cannot_be_replayed_for_a_new_session() {
        let token: SecretString = "a".repeat(64).into();
        let client = nonce();
        let server = nonce();
        let captured = proof(&token, b"server", &client, &server)
            .unwrap()
            .finalize()
            .into_bytes();
        assert!(
            proof(&token, b"client", &client, &server)
                .unwrap()
                .verify_slice(&captured)
                .is_err()
        );
        assert!(
            proof(&token, b"server", &client, &nonce())
                .unwrap()
                .verify_slice(&captured)
                .is_err()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn unix_socket_is_private_and_removed_when_listener_closes() {
        use std::os::unix::fs::PermissionsExt;
        let listener = Listener::bind().unwrap();
        let path = std::path::PathBuf::from(listener.endpoint());
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        drop(listener);
        assert!(!path.exists());
        assert!(!path.parent().unwrap().exists());
    }
}
