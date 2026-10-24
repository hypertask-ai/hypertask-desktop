use ht_core::{
    Transport, compose_routing,
    http::{Response, TransportError},
};
use serde_json::{Value, json};
fn boards() -> Vec<Value> {
    vec![
        json!({"id":1,"title":"Product","uniqueIdentifier":"PRD"}),
        json!({"id":2,"title":"Release Notes","uniqueIdentifier":"RN"}),
        json!({"id":3,"title":"C++ [β]","uniqueIdentifier":"CPP"}),
    ]
}
#[test]
fn typed_board_routing_matches_web_prepositions_quotes_whitespace_identifiers_and_boundaries() {
    for (text, expected) in [
        ("Write for Product", Some(1)),
        ("task on the board ‘Release Notes’", Some(2)),
        ("ship into project PRD", Some(1)),
        ("note to \"C++ [β]\"", Some(3)),
        ("in the\n board   release\t notes", Some(2)),
        ("in PRDX", None),
        ("origin PRD", None),
        ("_in PRD", None),
        ("in PRD_", None),
        ("in PRDé", None),
        ("Product needs a fix", None),
        ("for Product and in RN", None),
    ] {
        assert_eq!(
            compose_routing::match_board(text, &boards()),
            expected,
            "{text}"
        );
    }
    let overlapping = vec![
        json!({"id":1,"title":"Product"}),
        json!({"id":2,"title":"Product Team"}),
    ];
    assert_eq!(
        compose_routing::match_board("for Product Team", &overlapping),
        None
    );
    let unicode = vec![json!({"id":1,"title":"Équipe été"})];
    assert_eq!(
        compose_routing::match_board("for ÉQUIPE ÉTÉ", &unicode),
        Some(1)
    );
}
#[test]
fn routing_reads_the_web_board_catalog_and_rejects_accepting_shaped_auth_refusals() {
    struct Server(u16);
    impl Transport for Server {
        fn get(&mut self, path: &str) -> Result<Response, TransportError> {
            assert_eq!(path, "/api/projects/getAllMinimal?mode=ExtraMinimal");
            Ok(Response {
                status: self.0,
                body: json!(boards()),
            })
        }
        fn post(&mut self, _: &str, _: &Value, _: &str) -> Result<Response, TransportError> {
            panic!("read only")
        }
    }
    for status in [200, 401, 403] {
        let result = compose_routing::boards(&mut Server(status));
        if status == 200 {
            assert_eq!(result.unwrap(), boards());
        } else {
            assert!(result.is_err());
        }
    }
}
