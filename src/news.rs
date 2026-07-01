use std::collections::HashSet;

use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use rss::Channel;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewsItem {
    pub id: String,
    pub title: String,
    pub source: String,
    pub author: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FeedSource<'a> {
    pub label: &'a str,
    pub url: &'a str,
}

pub fn fetch_news(feeds: &[FeedSource<'_>]) -> Result<Vec<NewsItem>, String> {
    let client = Client::builder()
        .user_agent("ApeTerm/0.1")
        .build()
        .map_err(|error| error.to_string())?;
    let mut items = Vec::new();

    for (feed_index, feed) in feeds.iter().enumerate() {
        let response = client
            .get(feed.url)
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(|error| format!("{}: {}", feed.label, error))?;
        let bytes = response.bytes().map_err(|error| error.to_string())?;
        let channel =
            Channel::read_from(&bytes[..]).map_err(|error| format!("{}: {}", feed.label, error))?;

        for (item_index, item) in channel.items().iter().enumerate() {
            if let Some(parsed) = parse_item(item, feed.label) {
                items.push((parsed, feed_index, item_index));
            }
        }
    }

    Ok(dedupe_and_sort(items))
}

fn parse_item(item: &rss::Item, source: &str) -> Option<NewsItem> {
    let title = item.title()?.trim();
    let url = item.link()?.trim();
    if title.is_empty() || url.is_empty() {
        return None;
    }

    let id = item.guid().map(|guid| guid.value()).unwrap_or(url).trim();
    if id.is_empty() {
        return None;
    }

    Some(NewsItem {
        id: id.to_string(),
        title: title.to_string(),
        source: source.to_string(),
        author: item
            .author()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string),
        published_at: item.pub_date().and_then(parse_timestamp),
        url: url.to_string(),
        description: item
            .description()
            .map(strip_html)
            .filter(|value| !value.trim().is_empty()),
    })
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc2822(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .or_else(|_| {
            DateTime::parse_from_rfc3339(value).map(|timestamp| timestamp.with_timezone(&Utc))
        })
        .ok()
}

fn dedupe_and_sort(items: Vec<(NewsItem, usize, usize)>) -> Vec<NewsItem> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for (item, feed_index, item_index) in items {
        if seen.insert(item.id.clone()) {
            deduped.push((item, feed_index, item_index));
        }
    }

    deduped.sort_by(|left, right| {
        right
            .0
            .published_at
            .cmp(&left.0.published_at)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
    });

    deduped.into_iter().map(|(item, _, _)| item).collect()
}

fn strip_html(value: &str) -> String {
    let mut text = String::with_capacity(value.len());
    let mut in_tag = false;
    let mut entity = String::new();
    let mut in_entity = false;

    for character in value.chars() {
        match character {
            '<' => {
                in_tag = true;
                if !text.ends_with(' ') {
                    text.push(' ');
                }
            }
            '>' => in_tag = false,
            '&' if !in_tag => {
                in_entity = true;
                entity.clear();
            }
            ';' if in_entity => {
                in_entity = false;
                text.push_str(match entity.as_str() {
                    "amp" => "&",
                    "lt" => "<",
                    "gt" => ">",
                    "quot" => "\"",
                    "#39" | "apos" => "'",
                    "nbsp" => " ",
                    _ => "",
                });
            }
            _ if in_tag => {}
            _ if in_entity => entity.push(character),
            _ => text.push(character),
        }
    }

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_item_preferring_guid_for_id() {
        let channel = Channel::read_from(
            br#"<?xml version="1.0"?>
            <rss version="2.0">
              <channel>
                <title>Feed</title>
                <item>
                  <title>Headline</title>
                  <link>https://example.com/story</link>
                  <guid>story-1</guid>
                  <author>Jane Doe</author>
                  <pubDate>Mon, 01 Jul 2024 12:00:00 GMT</pubDate>
                  <description><![CDATA[<p>Hello <b>world</b></p>]]></description>
                </item>
              </channel>
            </rss>"#
                .as_slice(),
        )
        .unwrap();

        let item = parse_item(&channel.items()[0], "Feed").unwrap();
        assert_eq!(item.id, "story-1");
        assert_eq!(item.title, "Headline");
        assert_eq!(item.author.as_deref(), Some("Jane Doe"));
        assert_eq!(item.description.as_deref(), Some("Hello world"));
        assert!(item.published_at.is_some());
    }

    #[test]
    fn dedupes_by_id_before_link_fallback() {
        let first = NewsItem {
            id: "same".to_string(),
            title: "First".to_string(),
            source: "Feed".to_string(),
            author: None,
            published_at: None,
            url: "https://example.com/a".to_string(),
            description: None,
        };
        let second = NewsItem {
            id: "same".to_string(),
            title: "Second".to_string(),
            source: "Feed".to_string(),
            author: None,
            published_at: None,
            url: "https://example.com/b".to_string(),
            description: None,
        };

        let items = dedupe_and_sort(vec![(first, 0, 0), (second, 0, 1)]);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "First");
    }

    #[test]
    fn sorts_newest_first_and_falls_back_to_feed_order() {
        let older = NewsItem {
            id: "1".to_string(),
            title: "Older".to_string(),
            source: "Feed".to_string(),
            author: None,
            published_at: Some(
                DateTime::parse_from_rfc3339("2024-01-01T10:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            url: "https://example.com/1".to_string(),
            description: None,
        };
        let newer = NewsItem {
            id: "2".to_string(),
            title: "Newer".to_string(),
            source: "Feed".to_string(),
            author: None,
            published_at: Some(
                DateTime::parse_from_rfc3339("2024-01-01T11:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            ),
            url: "https://example.com/2".to_string(),
            description: None,
        };
        let undated = NewsItem {
            id: "3".to_string(),
            title: "Undated".to_string(),
            source: "Feed".to_string(),
            author: None,
            published_at: None,
            url: "https://example.com/3".to_string(),
            description: None,
        };

        let items = dedupe_and_sort(vec![(older, 0, 0), (newer, 0, 1), (undated, 1, 0)]);
        assert_eq!(
            items
                .iter()
                .map(|item| item.title.as_str())
                .collect::<Vec<_>>(),
            vec!["Newer", "Older", "Undated"]
        );
    }
}
