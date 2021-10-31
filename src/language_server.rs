use std::sync::Arc;

use log::info;
use tower_lsp::{
    lsp_types::{InitializeResult, ServerInfo},
    Client,
};

use crate::capabilities;

#[derive(Debug, Clone)]
pub struct LanguageServer(Arc<tokio::sync::Mutex<Inner>>);

impl LanguageServer {
    pub fn new(client: Client) -> Self {
        Self(Arc::new(tokio::sync::Mutex::new(Inner::new(client))))
    }

    fn inner(&self) -> &Arc<tokio::sync::Mutex<Inner>> {
        &self.0
    }
}
#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for LanguageServer {
    async fn initialize(
        &self,
        params: tower_lsp::lsp_types::InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<tower_lsp::lsp_types::InitializeResult> {
        self.inner().lock().await.initialize(params).await
    }

    async fn shutdown(&self) -> tower_lsp::jsonrpc::Result<()> {
        todo!()
    }
}

#[derive(Debug)]
pub struct Inner {
    /// The LSP client that this LSP server is connected to.
    client: Client,
}

impl Inner {
    fn new(client: Client) -> Self {
        Self { client }
    }

    async fn initialize(
        &self,
        params: tower_lsp::lsp_types::InitializeParams,
    ) -> tower_lsp::jsonrpc::Result<InitializeResult> {
        let capabilities = capabilities::server_capabilities(&params.capabilities);
        let server_info = ServerInfo {
            name: "todome-language-server".to_owned(),
            version: Some(crate::version()),
        };

        if let Some(client_info) = params.client_info {
            info!(
                "Connected to \"{}\" {}",
                client_info.name,
                client_info.version.unwrap_or_default(),
            );
        }

        Ok(InitializeResult {
            capabilities,
            server_info: Some(server_info),
        })
    }
}
