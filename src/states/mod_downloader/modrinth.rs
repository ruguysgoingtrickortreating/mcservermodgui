use itertools::Itertools;

pub fn search_recieved(data: Vec<u8>) {
    
}

pub async fn search_mods(
    offset: u64,
    query: Option<String>,
    facets: Vec<String>,
    sequence_number: usize,
) -> (usize, Result<Vec<u8>, String>) {
    (sequence_number, (|| async { // closure hackiness to get the ? operator to work when we need to bundle the sequence_number
        let mut args: Vec<(&'static str, String)> =
            vec![("limit", "20".to_string()), ("offset", offset.to_string())];

        if let Some(q) = query {
            args.push(("query", q))
        };

        args.push((
            "facets",
            format!(
                "[{}]",
                facets.iter()
                    .map(|f| format!("[{f}]"))
                    .join(",")
            ),
        ));

        let get = crate::REQ_CLIENT
            .get("https://api.modrinth.com/v2/search")
            .query(&args);

        let response = get.send().await.map_err(|err| err.to_string())?;

        println!("{}", response.url());

        if !response.status().is_success() {
            Err(format!(
                "Error {} from search with args {:?}",
                response.status(),
                args
            ))
        } else {
            let bytes = response
                .bytes()
                .await
                .map_err(|err| err.to_string())?
                .to_vec();
            Ok(bytes)
        }
    })().await)
}