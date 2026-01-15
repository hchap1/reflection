use std::{convert::Infallible, net::{Ipv4Addr, SocketAddr}};
use hyper::service::service_fn;
use http_body_util::Full;
use hyper::{Request, Response, body::Bytes, server::conn::http1};
use hyper_util::rt::{TokioIo, TokioTimer};
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
