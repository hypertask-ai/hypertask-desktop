use crate::{Result, Transport};
use anyhow::{Context, ensure};
use serde_json::Value;

pub fn boards(transport: &mut impl Transport) -> Result<Vec<Value>> {
    let response = transport.get("/api/projects/getAllMinimal?mode=ExtraMinimal")?;
    ensure!(
        response.status == 200,
        "{}",
        response.body["message"]
            .as_str()
            .or(response.body["error"].as_str())
            .unwrap_or("Could not load boards")
    );
    let rows = response.body.as_array().context("Could not load boards")?;
    ensure!(
        rows.iter().all(
            |board| board["id"].as_i64().is_some_and(|id| id > 0) && board["title"].is_string()
        ),
        "Could not load boards"
    );
    Ok(rows.clone())
}
fn normalized(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn bounded(text: &str, needle: &str) -> bool {
    let word = |ch: char| ch.is_alphanumeric() || ch == '_';
    text.match_indices(needle).any(|(at, _)| {
        text[..at].chars().next_back().is_none_or(|ch| !word(ch))
            && text[at + needle.len()..]
                .chars()
                .next()
                .is_none_or(|ch| !word(ch))
    })
}
/// Web composeTaskBoard.ts resolves a reference only when exactly one board matches.
pub fn match_board(text: &str, boards: &[Value]) -> Option<i64> {
    let note = normalized(text);
    let names = |board: &Value| {
        ["title", "uniqueIdentifier"]
            .into_iter()
            .filter_map(|field| board[field].as_str().map(normalized))
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>()
    };
    let references: Vec<_> = boards
        .iter()
        .flat_map(|board| {
            names(board).into_iter().filter(|name| {
                ["for", "on", "in", "into", "to"].iter().any(|preposition| {
                    ["", "the "].iter().any(|article| {
                        ["", "project ", "board "].iter().any(|kind| {
                            ["", "\"", "'", "‘", "’", "“", "”"].iter().any(|quote| {
                                bounded(
                                    &note,
                                    &format!("{preposition} {article}{kind}{quote}{name}"),
                                )
                            })
                        })
                    })
                })
            })
        })
        .collect();
    let matches: Vec<_> = boards
        .iter()
        .filter(|board| {
            names(board)
                .iter()
                .any(|name| references.iter().any(|reference| bounded(reference, name)))
        })
        .collect();
    if matches.len() == 1 {
        matches[0]["id"].as_i64()
    } else {
        None
    }
}
