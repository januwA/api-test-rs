use serde::{Deserialize, Serialize};

pub const K_CACHE_PATH: &str = "./sm_ms_cache.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct SmMsCacheData {
    pub token: Option<String>,
}

impl SmMsCacheData {
    /// 获取或则创建缓存文件
    pub fn get_or_create() -> Option<SmMsCacheData> {
        let cache_path = std::path::Path::new(K_CACHE_PATH);
        if cache_path.exists() {
            Self::from().ok()
        } else {
            std::fs::File::create(cache_path).unwrap();
            None
        }
    }

    pub fn save(data: Self) -> anyhow::Result<()> {
        let cache_path = std::path::Path::new(K_CACHE_PATH);
        Ok(std::fs::write(cache_path, serde_json::to_vec(&data)?)?)
    }

    pub fn from() -> anyhow::Result<Self> {
        let cache_path = std::path::Path::new(K_CACHE_PATH);
        Ok(serde_json::from_slice::<SmMsCacheData>(
            std::fs::read(cache_path)?.as_ref(),
        )?)
    }
}
