use std::{convert::Infallible, net::{Ipv4Addr, SocketAddr}};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use hyper::service::service_fn;
use http_body_util::Full;
use hyper::{Request, Response, body::Bytes, server::conn::http1};
use hyper_util::rt::{TokioIo, TokioTimer};
use rand::{Rng, distr::Alphanumeric, rng};
use sha2::Sha256;
use sha2::Digest;
use tokio::net::TcpListener;
use async_channel::unbounded;

#[derive(Clone, Debug)]
pub enum ServerError {
    NoQueryOnCallback,
    MalformedCallback,
    MalformedCSRF,
    InvalidCSRF
}

use crate::error::{ChannelError, Res};

const CSRF_CHARSET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
      abcdefghijklmnopqrstuvwxyz\
      0123456789";

pub fn generate_csrf() -> String {
    let mut rng = rng();
    (0..32)
        .map(|_| {
            let id = rng.random_range(0..CSRF_CHARSET.len());
            CSRF_CHARSET[id] as char
        }).collect()
}

/// Create dual PKCE codes.
/// The verifier is stored in memory and the challenge is sent in the URL. Later, the client can be verified by checking the hash of verifier against the challenge.
pub fn generate_pkce() -> (String, String) {
    let verifier: String = rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    // Perform a SHA256 hash on the verifier.
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hashed = hasher.finalize();
    let challenge = BASE64_URL_SAFE_NO_PAD.encode(hashed);

    (verifier, challenge)
}

pub fn process_callback(request: Request<impl hyper::body::Body>, csrf: String) -> Res<String> {
    let query = request.uri().query().ok_or(ServerError::NoQueryOnCallback)?;
    let mut parts = query.split("&");

    let code_substr = parts.next().ok_or(ServerError::MalformedCallback)?;
    let state_substr = parts.next().ok_or(ServerError::MalformedCSRF)?;

    let code = code_substr.strip_prefix("code=").ok_or(ServerError::MalformedCallback)?.to_string();
    let returned_csrf = state_substr.strip_prefix("state=").ok_or(ServerError::MalformedCSRF)?.to_string();

    if returned_csrf == csrf {
        Ok(code)
    } else {
        Err(ServerError::InvalidCSRF.into())
    }
}

pub async fn run_server(csrf: String) -> Res<String> {
    let addr: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000u16);
    let listener = TcpListener::bind(addr).await?;

    let (tcp, _) = listener.accept().await?;
    let io = TokioIo::new(tcp);
    let (sender, receiver) = unbounded();
    let (shutdown_sender, shutdown_receiver) = unbounded::<()>();

    let connection = http1::Builder::new()
        .timer(TokioTimer::new())
        .serve_connection(
            io,
            service_fn({
                async |req| {
                    let csrf_instance = csrf.clone();
                    let res = process_callback(req, csrf_instance);
                    let _ = sender.send(res).await;
                    let _ = shutdown_sender.send(()).await;
                    Ok::<_, Infallible>(Response::new(Full::new(Bytes::from("Received code. Close this tab."))))
                }
            })
        );


    tokio::select! {
        result = connection => {
            result?;
        }

        _ = shutdown_receiver.recv() => {}
    }

    match receiver.try_recv() {
        Ok(code) => code,
        Err(e) => Err(ChannelError::from(e).into())
    }
}
