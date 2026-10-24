use super::*;
use serde_json::json;
#[derive(Clone)]
pub(super) struct BoardDrag {
    pub project: i64,
    pub title: String,
}
impl Render for BoardDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p(px(8.))
            .bg(color(0x30343b))
            .text_color(color(0xffffff))
            .child(self.title.clone())
    }
}
impl Desktop {
    pub(in crate::ui::shell) fn load_switcher_metadata(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.options.offline || self.shell.metadata_loading {
            return;
        }
        let Some(token) = self.cli_token() else {
            return;
        };
        self.shell.metadata_loading = true;
        self.shell.metadata_generation += 1;
        let generation = self.shell.metadata_generation;
        let user = self.db.current_user_id();
        let expected_token = token.clone();
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = ht_core::Http::new(token)
                .and_then(|mut http| ht_core::board_sidebar::load(&mut http));
            let _ = send.send(result);
        });
        cx.spawn_in(window, async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                match receive.try_recv() {
                    Ok(result) => {
                        let _ = view.update(cx, |this, cx| {
                            if this.shell.metadata_generation != generation {
                                return;
                            }
                            this.shell.metadata_loading = false;
                            if this.cli_token().as_deref() != Some(expected_token.as_str()) {
                                return;
                            }
                            // Web switcher leaves its last successful data in place on read refusal.
                            if let Ok((resolved_user, teams, favorites)) = result {
                                if user != Some(resolved_user) {
                                    let _ = this
                                        .db
                                        .kv_set("current-user-id", &resolved_user.to_string());
                                }
                                let _ = this.db.cache_sidebar(resolved_user, &teams, &favorites);
                            }
                            cx.notify();
                        });
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => (),
                    _ => break,
                }
            }
        })
        .detach();
    }
    pub(in crate::ui::shell) fn switcher_boards(&self) -> Vec<Value> {
        let used = |id: &Value| {
            self.db
                .kv_get(&format!("boardUsed:{id}"))
                .ok()
                .flatten()
                .and_then(|s| s.parse::<u128>().ok())
                .unwrap_or(0)
        };
        let mut favorites = self
            .db
            .sidebar_data("favorites")
            .as_array()
            .cloned()
            .unwrap_or_default();
        favorites.sort_by_key(|f| match f["index"].as_i64().unwrap_or(0) {
            0 => 10,
            n => n,
        });
        let mut result: Vec<_> = favorites
            .into_iter()
            .filter_map(|f| {
                let mut p = f["project"].clone();
                if !p.is_object() {
                    return None;
                }
                p["desktopGroup"] = json!("Favorite Boards");
                p["desktopFavorite"] = json!(true);
                Some(p)
            })
            .collect();
        let mut teams = self
            .db
            .sidebar_data("teams")
            .as_array()
            .cloned()
            .unwrap_or_default();
        teams.sort_by(|a, b| {
            let latest = |team: &Value| {
                team["projects"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|p| used(&p["id"]))
                    .max()
                    .unwrap_or(0)
            };
            latest(b)
                .cmp(&latest(a))
                .then_with(|| a["title"].as_str().cmp(&b["title"].as_str()))
        });
        let mut boards = vec![];
        for team in teams {
            let mut projects = team["projects"].as_array().cloned().unwrap_or_default();
            projects.sort_by_key(|p| p["id"].as_i64());
            for mut project in projects {
                project["desktopGroup"] = if self.shell.recent {
                    json!("All Boards")
                } else {
                    team["title"].clone()
                };
                boards.push(project);
            }
        }
        if boards.is_empty() {
            boards = self
                .db
                .projects()
                .unwrap_or_default()
                .into_iter()
                .filter(|p| p["status"].as_str().is_none_or(|s| s == "Normal"))
                .collect();
        }
        if self.shell.recent {
            boards.sort_by_key(|p| std::cmp::Reverse(used(&p["id"])));
        }
        result.extend(boards);
        result
    }
    pub(in crate::ui::shell) fn switcher_rows(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let t = self.colors;
        let mut previous = String::from("Favorite Boards");
        self.shell_boards(cx)
            .into_iter()
            .enumerate()
            .map(|(i, board)| {
                let id = board["id"].as_i64().unwrap_or(0);
                let title = board["title"]
                    .as_str()
                    .or(board["name"].as_str())
                    .unwrap_or("")
                    .to_owned();
                let group = board["desktopGroup"].as_str().unwrap_or("").to_owned();
                let heading = !group.is_empty() && group != previous;
                previous = group.clone();
                let favorite = board["desktopFavorite"] == true;
                div()
                    .when(heading, |d| {
                        d.child(
                            div()
                                .px(px(16.))
                                .pt(px(12.))
                                .text_color(color(t.muted))
                                .child(group),
                        )
                    })
                    .child(
                        div()
                            .id(("shell-board", i))
                            .px(px(16.))
                            .py(px(8.))
                            .cursor_pointer()
                            .when(i == self.shell.choice, |d| d.bg(color(t.active)))
                            .when(self.shell.query.read(cx).content.is_empty(), |d| {
                                d.on_drag(
                                    BoardDrag {
                                        project: id,
                                        title: title.clone(),
                                    },
                                    |drag, _, _, cx| cx.new(|_| drag.clone()),
                                )
                            })
                            .on_drop(cx.listener(move |this, drag: &BoardDrag, _, cx| {
                                cx.stop_propagation();
                                this.drop_favorite(drag.project, Some(id), favorite, cx);
                            }))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.choose_board(id, window, cx)
                            }))
                            .child(title.clone())
                            .layout_key(&self.layout, format!("shell.board.{i}"), Some(title)),
                    )
                    .into_any_element()
            })
            .collect()
    }
    pub(in crate::ui::shell) fn drop_favorite(
        &mut self,
        project: i64,
        before: Option<i64>,
        favorite: bool,
        cx: &mut Context<Self>,
    ) {
        let mut rows = self
            .db
            .sidebar_data("favorites")
            .as_array()
            .cloned()
            .unwrap_or_default();
        rows.sort_by_key(|f| match f["index"].as_i64().unwrap_or(0) {
            0 => 10,
            n => n,
        });
        let mut ids: Vec<_> = rows
            .iter()
            .filter_map(|f| f["projectId"].as_i64())
            .collect();
        let was_favorite = ids.contains(&project);
        ids.retain(|id| *id != project);
        if favorite {
            let index = before
                .and_then(|id| ids.iter().position(|p| *p == id))
                .unwrap_or(ids.len());
            ids.insert(index, project);
        } else if !was_favorite {
            return;
        }
        self.enqueue(Write::Favorites { project_ids: ids }, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::tests::{measured, setup};
    #[gpui::test]
    fn ctrl_b_groups_authoritative_teams_and_favorites_and_queues_real_reordering(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        view.update(&mut cx,|this,_,_|{
            this.db.cache_sidebar(1,&json!([{"id":"team-3","title":"Team Red","projects":[{"id":15,"title":"Alpha"},{"id":16,"title":"Beta"}]}]),&json!([{"projectId":16,"index":1,"project":{"id":16,"title":"Beta"}}])).unwrap();
        }).unwrap();
        cx.simulate_keystrokes("ctrl-b");
        assert_eq!(measured(view, &mut cx, "shell.board.0")["text"], "Beta");
        view.update(&mut cx, |this, _, cx| {
            let rows = this.shell_boards(cx);
            assert_eq!(rows[0]["desktopGroup"], "Favorite Boards");
            assert_eq!(rows[1]["desktopGroup"], "Team Red");
            this.drop_favorite(15, Some(16), true, cx);
            assert_eq!(this.db.sidebar_data("favorites")[0]["projectId"], 15);
            assert_eq!(this.db.mutations().unwrap()[0].kind, "web_favorites");
        })
        .unwrap();
        cx.simulate_keystrokes("escape ctrl-b");
        assert_eq!(measured(view, &mut cx, "shell.board.0")["text"], "Alpha");
    }
}
