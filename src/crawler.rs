use crate::utils::read_file;
use mongodb::Client;
use reqwest;
use scraper::Selector;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Borrow,
    collections::{HashSet, VecDeque},
    fs,
    sync::{Arc, Mutex},
};
use url::Url;

const MAX_THREADS: usize = 8;

lazy_static::lazy_static! {
    pub static ref TAGS_TO_CHECK: [Selector; 9] = [
        Selector::parse("h1").unwrap(),
        Selector::parse("h2").unwrap(),
        Selector::parse("h3").unwrap(),
        Selector::parse("h4").unwrap(),
        Selector::parse("h5").unwrap(),
        Selector::parse("h6").unwrap(),
        Selector::parse("p").unwrap(),
        Selector::parse("li").unwrap(),
        Selector::parse("q").unwrap(),
    ];
}

#[derive(Serialize, Deserialize, Clone)]
struct Page {
    url: String,
    title: String,
    description: String,
    html: String,
}

#[derive(Copy, Clone)]
pub enum SaveLocation {
    File,
    Database,
}

pub struct Crawler {
    queue: Arc<Mutex<VecDeque<String>>>,
    seeds: Arc<Mutex<Vec<Page>>>,
    visited: Arc<Mutex<HashSet<String>>>,
    save_location: SaveLocation,
}

impl Crawler {
    pub fn new(save_location: SaveLocation) -> Crawler {
        match save_location {
            SaveLocation::File => {
                let _ = fs::create_dir("crawler/");
            }
            SaveLocation::Database => {}
        }
        Crawler {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            seeds: Arc::new(Mutex::new(Vec::new())),
            visited: Arc::new(Mutex::new(HashSet::new())),
            save_location,
        }
    }

    pub fn sanitize_url(url: &Url) -> String {
        let mut base_url = url.clone();
        base_url.set_fragment(None);
        base_url.set_query(None);

        base_url.to_string()
    }

    fn read_seeds(&mut self) {
        let mut seeds: VecDeque<String> = VecDeque::new();
        read_file("seeds.txt", &mut seeds);

        for seed in seeds {
            self.queue.lock().unwrap().push_back(seed.clone());
            self.seeds.lock().unwrap().push(Page {
                url: seed,
                title: "".to_owned(),
                description: "".to_owned(),
                html: "".to_owned(),
            });
        }
    }

    fn get_page(html: &str, url: &str) -> Page {
        let document = scraper::Html::parse_document(html);
        let title_selector = Selector::parse("title").unwrap();
        let description_selector = Selector::parse("meta[name=description]").unwrap();

        let title = match document.select(&title_selector).next() {
            Some(title) => title.text().collect::<Vec<_>>().join(" "),
            None => "".to_owned(),
        };

        let description = match document.select(&description_selector).next() {
            Some(description) => description.value().attr("content").unwrap().to_owned(),
            None => "".to_owned(),
        };
        let html = document.html();

        Page {
            url: url.to_string(),
            title,
            description,
            html,
        }
    }

    fn get_urls(html: &str, seed: &str, visited: &Arc<Mutex<HashSet<String>>>) -> HashSet<String> {
        let document = scraper::Html::parse_document(html);
        let anchor_selector = Selector::parse("a[href]").unwrap();
        let seed_url = Url::parse(seed).unwrap();

        document
            .borrow()
            .select(&anchor_selector)
            .filter_map(|element| element.value().attr("href"))
            .filter(|href| !href.starts_with("#"))
            .filter_map(|href| seed_url.join(href).ok())
            .map(|url| Crawler::sanitize_url(&url))
            .filter(|url| {
                if url.starts_with(seed) {
                    return true;
                }
                return false;
            })
            .filter(|url| !visited.lock().unwrap().contains(url))
            .collect::<HashSet<String>>()
    }

    async fn get_html(url: &str) -> Result<String, String> {
        let client = match reqwest::ClientBuilder::new()
            .user_agent("RusticQuery")
            .build()
        {
            Ok(client) => client,
            Err(err) => {
                panic!("Error building client: {}", err.to_string());
            }
        };
        let response = match client.get(url).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    return Err(response.status().to_string());
                }
                response
            }
            Err(err) => {
                return Err(err.to_string());
            }
        };
        let html = match response.text().await {
            Ok(html) => html,
            Err(err) => {
                return Err(err.to_string());
            }
        };
        return Ok(html);
    }
    async fn save_page_to_database(client: &Client, page: &Page) -> Result<(), String> {
        let database = client.database("rusticquery");
        let collection = database.collection::<Page>("pages");
        match collection.insert_one(page, None).await {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }
    fn save_page_to_file(page: &Page, seed: &Page) -> Result<(), String> {
        let seed_host = match Url::parse(&seed.url) {
            Ok(seed) => seed.host_str().unwrap().replace(".", "_"),
            Err(err) => return Err(err.to_string()),
        };
        let page_path = match Url::parse(&page.url) {
            Ok(page) => page.path().replace("/", "_"),
            Err(err) => return Err(err.to_string()),
        };

        let _ = fs::create_dir(format!("crawler/{}", seed_host));

        let file = match fs::File::create(format!("crawler/{}/{}.json", seed_host, page_path)) {
            Ok(file) => file,
            Err(err) => return Err(err.to_string()),
        };
        match serde_json::to_writer_pretty(file, &page) {
            Ok(_) => return Ok(()),
            Err(err) => return Err(err.to_string()),
        }
    }

    async fn save_page(
        save_location: &Arc<Mutex<SaveLocation>>,
        client: &Client,
        page: &Page,
        seed: &Page,
    ) -> Result<(), String> {
        let save_location = *save_location.lock().unwrap();
        match save_location {
            SaveLocation::File => Crawler::save_page_to_file(page, seed),
            SaveLocation::Database => Crawler::save_page_to_database(client, page).await,
        }
    }

    pub async fn start(&mut self) {
        self.read_seeds();

        let client = match Client::with_uri_str("mongodb://localhost:27017").await {
            Ok(client) => client,
            Err(err) => {
                println!("Error connecting to mongodb: {}", err.to_string());
                return;
            }
        };

        let save_location = Arc::new(Mutex::new(self.save_location));
        let mut threads = Vec::with_capacity(MAX_THREADS);

        for i in 0..MAX_THREADS {
            let queue = self.queue.clone();
            let seeds = self.seeds.clone();
            let visited = self.visited.clone();
            let client = client.clone();
            let save_location = save_location.clone();

            let thread = tokio::spawn(async move {
                loop {
                    if queue.lock().unwrap().is_empty() && !visited.lock().unwrap().is_empty() {
                        println!("Thread {} finished", i);
                        break;
                    } else if queue.lock().unwrap().is_empty() {
                        continue;
                    }

                    let link = queue.lock().unwrap().pop_front().unwrap();
                    if visited.lock().unwrap().contains(&link) {
                        continue;
                    }

                    let html = match Crawler::get_html(&link).await {
                        Ok(html) => html,
                        Err(_) => continue,
                    };
                    let page = Crawler::get_page(&html, &link);

                    let seed = {
                        let mut res: Option<Page> = None;
                        for seed in seeds.lock().unwrap().iter() {
                            if link.starts_with(&seed.url) {
                                res = Some(seed.clone());
                                break;
                            }
                        }
                        match res {
                            Some(seed) => seed,
                            None => continue,
                        }
                    };

                    if !visited.lock().unwrap().contains(&link) {
                        match Crawler::save_page(&save_location, &client, &page, &seed).await {
                            Ok(_) => {}
                            Err(err) => {
                                println!("Error saving page: {}", err);
                                continue;
                            }
                        };
                    }
                    visited.lock().unwrap().insert(link.clone());

                    let urls = Crawler::get_urls(&html, &seed.url, &visited);
                    queue.lock().unwrap().extend(urls.to_owned());

                    println!("{i}:{}", link);

                    if queue.lock().unwrap().is_empty() {
                        println!("Thread {} finished", i);
                        break;
                    }
                }
            });

            threads.push(thread);
        }
        for thread in threads {
            thread.await.unwrap();
        }
    }
}
