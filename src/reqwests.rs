use std::sync::LazyLock;

use reqwest::Client;


#[derive(Debug,Clone)]
pub struct ImageData {
    pub url: String,
    pub bytes: Vec<u8>,
    pub is_svg: bool,
}

pub async fn download_image(url: String) -> Result<ImageData, String> {
    let response = crate::REQ_CLIENT.get(&url).send().await.map_err(|err| err.to_string())?;

    if !response.status().is_success() {
        Err(format!("Error {} from url: {url}",response.status()))
    } else {
        let bytes = response.bytes().await.map_err(|err| err.to_string())?.to_vec();
        Ok(ImageData { is_svg: bytes.starts_with(b"<svg "), url, bytes, })
    }
}

pub async fn search_mods(offset:u64, query:Option<String>, facets:Option<Vec<String>>) -> Result<Vec<u8>, String> {
    let mut args: Vec<(&'static str, String)> = vec![("limit","20".to_string()),("offset",offset.to_string())];

    if let Some(q) = query {args.push(("query",q))};
    if let Some(f) = facets {args.push(("facets",
        format!("[{}]",f.iter().map(|f|format!("[\"{f}\"]")).collect::<Vec<String>>().join(","))
    ))};

    let get = crate::REQ_CLIENT.get(format!("https://api.modrinth.com/v2/search"))
        .query(&args);
    
    let response = get.send().await.map_err(|err| err.to_string())?;

    if !response.status().is_success() {
        Err(format!("Error {} from search with args {:?}",response.status(),args))
    } else {
        let bytes = response.bytes().await.map_err(|err| err.to_string())?.to_vec();
        Ok(bytes)
    }
}

pub async fn fetch_mod(id:String) -> Result<Vec<u8>, String> {
    let response = crate::REQ_CLIENT.get(format!("https://api.modrinth.com/v2/project/{id}")).send().await.map_err(|err| err.to_string())?;

    if !response.status().is_success() {
        Err(format!("Error {} from mod fetch with id {:?}",response.status(),id))
    } else {
        let bytes = response.bytes().await.map_err(|err| err.to_string())?.to_vec();
        Ok(bytes)
    }
}