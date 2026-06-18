#[derive(Clone, Debug, Default)]
pub struct Page {
    pub url: String,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub sections: Vec<PageSection>,
}

#[derive(Clone, Debug, Default)]
pub struct PageSection {
    pub heading: String,
    pub body: String,
}
