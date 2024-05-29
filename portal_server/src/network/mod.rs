use futures::future::select_ok;
use futures::{FutureExt, TryStreamExt};
use std::net::{IpAddr, SocketAddr};
use thiserror::Error;
mod server;
pub use self::server::spawn;
mod proxy;
pub use self::proxy::proxy_stream;
use crate::network::server::{HostQuery, HostQueryResponse};
use crate::{get_config, ClientId};
use reqwest::StatusCode;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::error::Error as WsError;
use tokio_tungstenite::{connect_async, WebSocketStream};
use trust_dns_resolver::TokioAsyncResolver;
use crate::control_server::{SinkExt, StreamExt};

#[derive(Error, Debug)]
pub enum Error {
    #[error("IOError: {0}")]
    Io(#[from] std::io::Error),

    #[error("RequestError: {0}")]
    Request(#[from] reqwest::Error),

    #[error("ResolverError: {0}")]
    Resolver(#[from] trust_dns_resolver::error::ResolveError),

    #[error("Does not serve host")]
    DoesNotServeHost,

}

/// An instance of our server
#[derive(Debug, Clone)]
pub struct Instance {
    pub ip: IpAddr,
}

impl Instance {
    /// get all instances where our app runs
    async fn get_instances() -> Result<Vec<Instance>, Error> {
        let query = if let Some(dns) = get_config().gossip_dns_host.clone() {
            dns
        } else {
            tracing::warn!("warning! gossip mode disabled!");
            return Ok(vec![]);
        };

        tracing::debug!("querying app instances");

        let resolver = TokioAsyncResolver::tokio_from_system_conf()?;

        let ips = resolver.lookup_ip(query).await?;

        let instances = ips.iter().map(|ip| Instance { ip }).collect();
        tracing::debug!("Found app instances: {:?}", &instances);
        Ok(instances)
    }


    /// query the instance and see if it runs our host
    async fn serves_host(self, host: &str) -> Result<(Instance, ClientId), Error> {
        let addr = SocketAddr::new(self.ip, get_config().internal_network_port);
        let url = format!("http://{}", addr);
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .timeout(std::time::Duration::from_secs(2))
            .query(&HostQuery {
                host: host.to_string(),
            })
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error=?e, "failed to send a host query");
                e
            })?;
        let status = response.status();
        let result: HostQueryResponse = response.json().await?;

        let found_client = result
            .client_id
            .as_ref()
            .map(|c| c.to_string())
            .unwrap_or_default();
        tracing::debug!(status=%status, found=%found_client, "got net svc response");

        match (status, result.client_id) {
            (StatusCode::OK, Some(client_id)) => Ok((self, client_id)),
            _ => Err(Error::DoesNotServeHost),
        }
    }
    // async fn serves_websocket_host(self, host: &str) -> Result<(Instance, ClientId), Error> {
    //     let addr = SocketAddr::new(self.ip, get_config().internal_network_port);
    //     let url = format!("ws://{}", addr);
    //     let (mut ws_stream, _) = connect_async(url).await.map_err(|e| {
    //         tracing::error!(error=?e, "failed to establish WebSocket connection");
    //         e.into()
    //     })?;
    //
    //     let request = WsMessage::Text(HostQuery { host: host.to_string() }.to_json()?);
    //     ws_stream.send(request).await.map_err(|e| {
    //         tracing::error!(error=?e, "failed to send a host query over WebSocket");
    //         e.into()
    //     })?;
    //
    //     let response = ws_stream
    //         .try_next()
    //         .await
    //         .ok_or(Error::DoesNotServeHost)?
    //         .map_err(|e| {
    //             tracing::error!(error=?e, "failed to receive a response over WebSocket");
    //             e.into()
    //         })?;
    //
    //     if let WsMessage::Text(text) = response {
    //         let result: HostQueryResponse = serde_json::from_str(&text)?;
    //         let found_client = result.client_id.unwrap_or_default();
    //
    //         tracing::debug!("got WebSocket response: {:?}", result);
    //         Ok((self, found_client))
    //     }
    // }
}
// pub async fn pywebsocket(instance: Instance,mut stream:WebSocketStream<TcpStream>){
//     let url = format!("ws://{}:{}", instance.ip, get_config().remote_port);
//     let (mut ws_stream) = match tokio_tungstenite::connect_async(url).await {
//         Ok((stream, _)) => (stream),
//     };
//
//     let (mut ws_read, mut ws_write) = ws_stream.split();
//     let (mut r_read, mut r_write) = stream.split();
//     let _ = futures::future::join(
//         r_read.forward(ws_write),
//         ws_read.forward(r_write),
//     )
//         .await;
// }

/// get the ip address we need to connect to that runs our host
#[tracing::instrument]
pub async fn instance_for_host(host: &str) -> Result<(Instance, ClientId), Error> {
    let instances = Instance::get_instances()
        .await?
        .into_iter()
        .map(|i| i.serves_host(host).boxed());
        // .map(|i| async {
        //     let serves_host = i.clone().serves_host(host).boxed();
        //     let serves_websocket_host = i.serves_websocket_host(host).boxed();
        //     futures::try_join!(serves_host)
        // });

    if instances.len() == 0 {
        return Err(Error::DoesNotServeHost);
    }
    let instance = select_ok(instances).await?.0;
    tracing::info!(instance_ip=%instance.0.ip, client_id=%instance.1.to_string(), subdomain=%host, "found instance for host");
    // let instance = instances.into_iter().find_map(Result::ok).ok_or(Error::DoesNotServeHost)?;
    // tracing::info!(instance_ip=%instance.0.ip, client_id=%instance.1.to_string(), subdomain=%host, "found instance for WebSocket host");
    Ok(instance)
}