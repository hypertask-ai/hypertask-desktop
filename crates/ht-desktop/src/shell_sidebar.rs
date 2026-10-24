use super::*;
use serde_json::json;
#[derive(Clone)]
pub(super) struct BoardDrag {
    pub project: i64,
    pub title: String,
    pub favorite: bool,
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
            let result = ht_core::Http::new(token).and_then(|mut http| {
                let data = ht_core::board_sidebar::load(&mut http)?;
                let profile = http.json_request("GET", "/api/users/getById", None).ok();
                Ok((data, profile))
            });
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
                            if let Ok(((resolved_user, teams, favorites), profile)) = result {
                                if user != Some(resolved_user) {
                                    let _ = this
                                        .db
                                        .kv_set("current-user-id", &resolved_user.to_string());
                                }
                                let _ = this.db.cache_sidebar(resolved_user, &teams, &favorites);
                                if let Some(profile) = profile {let _ = this.db.cache_sidebar_profile(resolved_user, &profile);}
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
        let show_favorites = self.shell.favorites_expanded;
        let mut result: Vec<_> = favorites
            .into_iter().filter(|_| show_favorites)
            .filter_map(|f| {
                let mut p = f["project"].clone();
                if !p.is_object() {
                    return None;
                }
                p["desktopGroup"] = json!("Favorite Boards");
                p["desktopFavorite"] = json!(true);
                p["desktopFavoriteIndex"] = f["index"].clone();
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
                                        favorite,
                                    },
                                    |drag, _, _, cx| cx.new(|_| drag.clone()),
                                )
                            })
                            .on_drop(cx.listener(move |this, drag: &BoardDrag, _, cx| {
                                cx.stop_propagation();
                                if !drag.favorite && favorite && this.db.sidebar_data("favorites").as_array().into_iter().flatten().any(|f|f["projectId"]==drag.project) {return;}
                                this.drop_favorite(drag.project, Some(id), favorite, cx);
                            }))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.choose_board(id, window, cx)
                            }))
                            .child(title.clone())
                            .when(favorite, |d|d.child(div().ml_auto().text_color(color(t.muted)).child(format!("{} {}",if cfg!(target_os="macos"){"CTRL"}else{"ALT"},board["desktopFavoriteIndex"]))))
                            .layout_key(&self.layout, format!("shell.board.{i}"), Some(title)),
                    )
                    .into_any_element()
            })
            .collect()
    }
    pub(in crate::ui::shell) fn open_favorite(&mut self, index: u8, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.db.sidebar_data("favorites").as_array().into_iter().flatten()
            .find(|f|f["index"] == index).and_then(|f|f["projectId"].as_i64());
        if let Some(id) = id {
            self.shell.boards=true;
            self.choose_board(id,window,cx);
        }
    }
    pub(in crate::ui::shell) fn open_favorites(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shell.boards=false;
        self.shell.favorites_manager=true;
        self.shell.favorite_slot=None;
        self.load_switcher_metadata(window,cx);
        window.focus(&self.focus,cx);
        cx.notify();
    }
    pub(in crate::ui::shell) fn close_favorites(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shell.favorites_manager=false;
        self.shell.favorite_slot=None;
        window.focus(&self.focus,cx);
        cx.notify();
    }
    fn set_favorite_slot(&mut self, index: u8, project: Option<i64>, cx: &mut Context<Self>) {
        self.shell.favorite_slot=None;
        self.enqueue(Write::FavoriteSlot { index, project_id:project },cx);
    }
    pub(in crate::ui::shell) fn favorites_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let t=self.colors;
        let favorites=self.db.sidebar_data("favorites");
        let mut slots=div().id("favorite-slots").max_h(px(364.)).overflow_y_scroll().flex().flex_col();
        for position in 0..10u8 {
            let index=if position==9 {0}else{position+1};
            let title=favorites.as_array().into_iter().flatten().find(|f|f["index"]==index)
                .and_then(|f|f["project"]["title"].as_str()).unwrap_or("Select Board").to_owned();
            let mut slot=div().px(px(16.)).py(px(4.)).flex().flex_col().child(div().h(px(36.)).flex().items_center().justify_between()
                .child(format!("{} {index}",if cfg!(target_os="macos"){"CTRL"}else{"ALT"}))
                .child(div().id(("favorite-slot",position as usize)).w(px(244.)).px(px(8.)).py(px(4.)).border_1().border_color(color(t.border)).cursor_pointer().child(format!("{title}  ▾"))
                    .on_click(cx.listener(move|this,_,_,cx|{this.shell.favorite_slot=if this.shell.favorite_slot==Some(index){None}else{Some(index)};cx.notify();}))
                    .layout_key(&self.layout,format!("shell.favorites.slot.{index}"),Some(title))));
            if self.shell.favorite_slot==Some(index) {
                let mut choices=div().id(("favorite-choices",position as usize)).max_h(px(200.)).overflow_y_scroll().ml_auto().w(px(244.)).p(px(8.)).bg(color(t.card))
                    .child(div().id(("favorite-reset",position as usize)).p(px(6.)).cursor_pointer().child("Do Nothing")
                        .on_click(cx.listener(move|this,_,_,cx|this.set_favorite_slot(index,None,cx)))
                        .layout_key(&self.layout,format!("shell.favorites.reset.{index}"),None));
                for (team_index,team) in self.db.sidebar_data("teams").as_array().into_iter().flatten().enumerate() {
                    choices=choices.child(div().text_color(color(t.muted)).child(team["title"].as_str().unwrap_or("").to_owned()));
                    for (board_index,board) in team["projects"].as_array().into_iter().flatten().enumerate() {
                        let Some(project)=board["id"].as_i64() else {continue;};
                        choices=choices.child(div().id((gpui::SharedString::from(format!("favorite-board-{team_index}")),board_index)).p(px(6.)).cursor_pointer().hover(|d|d.bg(color(t.hover)))
                            .child(board["title"].as_str().unwrap_or("").to_owned())
                            .on_click(cx.listener(move|this,_,_,cx|this.set_favorite_slot(index,Some(project),cx)))
                            .layout_key(&self.layout,format!("shell.favorites.board.{project}"),None));
                    }
                }
                slot=slot.child(choices);
            }
            slots=slots.child(slot);
        }
        div().absolute().inset_0().bg(rgba(0x00000040))
            .on_mouse_down(MouseButton::Left,cx.listener(|this,_,window,cx|this.close_favorites(window,cx)))
            .child(div().id("manage-favorites-modal").absolute().top(px(100.)).left(gpui::relative(0.5)).ml(px(-280.)).w(px(560.)).rounded(px(8.)).bg(color(t.container))
                .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation())
                .child(div().px(px(20.)).py(px(16.)).flex().justify_between().child("Manage Favorites")
                    .child(div().id("favorites-close").cursor_pointer().child(icon("x",t.muted,18.))
                        .on_click(cx.listener(|this,_,window,cx|this.close_favorites(window,cx)))
                        .layout_key(&self.layout,"shell.favorites.close",None)))
                .child(slots).layout_key(&self.layout,"shell.favorites.manager",None)).into_any_element()
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
        if favorite && !was_favorite && ids.len()>=10 {return;}
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
    fn favorites_manager_has_ten_real_slots_and_clicks_queue_web_slot_bodies(cx: &mut gpui::TestAppContext) {
        use crate::ui::shell::tests::click;
        let(view,mut cx)=setup(cx);
        view.update(&mut cx,|this,window,cx|{
            this.db.cache_sidebar(1,&json!([{"id":"team-3","title":"Team Red","projects":[{"id":15,"title":"Alpha"},{"id":16,"title":"Beta"}]}]),&json!([])).unwrap();
            this.db.cache_sidebar_profile(1,&json!({"id":1,"UserSettingId":99})).unwrap();
            this.run_shell_command(crate::command_center::ShellCommand::Favorites,window,cx);
        }).unwrap();
        measured(view,&mut cx,"shell.favorites.manager");
        for index in 0..10 {measured(view,&mut cx,&format!("shell.favorites.slot.{index}"));}
        click(view,&mut cx,"shell.favorites.slot.1");
        click(view,&mut cx,"shell.favorites.board.16");
        view.read_with(&cx,|this,_|{
            let rows=this.db.mutations().unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0].kind,"web_favorite_slot");
            assert_eq!(rows[0].payload_json["body"],json!({"userSettingId":99,"index":1,"projectId":16}));
        }).unwrap();
        cx.simulate_keystrokes("escape ctrl-b");
        measured(view,&mut cx,"shell.favorites.collapse");
        click(view,&mut cx,"shell.favorites.collapse");
        view.update(&mut cx,|this,_,cx|{
            assert!(!this.shell.favorites_expanded);
            assert!(!this.shell_boards(cx).iter().any(|b|b["desktopFavorite"]==true));
        }).unwrap();
        cx.simulate_keystrokes(if cfg!(target_os="macos"){"ctrl-1"}else{"alt-1"});
        view.read_with(&cx,|this,_|{assert_eq!(this.options.board,16);assert!(!this.shell.boards);}).unwrap();
    }
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
