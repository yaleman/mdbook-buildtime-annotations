use clap::Parser;
use mdbook_buildtime_annotations::{
    Processor,
    cli::{CliOpts, Cmd, init_logger},
    handle_preprocessing, handle_supports,
};
use mdbook_preprocessor::Preprocessor;
use tracing::error;

pub fn main() {
    init_logger();
    let app = CliOpts::parse();
    let processor = Processor;

    if let Some(Cmd::Supports { renderer }) = app.cmd {
        handle_supports(processor, &renderer);
    } else if let Err(e) = handle_preprocessing() {
        error!("{} failed to handle preprocessing: {}", processor.name(), e);
        std::process::exit(1);
    }
}
