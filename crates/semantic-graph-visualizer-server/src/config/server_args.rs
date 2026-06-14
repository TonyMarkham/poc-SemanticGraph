use clap::Parser;
use std::{net::SocketAddr, path::PathBuf};

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub(crate) struct ServerArgs {
    #[arg(long)]
    pub(crate) database_path: Option<PathBuf>,

    #[arg(long)]
    pub(crate) bind: Option<SocketAddr>,
}
