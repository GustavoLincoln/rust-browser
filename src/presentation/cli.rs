use crate::application::browser_service::{BrowserService, NavigationOutcome};

pub fn run_simulation(service: &mut BrowserService, urls: &[&str]) {
    println!("Simulating browser navigation...");

    for url in urls {
        let label = match service.navigate(url) {
            NavigationOutcome::Allowed => "[ALLOWED]",
            NavigationOutcome::Blocked => "[BLOCKED] - Blocked by security filter",
        };

        println!("{label}: {url}");
    }
}
