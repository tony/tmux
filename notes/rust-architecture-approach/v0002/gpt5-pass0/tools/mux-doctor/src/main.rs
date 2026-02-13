use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    full: bool,
}

fn main() {
    let _ = Args::parse();
}
