use env_file_reader::read_file;
use reqwest;

use rspotify;
use rspotify::model::{Country, Market, PlayableId, SearchResult, SearchType, TrackId, UserId};
use rspotify::prelude::{BaseClient, Id, OAuthClient};
use rspotify::{scopes, AuthCodeSpotify, Config, Credentials, OAuth};
use scraper::{Html, Selector};
use std::collections::HashMap;

#[tokio::main]
async fn main() {
    let env: HashMap<String, String> =
        read_file("src/.env").expect("error loading environment variables");

    let spotify = spot_login(&env).await;
    let date = get_date();
    let top_100 = get_top_100(&env, &date).await;
    let all_tracks = get_songs_list(&spotify, &top_100, &date).await;

    create_playlist(&spotify, &all_tracks, &env, &date).await;
}

fn get_date() -> (i32, String, String) {
    let mut year = String::new();
    let mut month = String::new();
    let mut day = String::new();
    println!(" Top 100 Song playlist creator \nYear: ");
    std::io::stdin()
        .read_line(&mut year)
        .expect("error reading input for year");

    println!("Month: ");
    std::io::stdin()
        .read_line(&mut month)
        .expect("error reading input for month");

    println!("Day: ");
    std::io::stdin()
        .read_line(&mut day)
        .expect("error reading input for day");

    let year = year.trim().parse::<i32>().unwrap();
    let month = month.trim().to_string();
    let day = day.trim().to_string();

    (year, month, day)
}

async fn get_top_100<'a>(
    x: &HashMap<String, String>,
    date: &(i32, String, String),
) -> Vec<(String, String)> {
    println!("Top 100 song list for entered year being gathered...");
    let url = format!(
        "https://www.billboard.com/charts/hot-100/{}-{}-{}?rank=1",
        date.0, date.1, date.2
    );
    // dbg!(&url);
    let body = reqwest::get(&url)
        .await
        .expect("error with GET request to billboard url")
        .text()
        .await
        .expect("error converting response to text");
    let document = Html::parse_document(body.as_str());
    let song = Selector::parse(&x["SONG_SELECTOR"]).expect("could not parse song selector");
    let artist = Selector::parse(&x["ARTIST_SELECTOR"]).expect("could not parse artist selector");
    let songs = document.select(&song);
    let artists = document.select(&artist);
    let mut list = Vec::new();
    let _: Vec<_> = songs
        .zip(artists)
        .map(|(name, artist)| {
            list.push((
                name.inner_html().trim().to_string(),
                artist.inner_html().trim().to_string(),
            ));
        })
        .collect();
    dbg!(&list[..10]);
    list
}

async fn spot_login(x: &HashMap<String, String>) -> AuthCodeSpotify {
    println!("Authenticating...");
    let creds = Credentials::new(&x["SPOTIPY_CLIENT_ID"], &x["SPOTIPY_CLIENT_SECRET"]);
    // let creds = Credentials::from_env().unwrap();

    let oauth = OAuth {
        redirect_uri: x["SPOTIPY_REDIRECT_URI"].to_string(),
        scopes: scopes!("playlist-modify-private"),
        ..Default::default()
    };

    // Enabling automatic token refreshing in the config
    let config = Config {
        token_refreshing: true,
        token_cached: true,
        ..Default::default()
    };

    // let oauth = OAuth::from_env(scopes!("user-read-currently-playing")).unwrap();
    let mut spotify = AuthCodeSpotify::with_config(creds, oauth, config);

    // Obtaining the access token
    let url = spotify
        .get_authorize_url(false)
        .expect("get_authorize_url error");

    // This function requires the `cli` feature enabled.
    spotify
        .prompt_for_token(&url)
        .await
        .expect("prompt_for_token error");

    spotify
}

async fn get_songs_list(
    spotify: &AuthCodeSpotify,
    top_100: &Vec<(String, String)>,
    date: &(i32, String, String),
) -> Vec<TrackId> {
    println!("Track ID's for all songs being located...");
    let mut all_tracks = Vec::new();
    // you can filter the top_100 slice for debugging. ex.  [..5] for only the first 4 songs.

    for song in &top_100[..] {
        // space is needed inbetween song name and filters (artist/year)
        let start_date: i32 = date.0 - 2;
        let end_date: i32 = date.0 + 2;

        let search_term = &format!(
            "{} artist:{} year:{}-{}",
            &song.0, &song.1, start_date, end_date
        );
        // dbg!(search_term);
        let market = Market::Country(Country::UnitedStates);
        let found_song = spotify
            .search(
                search_term,
                &SearchType::Track,
                Some(&market),
                None,
                Some(3),
                None,
            )
            .await;
        // .expect(format!("error finding song {}", search_term).as_str());

        match found_song {
            Ok(SearchResult::Tracks(t)) => {
                // dbg!(&t);
                match t.items.get(0) {
                    Some(t) => {
                        let current_track = t.id.clone().expect("id not found");
                        all_tracks.push(current_track);
                    }
                    None => {
                        println!("Song not found, skipping")
                    }
                }
            }
            _ => {}
        }
    }
    // dbg!(&all_tracks[..10]);
    all_tracks
}

async fn create_playlist(
    spotify: &AuthCodeSpotify,
    track_id_list: &Vec<TrackId>,
    env_: &HashMap<String, String>,
    date: &(i32, String, String),
) {
    println!("Playlist being created...");
    let id = UserId::from_id(&env_["USER_ID"]).expect("error creating UserId");
    let new_playlist = spotify
        .user_playlist_create(
            &id,
            format!("Top 100 from {} {} {}", date.0, date.1, date.2).as_str(),
            Some(false),
            None,
            Some("test list"),
        )
        .await
        .expect("error creating playlist");
    let playable_id_list = track_id_list.iter().map(|track| track as &dyn PlayableId);

    spotify
        .playlist_add_items(&new_playlist.id, playable_id_list, None)
        .await
        .expect("error adding items to playlist");
    println!("Playlist complete!");
}
