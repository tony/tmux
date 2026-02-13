use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    source: Option<String>,
}

fn main() {
    let _ = Args::parse();
}
