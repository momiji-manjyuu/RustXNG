//! Regression coverage for Google adapters retained in this local branch.

use rustxng::engine::{EngineError, EngineItem, EngineRegistry, EngineResponse};
use rustxng::engines;
use rustxng::query::{Query, TimeRange};
use rustxng::settings::Settings;

#[test]
fn google_adapters_keep_their_previous_names_and_enabled_defaults() {
    let settings = Settings::defaults();
    for (name, module, enabled) in [
        ("google", "google", true),
        ("google news", "google_news", true),
        ("google_play", "google_play", true),
        ("google_images", "google_images", true),
        ("google_scholar", "google_scholar", false),
        ("google_videos", "google_videos", false),
        ("google_cse", "google_cse", false),
    ] {
        let config = settings.engine(name).expect("retained Google settings");
        assert_eq!(config.engine, module);
        assert_eq!(config.disabled != Some(true), enabled);
        let engine = engines::build(config).expect("retained Google factory");
        assert_eq!(engine.meta().name, name);
    }
}

#[test]
fn partial_user_settings_keep_google_and_can_disable_it() {
    let settings = Settings::from_yaml_str("server:\n  port: 8888\n").unwrap();
    let registry = EngineRegistry::from_settings(&settings).unwrap();
    assert!(registry.get("google").is_some());
    assert!(registry.get("google_news").is_some());
    let query = Query::parse("!go Rust", &registry, &[]);
    assert!(query.specific);
    assert_eq!(query.engine_refs[0].name, "google");

    let settings = Settings::from_yaml_str(
        "engines:\n  - name: google\n    engine: google\n    disabled: true\n",
    )
    .unwrap();
    let registry = EngineRegistry::from_settings(&settings).unwrap();
    assert!(registry.get("google").is_none());
    assert!(registry.get("google_news").is_some());
}

#[test]
fn google_web_encodes_japanese_queries_paging_and_filters() {
    let settings = Settings::defaults();
    let engine = engines::build(settings.engine("google").unwrap()).unwrap();
    let mut query = Query::parse("日本語 Rust & Tokio", &EngineRegistry::default(), &[]);
    query.locale = "ja-JP".into();
    query.apply_language_param("ja").unwrap();
    query.safesearch = 2;
    query.time_range = Some(TimeRange::Week);
    let request = engine.build_request(&query, 2).unwrap();
    let url = url::Url::parse(&request.url).unwrap();
    let pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();
    assert_eq!(url.host_str(), Some("www.google.com"));
    assert_eq!(pairs["q"], "日本語 Rust & Tokio");
    assert_eq!(pairs["start"], "10");
    assert_eq!(pairs["hl"], "ja");
    assert_eq!(pairs["lr"], "lang_ja");
    assert_eq!(pairs["safe"], "high");
    assert_eq!(pairs["tbs"], "qdr:w");
    assert_eq!(pairs["ie"], "utf8");
    assert_eq!(request.headers["accept"], "*/*");
    assert!(
        request.headers["user-agent"]
            .to_str()
            .unwrap()
            .starts_with("Nokia")
    );
    assert!(!pairs.contains_key("gbv"));
}

#[test]
fn google_web_parses_results_and_removes_duplicates() {
    let settings = Settings::defaults();
    let engine = engines::build(settings.engine("google").unwrap()).unwrap();
    let body = r#"<div class="g"><a href="/url?q=https%3A%2F%2Fexample.com%2Frust&amp;sa=U"><h3>Rust 日本語</h3></a><div class="VwiC3b">説明 &amp; snippet</div></div>
        <div class="g"><a href="https://example.com/rust"><h3>Duplicate</h3></a></div>"#;
    let response = EngineResponse::new(200, "https://www.google.com/search", body.into());
    let query = Query::parse("Rust", &EngineRegistry::default(), &[]);
    let items = engine.parse(&response, &query).unwrap();
    assert_eq!(items.len(), 1);
    let EngineItem::Result(result) = &items[0] else {
        panic!("expected a search result");
    };
    assert_eq!(result.url, "https://example.com/rust");
    assert_eq!(result.title, "Rust 日本語");
    assert_eq!(result.content, "説明 & snippet");
}

#[test]
fn google_web_distinguishes_empty_results_from_blocked_responses() {
    let settings = Settings::defaults();
    let engine = engines::build(settings.engine("google").unwrap()).unwrap();
    let query = Query::parse("Rust", &EngineRegistry::default(), &[]);
    for body in ["detected unusual traffic", "/sorry/index"] {
        let response = EngineResponse::new(200, "https://www.google.com/search", body.into());
        assert!(matches!(
            engine.parse(&response, &query),
            Err(EngineError::Captcha)
        ));
    }
    let response = EngineResponse::new(
        200,
        "https://www.google.com/search",
        b"interstitial".to_vec(),
    );
    assert!(matches!(
        engine.parse(&response, &query),
        Err(EngineError::Parse(_))
    ));
    let response = EngineResponse::new(
        200,
        "https://www.google.com/search",
        b"did not match any documents".to_vec(),
    );
    assert!(engine.parse(&response, &query).unwrap().is_empty());
}

#[test]
fn google_web_parses_compact_xml_layout_and_ignores_non_result_cards() {
    let settings = Settings::defaults();
    let engine = engines::build(settings.engine("google").unwrap()).unwrap();
    let body = r#"<?xml version="1.0" encoding="UTF-8"?><html><body>
        <div class="zMzFAb"><a href="https://example.com/navigation">Navigation</a></div>
        <div class="zMzFAb"><a class="fuLhoc" href="/url?opi=1&amp;q=https%3A%2F%2Fexample.com%2Frust%3Fa%3D1%26b%3D2&amp;sa=U"><span class="CVA68e qXLe6d">Rust 日本語</span><span>example.com</span></a>
            <div class="taTFJ"><span class="FrIlee">日本語の説明 &amp; Rust.</span><span class="FrIlee">Sitelinks</span></div></div>
        <div class="zMzFAb"><a class="fuLhoc" href="https://example.com/rust?a=1&amp;b=2"><span class="CVA68e">Duplicate</span></a></div>
        <div class="zMzFAb"><a class="fuLhoc" href="/url?url=javascript%3Aalert%281%29"><span class="CVA68e">Unsafe</span></a></div>
        <div class="zMzFAb"><a class="fuLhoc" href="/url?sa=U"><span class="CVA68e">Missing target</span></a></div>
        <noscript><a href="/httpservice/retry/enablejs">Enable JS</a></noscript>
        </body></html>"#;
    let response = EngineResponse::new(200, "https://www.google.com/search", body.into());
    let query = Query::parse("Rust", &EngineRegistry::default(), &[]);
    let items = engine.parse(&response, &query).unwrap();
    assert_eq!(items.len(), 1);
    let EngineItem::Result(result) = &items[0] else {
        panic!("expected a search result");
    };
    assert_eq!(result.url, "https://example.com/rust?a=1&b=2");
    assert_eq!(result.title, "Rust 日本語");
    assert_eq!(result.content, "日本語の説明 & Rust.");
}

#[test]
fn google_web_reports_javascript_challenges_separately_from_http_errors() {
    let settings = Settings::defaults();
    let engine = engines::build(settings.engine("google").unwrap()).unwrap();
    let query = Query::parse("Rust", &EngineRegistry::default(), &[]);
    let response = EngineResponse::new(
        200, "https://www.google.com/search",
        br#"<noscript><meta http-equiv="refresh" content="0;url=/httpservice/retry/enablejs"></noscript><script>bootstrap()</script>"#.to_vec(),
    );
    assert!(
        matches!(engine.parse(&response, &query), Err(EngineError::Message(reason)) if reason.contains("JavaScript-required"))
    );
    let response = EngineResponse::new(403, "https://www.google.com/search", b"Forbidden".to_vec());
    assert!(matches!(
        engine.parse(&response, &query),
        Err(EngineError::AccessDenied)
    ));
    let response = EngineResponse::new(
        429,
        "https://www.google.com/search",
        b"Too many requests".to_vec(),
    );
    assert!(matches!(
        engine.parse(&response, &query),
        Err(EngineError::TooManyRequests)
    ));
    let response = EngineResponse::new(
        200,
        "https://sorry.google.com/sorry/",
        b"challenge".to_vec(),
    );
    assert!(matches!(
        engine.parse(&response, &query),
        Err(EngineError::Captcha)
    ));
}

#[test]
fn google_web_handles_first_page_all_languages_and_moderate_safesearch() {
    let settings = Settings::defaults();
    let engine = engines::build(settings.engine("google").unwrap()).unwrap();
    let mut query = Query::parse("Rust", &EngineRegistry::default(), &[]);
    query.locale = "all".into();
    for page in [0, 1] {
        let request = engine.build_request(&query, page).unwrap();
        let url = url::Url::parse(&request.url).unwrap();
        let pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();
        assert_eq!(pairs["hl"], "en");
        assert!(!pairs.contains_key("lr"));
        assert!(!pairs.contains_key("start"));
        assert!(!pairs.contains_key("safe"));
    }
    query.locale = "en".into();
    let request = engine.build_request(&query, 1).unwrap();
    assert!(
        !request.url.contains("lr="),
        "UI locale must not restrict result language"
    );
    query.safesearch = 1;
    let request = engine.build_request(&query, 1).unwrap();
    assert!(request.url.contains("safe=medium"));
    let response = EngineResponse::new(
        200,
        "https://www.google.com/search",
        "一致する情報は見つかりませんでした".as_bytes().to_vec(),
    );
    assert!(engine.parse(&response, &query).unwrap().is_empty());
}
