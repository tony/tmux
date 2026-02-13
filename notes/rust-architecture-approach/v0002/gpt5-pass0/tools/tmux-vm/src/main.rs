use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    list: bool,
}

fn main() {
    let _ = Args::parse();
}
