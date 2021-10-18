mod error;
mod generate;
mod handlers;
mod opts;
mod server;
mod util;

pub use crate::error::*;

use crate::opts::{Opts, Subcommand};
use clap::Clap;

fn main() {
    let opts = Opts::parse();

    match opts.subcmd {
        Subcommand::Gen(o) => crate::generate::exec(&o),
        Subcommand::Server(o) => crate::server::exec(o),
    }
}
