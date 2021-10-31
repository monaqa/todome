use anyhow::*;
use structopt::StructOpt;
use tokio::net::TcpListener;
use tower_lsp::{LspService, Server};

#[derive(Debug, Clone, StructOpt)]
struct Opts {
    #[structopt(long)]
    tcp: bool,
    #[structopt(short, long, default_value = "9527")]
    port: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::from_args();

    if opts.tcp {
        if std::env::var("RUST_LOG").unwrap_or_default().is_empty() {
            std::env::set_var("RUST_LOG", "info");
        }
        env_logger::init();

        let listener = TcpListener::bind(format!("127.0.0.1:{}", opts.port)).await?;
        let (stream, _) = listener.accept().await?;
        let (read, write) = tokio::io::split(stream);
        let (service, messages) = LspService::new(todome::language_server::LanguageServer::new);
        Server::new(read, write)
            .interleave(messages)
            .serve(service)
            .await;
    } else {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, messages) = LspService::new(todome::language_server::LanguageServer::new);
        Server::new(stdin, stdout)
            .interleave(messages)
            .serve(service)
            .await;
    }
    Ok(())
}
