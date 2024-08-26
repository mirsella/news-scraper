use super::{GetNewsOpts, News};
use anyhow::bail;
use anyhow::{Context, Result};
use headless_chrome::Tab;
use log::{debug, error, trace, warn};
use std::sync::Arc;

fn get_articles_links(tab: &Arc<Tab>) -> Result<Vec<String>> {
    tab.wait_for_elements(".article-wrapper > a")
        .context("wait_for_elements .article-wrapper > a")?;
    let result = tab
        .evaluate(
            "Array.from(document.querySelectorAll('.article-wrapper > a')).map(e => e.href)",
            false,
        )
        .unwrap();
    let props = result.preview.unwrap().properties;
    let mut links = props
        .iter()
        .map(|p| p.value.as_ref().unwrap().to_string())
        .collect::<Vec<_>>();
    links.retain(|link| !link.contains("videos-du-journal"));
    Ok(links)
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let tab = opts.browser.new_context()?.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_secs(120));
    let user_agent = opts.browser.get_version().unwrap().user_agent;
    let user_agent = user_agent.replace("HeadlessChrome", "Chrome");
    tab.set_user_agent(&user_agent, None, None)?;
    tab.navigate_to("https://www.sudouest.fr/")
        .context("navigate_to")?
        .wait_until_navigated()
        .context("wait_until_navigated")?;

    if let Ok(cookie) = tab.find_element("#didomi-notice-agree-button") {
        cookie.click().context("clicking on cookie")?;
        std::thread::sleep(std::time::Duration::from_secs(1));
        trace!("clicked cookie");
    }

    let links = get_articles_links(&tab).context("sudouest")?;
    debug!("found {} links", links.len());
    if links.is_empty() {
        bail!("no links found");
    }
    for url in links {
        if opts.is_seen(&url) {
            continue;
        }

        let cookiewall = "En acceptant les cookies, vous pourrez accéder aux contenus";
        let payload = match super::fetch_article(&url) {
            Ok(res) if res.content.contains(cookiewall) => {
                warn!("cookiewall on {url}");
                continue;
            }
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: opts.provider.clone(),
                date: res
                    .published
                    .parse()
                    .unwrap_or_else(|_| chrono::Local::now()),
                body: res.content,
                link: url.clone(),
                ..Default::default()
            }),
            Err(err) => {
                debug!("fetch_article: {err}");
                continue;
            }
        };
        if let Err(e) = opts.tx.blocking_send(payload) {
            error!("blocking_send: {e}");
            break;
        }
    }
    Ok(())
}
