use clap::Parser;

#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
#[command(version)]
pub struct Cli {
    /// Config path
    #[arg(short, long)]
    pub config: Box<str>,
}
