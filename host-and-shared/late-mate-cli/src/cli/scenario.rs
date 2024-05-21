mod example;
mod run;

#[derive(Debug, clap::Subcommand)]
pub enum Scenario {
    /// Run a latency testing scenario
    Run(run::Args),
    /// Scenario examples in JSON and TOML
    Example(example::Args),
}
