use super::{dedupe_links, fetch_article_with_browser_fallback, GetNewsOpts, News};
use anyhow::{Context, Result};
use log::{error, info};
use std::time::Duration;

const KEYWORDS: [&str; 4] = ["bonne nouvelle", "joie", "optimisme", "entraide"];

fn unescape_xml(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                let hex = value.get(index + 1..index + 3)?;
                decoded.push(u8::from_str_radix(hex, 16).ok()?);
                index += 3;
            }
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

fn bing_target(link: &str) -> Option<String> {
    let target = link.split("url=").nth(1)?.split('&').next()?;
    let target = percent_decode(target)?;
    target.starts_with("http").then_some(target)
}

fn parse_rss(xml: &str) -> Vec<String> {
    dedupe_links(xml.split("<item>").skip(1).filter_map(|item| {
        let start = item.find("<link>")? + "<link>".len();
        let end = item[start..].find("</link>")? + start;
        bing_target(&unescape_xml(&item[start..end]))
    }))
}

pub fn get_news(opts: GetNewsOpts) -> Result<()> {
    let mut links = Vec::new();
    for keyword in KEYWORDS {
        let endpoint = format!(
            "https://www.bing.com/news/search?q={}&format=rss&setlang=fr&cc=FR",
            keyword.replace(' ', "+")
        );
        let mut response = ureq::get(&endpoint)
            .config()
            .timeout_global(Some(Duration::from_secs(15)))
            .build()
            .call()
            .with_context(|| format!("fetching Bing News for {keyword}"))?;
        let xml = response
            .body_mut()
            .read_to_string()
            .context("reading Bing News RSS")?;
        let found = parse_rss(&xml);
        info!("found {} articles for {keyword}", found.len());
        links.extend(found);
    }
    let links = dedupe_links(links);
    info!("found {} articles", links.len());
    if links.is_empty() {
        anyhow::bail!("no links found");
    }

    for url in links {
        if opts.is_seen(&url) {
            continue;
        }
        let res = fetch_article_with_browser_fallback(&opts.browser, &url);
        let payload = match res {
            Ok(res) => Ok(News {
                title: res.title,
                caption: res.description,
                provider: opts.provider.clone(),
                date: res.published,
                body: res.content,
                link: url,
                ..Default::default()
            }),
            Err(err) => {
                error!("fetch_article on {url}: {err:?}");
                continue;
            }
        };
        opts.tx.blocking_send(payload).unwrap();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_the_target_url_from_a_bing_redirect() {
        let link = "https://www.bing.com/news/apiclick.aspx?ref=FexRss&aid=&tid=abc&url=https%3a%2f%2fexample.com%2fstory%3fa%3d1%26b%3d2&c=123";
        assert_eq!(
            bing_target(link).as_deref(),
            Some("https://example.com/story?a=1&b=2")
        );
    }

    #[test]
    fn parses_and_deduplicates_bing_rss_items() {
        let item = "<item><title>Story</title><link>https://www.bing.com/news/apiclick.aspx?url=https%3a%2f%2fexample.com%2fa&amp;c=1</link></item>";
        let xml = format!("<rss><channel>{item}{item}</channel></rss>");
        assert_eq!(parse_rss(&xml), vec!["https://example.com/a"]);
    }
}
