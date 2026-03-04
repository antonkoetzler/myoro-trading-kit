fn main() -> anyhow::Result<()> {
    let config = myoro_polymarket_terminal::config::load()?;
    myoro_polymarket_terminal::tui::run(config)
}
