use clap::Parser;

#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
#[command(version)]
pub struct Cli {
    /// Db path
    #[arg(short, long)]
    pub db: Box<str>,
    /// Server address
    #[arg(short, long)]
    pub addr: Box<str>,
}
