use anyhow::Ok;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BaseResult {
    pub success: bool,
    pub message: String,
    pub code: String,
    #[serde(rename(serialize = "RequestId", deserialize = "RequestId"))]
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenData {
    token: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenResult {
    #[serde(flatten)]
    base: BaseResult,

    // 返回错误可能没有data数据
    data: Option<TokenData>,
}

// https://doc.sm.ms/#api-_
pub async fn token(username: &str, password: &str) -> anyhow::Result<String> {
    let params = [("username", username), ("password", password)];

    let client = reqwest::Client::new();
    let res = client
        .post("https://sm.ms/api/v2/token")
        .form(&params)
        .send()
        .await?;

    let d = res.json::<TokenResult>().await?;

    // dbg!("{:?}", &d);

    if !d.base.success {
        anyhow::bail!(d.base.message);
    }

    Ok(d.data.unwrap().token)
}

#[derive(Debug, Deserialize)]
pub struct ProfileData {
    pub username: String,
    pub email: String,
    pub role: String,
    pub group_expire: String,
    pub email_verified: u8,
    pub disk_usage: String,
    pub disk_limit: String,
    pub disk_usage_raw: usize,
    pub disk_limit_raw: usize,
}

#[derive(Debug, Deserialize)]
pub struct ProfileResult {
    #[serde(flatten)]
    base: BaseResult,

    // 返回错误可能没有data数据
    data: Option<ProfileData>,
}

pub async fn profile(token: &str) -> anyhow::Result<ProfileData> {
    let client = reqwest::Client::new();
    let res = client
        .post("https://sm.ms/api/v2/profile")
        .header("Authorization", token)
        .send()
        .await?;

    let d = res.json::<ProfileResult>().await?;

    // dbg!("{:?}", &d);

    if !d.base.success {
        anyhow::bail!(d.base.message);
    }

    Ok(d.data.unwrap())
}

pub async fn delete_image(token: &str, hash: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("https://sm.ms/api/v2/delete/{}", hash))
        .header("Authorization", token)
        .send()
        .await?;

    let d = res.json::<BaseResult>().await?;

    // dbg!("{:?}", &d);

    if !d.success {
        anyhow::bail!(d.message);
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct UploadHistoryData {
    pub width: i32,
    pub height: i32,
    pub filename: String,
    pub storename: String,
    pub size: usize,
    pub path: String,
    pub hash: String,
    pub created_at: String,
    pub url: String,
    pub delete: String,
    pub page: String,
}

#[derive(Debug, Deserialize)]
pub struct UploadHistoryResult {
    #[serde(flatten)]
    base: BaseResult,

    // 返回错误可能没有data数据
    data: Option<Vec<UploadHistoryData>>,
}

pub async fn upload_history(token: &str) -> anyhow::Result<Vec<UploadHistoryData>> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("https://sm.ms/api/v2/upload_history?page={}", 0))
        .header("Authorization", token)
        .send()
        .await?;

    let d = res.json::<UploadHistoryResult>().await?;

    if !d.base.success {
        anyhow::bail!(d.base.message);
    }

    Ok(d.data.unwrap())
}

pub async fn upload(token: &str, upload_file_path: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    let upload_file_path_p = std::path::Path::new(upload_file_path);
    let filename = upload_file_path_p
        .file_name()
        .unwrap()
        .to_os_string()
        .into_string()
        .ok()
        .unwrap();

    let form = reqwest::multipart::Form::new().part(
        "smfile",
        reqwest::multipart::Part::bytes(tokio::fs::read(upload_file_path).await.unwrap())
            .file_name(filename),
    );

    let res = client
        .post("https://sm.ms/api/v2/upload")
        .header("Authorization", token)
        .multipart(form)
        .send()
        .await?;

    let d = res.json::<BaseResult>().await?;

    if !d.success {
        anyhow::bail!(d.message);
    }

    Ok(())
}
