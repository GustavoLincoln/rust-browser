use crate::domain::bookmark::Bookmark;

pub struct BookmarkStore {
    db: sled::Db,
}

impl BookmarkStore {
    pub fn open(path: &str) -> Result<Self, String> {
        let db = sled::open(path).map_err(|error| error.to_string())?;
        Ok(Self { db })
    }

    pub fn save(&self, url: &str, title: &str) -> Result<(), String> {
        let key = Self::key(url);
        self.db
            .insert(key, title.as_bytes())
            .map_err(|error| error.to_string())?;
        self.db.flush().map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn get(&self, url: &str) -> Result<Option<Bookmark>, String> {
        let key = Self::key(url);
        let record = self.db.get(key).map_err(|error| error.to_string())?;

        match record {
            Some(value) => {
                let title = String::from_utf8(value.to_vec()).map_err(|error| error.to_string())?;
                Ok(Some(Bookmark {
                    url: url.to_string(),
                    title,
                }))
            }
            None => Ok(None),
        }
    }

    fn key(url: &str) -> String {
        format!("bookmark:{url}")
    }
}
