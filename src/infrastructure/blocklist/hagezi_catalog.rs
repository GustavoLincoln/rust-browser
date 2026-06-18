use crate::domain::blocklist_profile::BlocklistProfile;

pub fn profile_url(profile: BlocklistProfile) -> &'static str {
    match profile {
        BlocklistProfile::Light => {
            "https://raw.githubusercontent.com/hagezi/dns-blocklists/main/domains/light.txt"
        }
        BlocklistProfile::Normal => {
            "https://raw.githubusercontent.com/hagezi/dns-blocklists/main/domains/multi.txt"
        }
        BlocklistProfile::Pro => {
            "https://raw.githubusercontent.com/hagezi/dns-blocklists/main/domains/pro.txt"
        }
    }
}
