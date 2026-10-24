use serde_json::Value;

pub fn emojis(query: &str) -> Vec<&'static Value> {
    static CATALOG: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
    let catalog = CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/emoji/catalog.json"))
            .expect("bundled web emoji catalog")
    });
    let mut query = query.to_lowercase();
    if let Some(at) = query.char_indices().find_map(|(at, c)| {
        (c == '-' && at > 0 && query.as_bytes()[at - 1].is_ascii_alphanumeric()).then_some(at)
    }) {
        query.replace_range(at..at + 1, " ");
    }
    let mut tokens: Vec<_> = vec![];
    for token in query.split(|c: char| c.is_whitespace() || matches!(c, '|' | ',')) {
        if !token.is_empty() && !tokens.contains(&token) {
            tokens.push(token);
        }
    }
    if tokens.is_empty() {
        return catalog.iter().collect();
    }
    let mut matches: Vec<_> = catalog
        .iter()
        .filter(|e| {
            tokens
                .iter()
                .all(|t| e["search"].as_str().unwrap().contains(&format!(",{t}")))
        })
        .collect();
    let last = tokens.last().unwrap();
    let rank = |e: &Value| {
        let search = e["search"].as_str().unwrap();
        if e["id"] == *last {
            0
        } else {
            search[..search.find(&format!(",{last}")).unwrap()]
                .encode_utf16()
                .count()
                + 1
        }
    };
    matches.sort_by(|a, b| {
        rank(a)
            .cmp(&rank(b))
            .then_with(|| a["id"].as_str().cmp(&b["id"].as_str()))
    });
    matches.truncate(90);
    matches
}
