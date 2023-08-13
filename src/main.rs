use anyhow::Result;
use futures::stream::{FuturesUnordered, StreamExt};
use headless_chrome::{Browser, LaunchOptionsBuilder};
use tokio::task::{spawn_blocking, JoinHandle};

fn get_title(browser: &Browser, url: &str) -> JoinHandle<Result<String>> {
    let browser = browser.clone();
    let url = url.to_string();
    spawn_blocking(move || -> Result<String> {
        let tab = browser.new_tab()?;
        tab.enable_stealth_mode()?;
        tab.navigate_to(&url)?;
        tab.wait_until_navigated()?;
        let title = tab.get_title()?;
        tab.close_target()?;
        Ok(title)
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(false)
            .build()
            .unwrap(),
    )
    .expect("failed to launch browser");
    let mut urls = vec![
        "https://www.sciencesetavenir.fr/",
        "https://www.lemonde.fr/",
        "https://www.lefigaro.fr/",
        "https://www.liberation.fr/",
        "https://www.leparisien.fr/",
        "https://www.20minutes.fr/",
        "https://www.lci.fr/",
        "https://www.francetvinfo.fr/",
        "https://www.bfmtv.com/",
        "https://www.europe1.fr/",
        "https://www.rtl.fr/",
        "https://www.franceinter.fr/",
        "https://www.francebleu.fr/",
        "https://www.franceinfo.fr/",
    ];
    // let mut futures: FuturesUnordered<JoinHandle<Result<String>>> = FuturesUnordered::new();
    let mut futures = FuturesUnordered::new();
    for _ in 0..4 {
        if let Some(url) = urls.pop() {
            futures.push(get_title(&browser, url));
        }
    }

    // Iterate over the FuturesUnordered set as futures complete
    while let Some(result) = futures.next().await {
        match result {
            Ok(Ok(url)) => println!("Successfully processed: {}", url),
            Ok(Err(err)) => eprintln!("Error processing: {err:?}"),
            Err(err) => println!("Task panicked {err:?}"),
        }
        if let Some(url) = urls.pop() {
            futures.push(get_title(&browser, url));
        }
    }
    Ok(())
}
