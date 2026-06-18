mod application;
mod domain;
mod infrastructure;
mod presentation;

use application::browser_service::BrowserService;

fn main() {
    let blocklist_path = "blocklist.txt";
    let storage_path = "target/browser-data";

    match BrowserService::bootstrap(blocklist_path, storage_path) {
        Ok(mut service) => {
            let urls = [
                "https://google.com",
                "http://malware-site.ru",
                "invalid_url",
            ];

            presentation::cli::run_simulation(&mut service, &urls);
        }
        Err(error) => eprintln!("Failed to initialize browser: {error}"),
    }
}
