//! Google engine, ported from `searx/engines/google.py`.
//!
//! Request the compact HTML layout on /search. The desktop layout can return
//! only a JavaScript bootstrap page; /wml/search rejects this client with 403.

use std::collections::HashSet;
use std::sync::Arc;

use crate::engine::util::*;
use crate::engine::{
    Engine, EngineDefaults, EngineError, EngineItem, EngineMeta, EngineRequest, EngineResponse,
};
use crate::query::{Query, TimeRange};
use crate::settings::EngineConfig;

const DEFAULTS: EngineDefaults = EngineDefaults {
    categories: &["general", "web"],
    shortcut: "go",
    paging: true,
    time_range_support: true,
    safesearch: true,
    language_support: true,
    weight: 1.0,
    timeout: 4.0,
};

const BASE_URL: &str = "https://www.google.com/search";
// The same compact-layout client hint used by this port's Google Images engine.
const USER_AGENT: &str =
    "Nokia7610/2.0 (5.0509.0) SymbianOS/7.0s Series60/2.1 Profile/MIDP-2.0 Configuration/CLDC-1.0";

/// Google web search (HTML endpoint).
pub struct Google {
    meta: EngineMeta,
}

impl Google {
    pub fn from_config(config: &EngineConfig) -> Result<Arc<Self>, String> {
        Ok(Arc::new(Self {
            meta: EngineMeta::from_config(config, DEFAULTS),
        }))
    }
}

impl Engine for Google {
    fn meta(&self) -> &EngineMeta {
        &self.meta
    }

    fn build_request(&self, query: &Query, pageno: u32) -> Result<EngineRequest, EngineError> {
        let language = locale_language(&query.locale);
        let mut pairs: Vec<(&str, String)> = vec![
            ("q", query.query_text()),
            ("sca_esv", "1".to_string()),
            (
                "hl",
                if language == "all" {
                    "en".to_string()
                } else {
                    language.clone()
                },
            ),
            ("ie", "utf8".to_string()),
            ("oe", "utf8".to_string()),
        ];
        // The UI locale controls hl. Restrict document language only when the
        // caller selected a search language, rather than for an automatic search.
        if let Some(requested) = query.languages.first() {
            let requested = locale_language(requested);
            if requested != "all" {
                pairs.push(("lr", format!("lang_{requested}")));
            }
        }
        let start = pageno.saturating_sub(1).saturating_mul(10);
        if start > 0 {
            pairs.push(("start", start.to_string()));
        }
        if query.safesearch > 0 {
            pairs.push((
                "safe",
                if query.safesearch == 1 {
                    "medium"
                } else {
                    "high"
                }
                .to_string(),
            ));
        }
        if let Some(range) = query.time_range {
            let value = match range {
                TimeRange::Day => "qdr:d",
                TimeRange::Week => "qdr:w",
                TimeRange::Month => "qdr:m",
                TimeRange::Year => "qdr:y",
            };
            pairs.push(("tbs", value.to_string()));
        }
        let url = format!("{BASE_URL}?{}", query_string(&pairs));
        Ok(EngineRequest::get(url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "*/*"))
    }

    fn parse(
        &self,
        response: &EngineResponse,
        _query: &Query,
    ) -> Result<Vec<EngineItem>, EngineError> {
        let body = response.text();
        if url::Url::parse(&response.final_url).is_ok_and(|url| {
            url.host_str() == Some("sorry.google.com") || url.path().starts_with("/sorry")
        }) || body.contains("detected unusual traffic")
            || body.contains("/sorry/index")
        {
            return Err(EngineError::Captcha);
        }
        match response.status {
            401 | 403 => return Err(EngineError::AccessDenied),
            429 => return Err(EngineError::TooManyRequests),
            400..=599 => {
                return Err(EngineError::Message(format!(
                    "Google returned HTTP {}",
                    response.status
                )));
            }
            _ => {}
        }
        let markup = body.trim_start();
        let markup = if markup.starts_with("<?xml") {
            markup
                .split_once("?>")
                .map(|(_, rest)| rest)
                .unwrap_or(markup)
        } else {
            markup
        };
        let html = parse_html(markup);
        let mut seen = HashSet::new();
        let mut results = Vec::new();
        for element in select_elements(&html, "div.zMzFAb, div.g, div.tF2Cxc, div.MjjYud")? {
            let Some(link) = child(element, "a.fuLhoc[href]").or_else(|| child(element, "a[href]"))
            else {
                continue;
            };
            let Some(href) = attr(link, "href") else {
                continue;
            };
            let Some(url) = unwrap_google_url(&href) else {
                continue;
            };
            let title = if element.value().classes().any(|class| class == "zMzFAb") {
                let Some(title) = text_of(link, "span.CVA68e") else {
                    continue;
                };
                title
            } else {
                text_of(element, "h3").unwrap_or_else(|| text(link))
            };
            if title.is_empty() || title.starts_with("http") {
                continue;
            }
            if !seen.insert(url.clone()) {
                continue;
            }
            let content = text_of(element, "div.taTFJ span.FrIlee")
                .or_else(|| text_of(element, "div.VwiC3b"))
                .or_else(|| text_of(element, "span.aCOpRe"))
                .or_else(|| text_of(element, "div[data-sncf]"))
                .or_else(|| text_of(element, "div.IsZvec"))
                .unwrap_or_default();
            results.push(ResultBuilder::new(url, title).content(content).build());
        }
        if results.is_empty() {
            if body.contains("/httpservice/retry/enablejs") {
                return Err(EngineError::Message(
                    "Google returned a JavaScript-required page instead of search results".into(),
                ));
            }
            if !body.contains("did not match any documents")
                && !body.contains("一致する情報は見つかりませんでした")
                && !body.contains("検索結果はありません")
            {
                return Err(EngineError::Parse(
                    "Google returned no recognized search result markup".into(),
                ));
            }
        }
        Ok(results)
    }
}

/// Unwrap the `/url?q=...` redirects Google uses in its result links.
fn unwrap_google_url(href: &str) -> Option<String> {
    let target = if let Some(rest) = href.strip_prefix("/url?") {
        url::form_urlencoded::parse(rest.as_bytes())
            .find(|(key, _)| key == "q" || key == "url")?
            .1
            .into_owned()
    } else {
        absolute_url(href, "https://www.google.com/")?
    };
    let parsed = url::Url::parse(&target).ok()?;
    (matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some())
        .then(|| parsed.to_string())
}
