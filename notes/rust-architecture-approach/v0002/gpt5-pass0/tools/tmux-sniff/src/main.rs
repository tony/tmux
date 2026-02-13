use clap::Parser;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    socket: Option<String>,
}

fn main() {
    let _ = Args::parse();
}
