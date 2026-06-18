mod application;
mod domain;
mod infrastructure;
mod presentation;

use application::runtime::{bootstrap_browser_service, load_app_settings, RuntimeConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RuntimeConfig::from_env();
    let settings = load_app_settings(&config).map_err(std::io::Error::other)?;
    let browser_service =
        bootstrap_browser_service(&config, &settings).map_err(std::io::Error::other)?;
    presentation::webview_shell::runner::run(browser_service, config, settings)
        .map_err(std::io::Error::other)?;
    Ok(())
}
