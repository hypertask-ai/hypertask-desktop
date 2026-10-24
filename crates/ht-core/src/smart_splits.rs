use crate::{Result, Transport, ViewWrite};
use anyhow::{Context, ensure};
use serde_json::{Value, json};

const PENDING_ID: &str = "desktop:pending-smart-split";

fn references(view: &Value) -> Vec<&str> {
    view["board_filters"]["addedFilters"].as_array().into_iter().flatten()
        .filter(|filter| filter["type"] == "Labels")
        .flat_map(|filter| filter["searchPayload"].as_array().into_iter().flatten())
        .filter_map(|label| label["id"].as_str()).filter(|id| !id.is_empty()).collect()
}

pub fn label<'a>(view: &Value, labels: &'a [Value], views: &[Value]) -> Option<&'a Value> {
    let ids = references(view);
    let smart = |label: &&Value| label["ai_prompt"].as_str().is_some_and(|prompt| !prompt.trim().is_empty());
    if let Some(label) = labels.iter().filter(smart).find(|label| label["id"] == view["id"] && label["id"].as_str().is_some_and(|id| ids.contains(&id))) { return Some(label); }
    let matches: Vec<_> = ids.iter().filter_map(|id| labels.iter().filter(smart).find(|label| label["id"] == *id)).collect();
    if matches.len() != 1 { return None; }
    let id = matches[0]["id"].as_str()?;
    let linked: Vec<_> = views.iter().filter(|view| references(view).contains(&id)).collect();
    (linked.len() == 1 && linked[0]["id"] == view["id"]).then_some(matches[0])
}

pub fn labels(transport: &mut impl Transport, project: i64) -> Result<Value> {
    let response = transport.get(&format!("/api/labels/getByProject?projectId={project}"))?;
    ensure!(response.status == 200, "{}", response.body["message"].as_str().unwrap_or("Could not load smart split details"));
    let rows = response.body.as_array().context("Could not load smart split details")?;
    ensure!(rows.iter().all(|label| label["id"].is_string() && label["projectId"] == project), "Invalid smart split details");
    Ok(response.body)
}

impl ViewWrite {
    pub(crate) fn smart_split(&self) -> bool { self.route.starts_with("smart-split-") }
    pub(crate) fn validate_smart_split(&self) -> Result<()> {
        if self.route != "smart-split-create" {
            ensure!(self.body["viewId"].as_str().is_some_and(|id| !id.is_empty() && id != PENDING_ID), "Select a smart split");
        }
        if self.route != "smart-split-delete" {
            ensure!(self.body["name"].as_str().is_some_and(|name| !name.trim().is_empty()), "Give the smart split a name");
            ensure!(self.body["prompt"].as_str().is_some_and(|prompt| !prompt.trim().is_empty()), "Describe which tasks should match");
            ensure!(self.body["prompt"].as_str().unwrap().encode_utf16().count() <= 1000, "Keep the prompt to 1,000 characters or fewer");
        }
        Ok(())
    }
    pub(crate) fn optimistic_smart_split(&self, project: &mut Value) {
        if self.route == "smart-split-create" {
            let mut view = project["project_view"]["default_view"].clone();
            if !view.is_object() { view = json!({}); }
            view["id"] = json!(PENDING_ID);
            view["title"] = self.body["name"].clone();
            view["visibility"] = json!("Public");
            view["pending"] = json!(true);
            view["board_filters"] = json!({"matchFilters":"ALL","addedFilters":[{"type":"Labels","match":"ANY","searchPayload":[{"id":PENDING_ID,"value":self.body["name"]}]}]});
            if !project["project_view"]["allViews"].is_array() { project["project_view"]["allViews"] = json!([]); }
            let views = project["project_view"]["allViews"].as_array_mut().unwrap();
            views.retain(|view| view["id"] != PENDING_ID);
            views.push(view);
            return;
        }
        let views = project["project_view"]["allViews"].as_array().cloned().unwrap_or_default();
        let Some(view) = views.iter().find(|view| view["id"] == self.body["viewId"]) else { return; };
        let labels = project["desktop_smart_labels"].as_array().cloned().unwrap_or_default();
        let Some(label) = label(view, &labels, &views) else { return; };
        let label_id = label["id"].clone();
        let deleting = self.route == "smart-split-delete";
        if let Some(labels) = project["desktop_smart_labels"].as_array_mut() {
            if deleting { labels.retain(|label| label["id"] != label_id); }
            else if let Some(label) = labels.iter_mut().find(|label| label["id"] == label_id) {
                label["value"] = self.body["name"].clone(); label["ai_prompt"] = self.body["prompt"].clone();
            }
        }
        let patch = |view: &mut Value| {
            if !view.is_object() { return; }
            if view["id"] == self.body["viewId"] {
                if deleting { *view = Value::Null; return; }
                view["title"] = self.body["name"].clone();
            }
            if let Some(filters) = view["board_filters"]["addedFilters"].as_array_mut() {
                for filter in filters.iter_mut().filter(|filter| filter["type"] == "Labels") {
                    if let Some(items) = filter["searchPayload"].as_array_mut() {
                        if deleting { items.retain(|item| item["id"] != label_id); }
                        else { for item in items.iter_mut().filter(|item| item["id"] == label_id) { item["value"] = self.body["name"].clone(); } }
                    }
                }
                if deleting { filters.retain(|filter| filter["type"] != "Labels" || filter["searchPayload"].as_array().is_some_and(|items| !items.is_empty())); }
            }
        };
        let pv = &mut project["project_view"];
        for view in pv["allViews"].as_array_mut().into_iter().flatten() { patch(view); }
        if let Some(views) = pv["allViews"].as_array_mut() { views.retain(|view| !view.is_null()); }
        patch(&mut pv["default_view"]);
        for user in pv["user_project_views"].as_array_mut().into_iter().flatten() {
            patch(&mut user["appliedView"]); patch(&mut user["unsavedView"]);
            if deleting { if let Some(order) = user["view_order"].as_array_mut() { order.retain(|id| *id != self.body["viewId"]); } }
        }
        if deleting { if let Some(order) = pv["default_view_order"].as_array_mut() { order.retain(|id| *id != self.body["viewId"]); } }
    }
    pub(crate) fn resolve_smart_split(&self, project: &mut Value, response: &Value) -> Result<()> {
        if self.route == "smart-split-create" {
            let (view, label) = (&response["view"], &response["label"]);
            ensure!(view["id"].as_str().is_some_and(|id| !id.is_empty() && id != PENDING_ID) && label["id"] == view["id"] && label["projectId"] == self.project && view["project_view_id"] == project["project_view"]["id"], "Wrong smart split acknowledgement");
            let views = project["project_view"]["allViews"].as_array_mut().context("Missing board views")?;
            views.retain(|row| row["id"] != PENDING_ID && row["id"] != view["id"]); views.push(view.clone());
            if !project["desktop_smart_labels"].is_array() { project["desktop_smart_labels"] = json!([]); }
            let labels = project["desktop_smart_labels"].as_array_mut().unwrap();
            labels.retain(|row| row["id"] != label["id"]); labels.push(label.clone());
        } else {
            ensure!(response["success"] == true, "Missing smart split acknowledgement");
            if self.route == "smart-split-edit" {
                ensure!(response.get("slug").is_some() && (response["slug"].is_string() || response["slug"].is_null()), "Missing smart split slug");
                let patch = |view: &mut Value| { if view["id"] == self.body["viewId"] { view["slug"] = response["slug"].clone(); } };
                let pv = &mut project["project_view"];
                for view in pv["allViews"].as_array_mut().into_iter().flatten() { patch(view); }
                patch(&mut pv["default_view"]);
                for row in pv["user_project_views"].as_array_mut().into_iter().flatten() { patch(&mut row["appliedView"]); }
            }
        }
        Ok(())
    }
}
