use rustic_qurry::crawler::{Crawler, SaveLocation};

#[tokio::main]
async fn main() {
    let mut crawler = Crawler::new(SaveLocation::File);
    let now = std::time::Instant::now();
    crawler.start().await;
    println!("Time elapsed: {:?}", now.elapsed());
    println!("Done! from main.rs");

    // let seed = "https://www.rust-lang.org/";
    // let html = r###"
    //             <html>
    //                 <head>
    //                     <title>Test</title>
    //                 </head>
    //                 <body>
    //                     <h1>Test</h1>
    //                     <p>Test</p>
    //
    //                     <a href="https://google.com/something">Google</a>
    //                     <a href="https://www.rust-lang.org/first#third">Rust</a>
    //                     <a href="https://www.rust-lang.org/first#second">Rust</a>
    //                     <a href="https://www.rust-lang.org/first#first">Rust</a>
    //                     <a href="something">Rust</a>
    //                     <a href="/somethingelse">something else</a>
    //                     <a href="#fragment">something else</a>
    //
    //                 </body>
    //             </html>
    //             "###;
    // let document = scraper::Html::parse_document(&html);
    // let anchor_selector = Selector::parse("a[href]").unwrap();
    // let url = match Url::parse(&seed) {
    //     Ok(url) => url,
    //     Err(err) => {
    //         println!("Error parsing url: {}", err.to_string());
    //         panic!("Error parsing url");
    //     }
    // };
    //
    // let urls = document
    //     .select(&anchor_selector)
    //     .filter_map(|element| element.value().attr("href"))
    //     .filter(|href| !href.starts_with("#"))
    //     .filter_map(|href| url.join(href).ok())
    //     .map(|url| Crawler::sanitize_url(&url))
    //     .filter(|url| {
    //         if url.starts_with(seed) {
    //             return true;
    //         }
    //         return false;
    //     })
    //     .collect::<HashSet<_>>();
    //
    // for url in urls {
    //     println!("{}", url);
    // }

    // let html = reqwest::get("https://www.rust-lang.org/")
    //     .await
    //     .unwrap()
    //     .text()
    //     .await
    //     .unwrap();
    // println!("{}", html);

    // let mut threads = Vec::new();

    // for i in 0..8 {
    //     let i = Arc::new(i);
    //     let t = tokio::spawn(async move {
    //         let f = async_fib(43).await;
    //         println!("f: {}, thread: {}", f, i);
    //     });
    //     threads.push(t);
    // }

    // for t in threads {
    //     t.await.unwrap();
    // }
}
