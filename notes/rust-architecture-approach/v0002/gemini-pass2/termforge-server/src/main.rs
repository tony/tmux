use mux_api::TermForge;
use mux_config::Config;

fn main() -> anyhow::Result<()> {
    println!("TermForge Server v0.0.2");
    let config = Config::default();
    let app = TermForge::new(config);
    app.start();
    Ok(())
}
