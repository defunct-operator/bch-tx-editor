#![allow(unused)]
use std::time::Duration;

use futures::{Stream, StreamExt};
use jsonrpsee::core::{
    client::{ClientT, SubscriptionClientT},
    params::ArrayParams,
    ClientError as Error,
};

/// Wrapper that adds convenience methods for interacting with the [Electrum Cash
/// Protocol](https://electrum-cash-protocol.readthedocs.io/en/latest/index.html).
#[derive(Debug)]
pub struct ElectrumClient<T> {
    pub client: T,
}

#[derive(Debug)]
pub struct ServerVersionResponse {
    pub server_software_version: String,
    /// The ElectrumX protocol version that will be used
    pub protocol_version: String,
}

#[derive(serde::Deserialize, Debug)]
pub struct BlockHeaders {
    pub height: i64,
    pub hex: String,
}

impl<T: ClientT + SubscriptionClientT> ElectrumClient<T> {
    /// The `server.version` method.
    pub async fn server_version(&self, client_name: &str) -> Result<ServerVersionResponse, Error> {
        let [server_software_version, protocol_version]: [String; 2] = self
            .client
            .request("server.version", (client_name, ["1.2", "1.4"]))
            .await?;
        Ok(ServerVersionResponse {
            server_software_version,
            protocol_version,
        })
    }

    /// The `blockchain.headers.subscribe` method.
    ///
    /// Returns the headers of the current block tip and a stream of block headers from the
    /// subscription.
    pub async fn blockchain_headers_subscribe(
        &self,
    ) -> Result<
        (
            BlockHeaders,
            impl Stream<Item = Result<BlockHeaders, Error>>,
        ),
        Error,
    > {
        let subscription = self
            .client
            .subscribe_to_method::<(BlockHeaders,)>("blockchain.headers.subscribe")
            .await
            .unwrap();
        let result: BlockHeaders = self
            .client
            .request("blockchain.headers.subscribe", ArrayParams::new())
            .await?;
        Ok((result, subscription.map(|x| x.map(|(y,)| y))))
    }

    /// The `server.ping` method.
    pub async fn server_ping(&self) -> Result<(), Error> {
        let _: Option<()> = self
            .client
            .request("server.ping", ArrayParams::new())
            .await?;
        Ok(())
    }

    pub fn new(client: T) -> Self {
        Self { client }
    }

    pub async fn ping_loop(&self) {
        loop {
            gloo::timers::future::sleep(Duration::from_secs(60)).await;
            let ping_result = self.server_ping().await;
            if let Err(e) = ping_result {
                leptos::logging::error!("Ping failed: {e:?}");
            }
        }
    }
}
