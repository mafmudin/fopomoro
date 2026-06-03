use std::env;

#[derive(Clone, Debug)]
pub struct SupabaseConfig {
    pub base_url: String,
    pub key: String,
}

impl SupabaseConfig {
    pub fn from_env() -> Option<Self> {
        let url = env::var("SUPABASE_URL").ok().filter(|s| !s.is_empty())?;
        let key = env::var("SUPABASE_ANON_KEY").ok().filter(|s| !s.is_empty())?;
        Some(Self {
            base_url: format!("{}/rest/v1", url.trim_end_matches('/')),
            key,
        })
    }
}
