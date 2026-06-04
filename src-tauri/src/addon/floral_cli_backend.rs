#[path = "floral_cli_backend/direct.rs"]
mod direct;
#[path = "floral_cli_backend/mcp.rs"]
mod mcp;
use floral_notepaper_lib::services::{
    addon::{OperationService, ADDON_PROTOCOL_VERSION},
    notes::{default_store, AppError, NoteStore},
};
use std::{env, path::PathBuf};

#[tokio::main]
async fn main() {
    let mut arguments = env::args().skip(1);
    match arguments.next().as_deref() {
        Some("--version") => {
            println!(
                "floral_cli_backend {} protocol {}",
                env!("CARGO_PKG_VERSION"),
                ADDON_PROTOCOL_VERSION
            );
        }
        Some("mcp") => match mcp_service(arguments.collect()) {
            Ok(service) => {
                if let Err(error) = mcp::serve(service).await {
                    eprintln!("MCP server stopped: {error}");
                    std::process::exit(1);
                }
            }
            Err(error) => {
                eprintln!("{}", error.message);
                std::process::exit(1);
            }
        },
        Some(operation) => direct::run(default_service(), operation),
        None => direct::run(
            Err(app_error(
                "missingOperation",
                "A backend operation is required.",
            )),
            "",
        ),
    }
}

fn default_service() -> Result<OperationService, AppError> {
    Ok(OperationService::new(default_store()?))
}

fn mcp_service(arguments: Vec<String>) -> Result<OperationService, AppError> {
    match arguments.as_slice() {
        [] => default_service(),
        [flag, path] if flag == "--data-dir" && !path.trim().is_empty() => Ok(
            OperationService::new(NoteStore::new(PathBuf::from(path.trim()))),
        ),
        _ => Err(app_error(
            "invalidArguments",
            "Usage: floral_cli_backend mcp [--data-dir PATH]",
        )),
    }
}

fn app_error(code: impl Into<String>, message: impl Into<String>) -> AppError {
    AppError {
        code: code.into(),
        message: message.into(),
        details: Default::default(),
    }
}
