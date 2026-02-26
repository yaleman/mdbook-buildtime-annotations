use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about, version)]
pub struct CliOpts {
    #[command(subcommand)]
    pub cmd: Option<Cmd>,
}

#[derive(Subcommand)]
pub enum Cmd {
    /// Check whether a renderer is supported by this preprocessor
    Supports { renderer: String },
}

// Grabbed from <https://github.com/rust-lang/mdBook/blob/master/src/main.rs#L93>
pub fn init_logger() {
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_env_var("MDBOOK_LOG")
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .from_env_lossy();
    let log_env = std::env::var("MDBOOK_LOG");
    // Silence some particularly noisy dependencies unless the user
    // specifically asks for them.
    let silence_unless_specified = |filter: tracing_subscriber::EnvFilter, target| {
        if !log_env
            .as_ref()
            .is_ok_and(|s| s.split(',').any(|directive| directive.starts_with(target)))
        {
            filter.add_directive(format!("{target}=warn").parse().unwrap())
        } else {
            filter
        }
    };
    let filter = silence_unless_specified(filter, "handlebars");
    let filter = silence_unless_specified(filter, "html5ever");

    // Don't show the target by default, since it generally isn't useful
    // unless you are overriding the level.
    let with_target = log_env.is_ok();

    tracing_subscriber::fmt()
        .without_time()
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .with_target(with_target)
        .init();
}
