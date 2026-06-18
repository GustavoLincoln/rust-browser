pub trait UrlPolicy {
    fn allows(&self, url: &str) -> bool;
}
