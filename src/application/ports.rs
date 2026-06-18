use crate::domain::page::Page;

pub trait PageLoader: Send + Sync {
    fn load(&self, url: &str) -> Result<Page, String>;
}
