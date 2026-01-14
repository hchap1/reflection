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

// Ensure the CSRF meets formatting requirements by limiting scope.
const CSRF_CHARSET: &[u8] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
      abcdefghijklmnopqrstuvwxyz\
      0123456789";

/// Generate a random string that is echoed back throughout OAUTH2 processes to validate authenticity of foreign servers. If this code is different, terminate.
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
        // 64 is the standard
        .take(64)
        .map(char::from)
        .collect();

    // Perform a SHA256 hash on the verifier.
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hashed = hasher.finalize();

    // Base64 encode the SHA256 hash as expected by the API, ending with a 43-byte string with URL protocol.
    let challenge = BASE64_URL_SAFE_NO_PAD.encode(hashed);

    // Return both the original PKCE and the hashed challenge.
    // Even if the original hash is intercepted, the verifier cannot be derived and thus an attacker cannot complete the authentication.
    (verifier, challenge)
}

/// Simple string parse to collect the callback CODE and STATE (CSRF) from the POST to localhost.
pub fn process_callback(request: Request<impl hyper::body::Body>, csrf: String) -> Res<String> {
    let query = request.uri().query().ok_or(ServerError::NoQueryOnCallback)?;
    let mut parts = query.split("&");

    // If the POST to localhost did not contain the correct headers, something has gone very wrong.
    let code_substr = parts.next().ok_or(ServerError::MalformedCallback)?;
    let state_substr = parts.next().ok_or(ServerError::MalformedCSRF)?;

    let code = code_substr.strip_prefix("code=").ok_or(ServerError::MalformedCallback)?.to_string();
    let returned_csrf = state_substr.strip_prefix("state=").ok_or(ServerError::MalformedCSRF)?.to_string();

    // The CSRF ensures that the response from the server is the expected response to the request sent.
    // It is echoed back ensuring that the returned code was not spoofed through man in the middle.
    // Added benefit of making sure that a specific callback is tied to a specific oauth instance.
    if returned_csrf == csrf {
        Ok(code)
    } else {
        Err(ServerError::InvalidCSRF.into())
    }
}

pub async fn run_server(csrf: String) -> Res<String> {

    // Bind the listener to localhost:3000. This is accessible to microsoft through the browswer.
    let addr: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000u16);
    let listener = TcpListener::bind(addr).await?;

    // Map traffic into TCP packets that can be parsed by tokio io.
    let (tcp, _) = listener.accept().await?;
    let io = TokioIo::new(tcp);

    // Sender/receiver pair to asynchronously retrieve the response from the callback.
    let (sender, receiver) = unbounded();

    // Sender/receiver pair to await an outside event. Probably not the idiomatic approach but it works.
    let (shutdown_sender, shutdown_receiver) = unbounded::<()>();

    let connection = http1::Builder::new()
        .timer(TokioTimer::new())

        // Serve one singular connection
        .serve_connection(
            io,
            
            // Serve many requests from one connection (although in this instance, it will quit after one)
            service_fn({
                async |req| {
                    let csrf_instance = csrf.clone();

                    // Parse the request
                    let res = process_callback(req, csrf_instance);
                    let success = res.is_ok();

                    // Pipe the request out of this async closure context.
                    let pipe_success = sender.send(res).await.is_ok();

                    // Signal the termination of the thread by breaking tokio::select.
                    let shutdown_success = shutdown_sender.send(()).await.is_ok();

                    if success && pipe_success && shutdown_success {
                        Ok::<_, Infallible>(Response::new(Full::new(Bytes::from("Received code. Close this tab."))))
                    } else {
                        Ok::<_, Infallible>(Response::new(Full::new(Bytes::from("
                                Critical Error.
                                Unable to ascertain the code, OR failed to extract code from async context OR failed to signal shutdown of code thread.
                                Recommended action: Restart authentication process.
                        "))))
                    }
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
