use std::sync::Arc;

use log::{debug, error, info, warn};
use tower_lsp::{
    jsonrpc::ErrorCode,
    lsp_types::{CompletionList, CompletionResponse, InitializeResult, ServerInfo},
    Client,
};

use crate::structure::syntax::DocumentCache;

mod capabilities;
mod completion;
mod diagnostics;

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
        Ok(())
    }

    async fn did_open(&self, params: tower_lsp::lsp_types::DidOpenTextDocumentParams) {
        self.inner().lock().await.did_open(params).await;
    }

    async fn did_change(&self, params: tower_lsp::lsp_types::DidChangeTextDocumentParams) {
        self.inner().lock().await.did_change(params).await;
    }

    async fn did_save(&self, params: tower_lsp::lsp_types::DidSaveTextDocumentParams) {
        self.inner().lock().await.did_save(params).await;
    }

    async fn did_close(&self, params: tower_lsp::lsp_types::DidCloseTextDocumentParams) {
        self.inner().lock().await.did_close(params).await;
    }
    async fn completion(
        &self,
        params: tower_lsp::lsp_types::CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<tower_lsp::lsp_types::CompletionResponse>> {
        self.inner().lock().await.completion(params).await
    }
}

#[derive(Debug)]
pub struct Inner {
    /// The LSP client that this LSP server is connected to.
    client: Client,
    document_cache: DocumentCache,
}

impl Inner {
    fn new(client: Client) -> Self {
        Self {
            client,
            document_cache: DocumentCache::default(),
        }
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

    async fn did_open(&mut self, params: tower_lsp::lsp_types::DidOpenTextDocumentParams) {
        info!("called did_open");
        let url = params.text_document.uri;
        let text = params.text_document.text;
        match self.document_cache.register_or_update(&url, text) {
            Ok(document) => {
                let diags = document.get_diagnostics();
                self.client.publish_diagnostics(url, diags, None).await;
            }
            Err(e) => {
                error!("Failed to register document {}", url);
                error!("{}", e);
            }
        }
    }

    async fn did_change(&mut self, mut params: tower_lsp::lsp_types::DidChangeTextDocumentParams) {
        info!("called did_change");
        let url = params.text_document.uri;
        // full changes を仮定
        if params.content_changes.get(0).is_some() {
            let text = params.content_changes.swap_remove(0).text;
            match self.document_cache.register_or_update(&url, text) {
                Ok(document) => {
                    let diags = document.get_diagnostics();
                    self.client.publish_diagnostics(url, diags, None).await;
                }
                Err(e) => {
                    error!("Failed to register document {}", url);
                    error!("{}", e);
                }
            }
        }
    }

    async fn did_save(&mut self, params: tower_lsp::lsp_types::DidSaveTextDocumentParams) {
        info!("called did_save");
        let url = params.text_document.uri;
        if let Some(document) = self.document_cache.get(&url) {
            debug!("{}", document);
            let diags = document.get_diagnostics();
            self.client.publish_diagnostics(url, diags, None).await;
        }
    }

    async fn did_close(&mut self, _params: tower_lsp::lsp_types::DidCloseTextDocumentParams) {
        info!("called did_close");
    }

    async fn completion(
        &mut self,
        params: tower_lsp::lsp_types::CompletionParams,
    ) -> tower_lsp::jsonrpc::Result<Option<tower_lsp::lsp_types::CompletionResponse>> {
        let url = params.text_document_position.text_document.uri.clone();
        if let Some(document) = self.document_cache.get(&url) {
            let completions =
                document
                    .get_completion(&params)
                    .map_err(|e| tower_lsp::jsonrpc::Error {
                        code: ErrorCode::InternalError,
                        message: format!("{}", e),
                        data: None,
                    })?;
            debug!("completions: {:#?}", completions);
            Ok(Some(CompletionResponse::Array(completions)))
        } else {
            warn!("Document not found.");
            Ok(None)
        }
    }
}
