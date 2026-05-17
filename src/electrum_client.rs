#![allow(unused)]
use std::time::Duration;

use bitcoincash::{Txid, hashes::hex::{FromHex, ToHex}};
use futures::{Stream, StreamExt};
use jsonrpsee::{
    core::{
        ClientError as Error,
        client::{ClientT, SubscriptionClientT},
        params::ArrayParams,
    },
    wasm_client::Client,
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

impl<T> ElectrumClient<Client<T>>
where
    Client<T>: ClientT + SubscriptionClientT,
{
    /// The `server.version` method.
    pub async fn server_version(&self, client_name: &str) -> Result<ServerVersionResponse, Error> {
        let [server_software_version, protocol_version]: [String; 2] = self
            .client
            .request("server.version", (client_name, ["1.5", "1.6"]))
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
        Ok((result, subscription.map(|x| Ok(x.map(|(y,)| y)?))))
    }

    /// The `server.ping` method.
    pub async fn server_ping(&self) -> Result<(), Error> {
        let _: Option<()> = self
            .client
            .request("server.ping", ArrayParams::new())
            .await?;
        Ok(())
    }

    /// The `blockchain.transaction.get` method.
    pub async fn blockchain_transaction_get(&self, txid: Txid) -> Result<Vec<u8>, Error> {
        let tx_hex: String = self
            .client
            .request("blockchain.transaction.get", (&txid.to_hex(),))
            .await?;
        Vec::from_hex(&tx_hex)
            .map_err(|e| Error::Custom(format!("invalid rpc response: {e}")))
    }

    pub fn new(client: Client<T>) -> Self {
        Self { client }
    }

    pub fn on_disconnect(&self) -> impl Future<Output = Error> {
        self.client.on_disconnect()
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
