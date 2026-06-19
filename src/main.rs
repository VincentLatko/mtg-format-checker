use std::collections::HashMap;
use serde::Deserialize;
use reqwest::{
    StatusCode, 
    header::{
        HeaderMap,
        HeaderValue,
        USER_AGENT
    }
};
use axum::{
    extract::Path,
    response::Html,
    routing::get,
    Router
};

#[derive(Deserialize, Debug)]
struct CardWrapper {}

#[derive(Deserialize, Debug)]
struct MoxfieldDeck {
    mainboard: HashMap<String, CardWrapper>,
    sideboard: Option<HashMap<String, CardWrapper>>,
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/{id}", get(format_check));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn format_check(Path(mox_id): Path<String>) -> Html<String> {
    let mut moxfield_headers = HeaderMap::new();
    moxfield_headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"));
    moxfield_headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8"));
    moxfield_headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9"));
    let mox_client = match reqwest::Client::builder().default_headers(moxfield_headers).build() {
        Ok(mc) => mc,
        Err(_) => return to_html("Could not connect to Moxfield!"),
    };

    let mox_link = format!("https://api2.moxfield.com/v2/decks/all/{mox_id}");
    let mox_response = match mox_client.get(mox_link).send().await {
        Ok(res) => res,
        Err(_) => return to_html("Could not fetch Moxfield link!"),
    };

    let json_response: MoxfieldDeck = match mox_response.json().await {
        Ok(jres) => jres,
        Err(_) => return to_html("Could not get deck list!"),
    };

    let mut decklist: Vec<&str> = Vec::new();
    for card in json_response.mainboard.keys(){
        decklist.push(card.as_str());
    }

    if let Some(ref sb) = json_response.sideboard {
        for card in sb.keys() {
            decklist.push(card.as_str());
        }
    }

    decklist.sort();
    decklist.dedup();

    let mut scryfall_headers = HeaderMap::new();
    scryfall_headers.insert(USER_AGENT, HeaderValue::from_static("MtGFormatChecker/1.0"));

    let scryfall_client = match reqwest::Client::builder().default_headers(scryfall_headers).build() {
        Ok(sc) => sc,
        Err(_) => return to_html("Could not connect to Scryfall!"),
    };

    let mut all_legal = true;
    let mut output = String::new();
    for card in decklist {
        let query = format!(r#"(eur<=2 and eur>0.02) game:paper unique:cards border:black !"{card}""#);
        let params = [("q", query.as_str())];
        let scryfall_response = match scryfall_client.get("https://api.scryfall.com/cards/search").query(&params).send().await {
            Ok(sres) => sres,
            Err(_) => return to_html(&format!("Failed to check {card}")),
        };

        if scryfall_response.status() == StatusCode::NOT_FOUND {
            all_legal = false;
            output.push_str(&format!("{}\n", card));
        }
    }

    if all_legal {
        output.push_str("All cards found were legal!");
    }

    to_html(&output)
}

fn to_html(content: &str) -> Html<String> {
    Html(
        format!(
            r#"<!DOCTYPE html>
            <html>
            <head>
                <title>MTG Format Checker</title>
                <style>
                    body {{ background-color: #000; color: #fff; font-family: monospace; padding: 20px; font-size: 16px; white-space: pre-wrap; line-height: 1.5; }}
                </style>
            </head>
            <body>{}</body>
            </html>"#,
            content
        )
    )
}
