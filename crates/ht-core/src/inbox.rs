use crate::{Db,Result};
use anyhow::{Context,ensure};
use serde_json::{Value,json};

pub(crate) fn notification_id(value: &Value) -> Result<i64> {
    if let Some(id) = value["id"].as_i64() {
        return Ok(id);
    }
    let text = value["id"].as_str().context("notification id missing")?;
    let id = text.parse::<i64>().context("invalid synthetic notification id")?;
    ensure!(
        value["waitingOnSynthetic"] == true
            && id < 0
            && id.to_string() == text
            && value["taskId"].as_i64().and_then(i64::checked_neg) == Some(id),
        "invalid synthetic notification id"
    );
    Ok(id)
}

impl Db {
    pub(crate) fn cache_inbox_structure(&self,response:&Value)->Result<()> {
        if response.get("user_notifications").is_none() {return Ok(());}
        let key=format!("inbox-splits:{}",self.current_user_id().unwrap_or(0));
        let Some(structure)=response.get("user_structured_data").or_else(||response.get("structuredData")) else {return self.kv_set(&key,"null");};
        let tabs=structure["tabs"].as_array().context("inbox tabs missing")?;
        let data=structure["data"].as_array().context("inbox split rows missing")?;
        let notifications=response["user_notifications"].as_array().context("inbox notifications missing")?;
        ensure!(tabs.len()==data.len(),"inbox tab/data length mismatch");
        let tabs=tabs.iter().zip(data).map(|(tab,rows)|{
            let name=tab["project"].as_str().context("inbox tab name missing")?;
            let ids=rows.as_array().context("inbox split must be an array")?.iter().map(|entry|{
                notification_id(entry.as_u64().and_then(|i|notifications.get(i as usize)).unwrap_or(entry))
                    .context("inbox split index is invalid")
            }).collect::<Result<Vec<_>>>()?;
            Ok(json!({"project":name,"projectId":tab["projectId"],"ids":ids}))
        }).collect::<Result<Vec<_>>>()?;
        self.kv_set(&key,&json!(tabs).to_string())
    }
    pub fn inbox_splits(&self)->Result<Option<Vec<Value>>> {
        self.kv_get(&format!("inbox-splits:{}",self.current_user_id().unwrap_or(0)))?
            .map(|value|serde_json::from_str(&value).map_err(Into::into)).transpose().map(Option::flatten)
    }
}
