use std::{convert::Infallible, net::{Ipv4Addr, SocketAddr}};
use hyper::service::service_fn;
use http_body_util::Full;
use hyper::{Request, Response, body::{Body, Bytes}, server::conn::http1};
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::TcpListener;
use async_channel::unbounded;

#[derive(Clone, Debug)]
pub enum ServerError {
    NoQueryOnCallback
}

use crate::error::{Error, Res};

pub fn process_callback(request: Request<impl hyper::body::Body>) -> Res<()> {
    let query = request.uri().query().ok_or(ServerError::NoQueryOnCallback)?;
    println!("QUERY: {query}");
    Ok(())
}

pub async fn run_server() -> Res<()> {
    let addr: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000u16);
    let listener = TcpListener::bind(addr).await?;

    let (tcp, _) = listener.accept().await?;
    let io = TokioIo::new(tcp);
    let (sender, receiver) = unbounded();

    http1::Builder::new()
        .timer(TokioTimer::new())
        .serve_connection(
            io,
            service_fn(
                async |req| {
                    let res = process_callback(req);
                    let _ = sender.send(res).await;
                    Ok::<_, Infallible>(Response::new(Full::new(Bytes::from("Received code. Close this tab."))))
            })
        ).await?;

    let code = receiver.try_recv()?;
}
