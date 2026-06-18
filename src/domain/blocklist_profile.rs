use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum BlocklistProfile {
    Light,
    #[default]
    Normal,
    Pro,
}

impl BlocklistProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Normal => "Normal",
            Self::Pro => "Pro",
        }
    }

    pub fn from_command(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "light" => Some(Self::Light),
            "normal" => Some(Self::Normal),
            "pro" => Some(Self::Pro),
            _ => None,
        }
    }
}
