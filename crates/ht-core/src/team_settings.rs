use crate::{Result, Transport};
use anyhow::ensure;
use serde_json::{Value, json};

#[derive(Clone, Debug)]
pub struct TeamSettings {
    pub user: Value,
    pub team: Value,
    pub members: Value,
    pub keys: Value,
    pub providers: Value,
    pub flags: Value,
}
fn read(transport: &mut impl Transport, path: &str) -> Result<Value> {
    let response = transport.get(path)?;
    ensure!(
        response.status == 200 && response.body["success"] != false,
        "{}",
        response.body["message"]
            .as_str()
            .or(response.body["error"].as_str())
            .unwrap_or("Request failed")
    );
    Ok(response.body)
}
pub fn load_for_scope(
    transport: &mut impl Transport,
    selected: Option<&str>,
    current: Option<&str>,
    previous: Option<&str>,
    keys: bool,
) -> Result<TeamSettings> {
    let identity = read(transport, "/user/context")?;
    let user = identity["user"]["id"]
        .as_i64()
        .filter(|id| *id > 0)
        .ok_or_else(|| anyhow::anyhow!("Unauthorized"))?;
    let response = transport.post(
        "/api/teams/getAllSidebar",
        &json!({"userId":user}),
        "settings-team-read",
    )?;
    ensure!(
        response.status == 200,
        "{}",
        response.body["message"]
            .as_str()
            .or(response.body["error"].as_str())
            .unwrap_or("Could not load teams")
    );
    let teams = response
        .body
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Could not load teams"))?;
    let team = [selected, current, previous]
        .into_iter()
        .flatten()
        .find(|id| teams.iter().any(|team| team["id"] == *id))
        .or_else(|| teams.first().and_then(|team| team["id"].as_str()))
        .ok_or_else(|| anyhow::anyhow!("No teams available"))?;
    load(transport, team, keys)
}
pub fn load(transport: &mut impl Transport, team: &str, keys: bool) -> Result<TeamSettings> {
    let identity = read(transport, "/user/context")?;
    ensure!(
        identity["user"]["id"].as_i64().is_some_and(|id| id > 0),
        "Unauthorized"
    );
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("teamId", team)
        .finish();
    let team_data = read(transport, &format!("/api/teams/getTeam?{query}"))?;
    ensure!(team_data["id"] == team, "Wrong settings team");
    let members = read(
        transport,
        &format!("/api/members/getAllTeamMembers?{query}"),
    )?;
    ensure!(members.is_object(), "Could not load team members");
    let (keys, providers) = if keys {
        let keys = read(transport, &format!("/api/teams/byokKeys?{query}"))?;
        let providers = read(transport, &format!("/api/teams/aiProviderSettings?{query}"))?;
        ensure!(
            keys["keys"].is_array() && providers["providers"].is_array(),
            "Could not load API keys"
        );
        (keys, providers)
    } else {
        (Value::Null, Value::Null)
    };
    let flags = transport
        .get("/api/flags")
        .ok()
        .filter(|response| response.status == 200)
        .map(|response| response.body["flags"].clone())
        .unwrap_or(Value::Null);
    Ok(TeamSettings {
        user: identity["user"].clone(),
        team: team_data,
        members,
        keys,
        providers,
        flags,
    })
}
impl TeamSettings {
    pub fn owner(&self) -> bool {
        !self.user["id"].is_null() && self.members["owner"]["id"] == self.user["id"]
    }
    pub fn manage_keys(&self) -> bool {
        self.owner()
            || self.user["accountId"]
                .as_str()
                .filter(|account| !account.is_empty())
                .is_some_and(|account| self.team["googleAccountId"] == account)
    }
    pub fn plan(&self) -> String {
        crate::ai_models::plan(&self.team).unwrap_or_else(|| "Free".into())
    }
    pub fn yearly(&self) -> bool {
        self.billing_interval() == Some(true)
    }
    pub fn billing_interval(&self) -> Option<bool> {
        let comped = self.team["compedUntil"]
            .as_str()
            .and_then(|date| chrono::DateTime::parse_from_rfc3339(date).ok())
            .is_some_and(|date| date.timestamp() > chrono::Utc::now().timestamp());
        if comped
            && self.plan()
                == if self.team["compedPlan"] == "BYOK" {
                    "BYOK"
                } else {
                    "Pro"
                }
        {
            return None;
        }
        let rows = self.team["subscriptionPlan"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let entitled = |row: &&Value| {
            row["subscriptionStatus"].as_str().is_some_and(|status| {
                matches!(
                    status.to_ascii_lowercase().as_str(),
                    "paid" | "active" | "trialing" | "past_due"
                )
            })
        };
        let row = rows
            .iter()
            .filter(entitled)
            .find(|row| row["subscriptionId"] == self.team["activeSubscriptionPlanId"])
            .or_else(|| rows.iter().find(entitled));
        row.and_then(|row| row["priceId"].as_str()).and_then(|id| {
            if [
                "price_1QjJeDIhmcH60VcciHzZ3mTJ",
                "price_1TMlqDIhmcH60VccRZGW1xK3",
                "price_1QCKkpIhmcH60Vcc2RqVACTc",
            ]
            .contains(&id)
            {
                return Some(false);
            }
            if id == "price_1QjJf8IhmcH60VccNUrF7YJf" {
                return Some(true);
            }
            let monthly = [
                option_env!("NEXT_PUBLIC_STRIPE_MONTHLY_PRICE_ID_01")
                    .filter(|id| !id.is_empty())
                    .unwrap_or("price_1OSg0zIhmcH60VccTpAFDv32"),
                option_env!("NEXT_PUBLIC_STRIPE_MONTHLY_PRICE_ID_02")
                    .filter(|id| !id.is_empty())
                    .unwrap_or("price_1QjJeDIhmcH60VcciHzZ3mTJ"),
                option_env!("NEXT_PUBLIC_STRIPE_MONTHLY_PRICE_ID_BYOK")
                    .filter(|id| !id.is_empty())
                    .unwrap_or("price_1TMlqCIhmcH60Vccgq4fpSWu"),
                option_env!("NEXT_PUBLIC_STRIPE_MONTHLY_PRICE_ID_PRO")
                    .filter(|id| !id.is_empty())
                    .unwrap_or("price_1QjJeDIhmcH60VcciHzZ3mTJ"),
            ];
            let yearly = [
                option_env!("NEXT_PUBLIC_STRIPE_YEARLY_PRICE_ID_01")
                    .filter(|id| !id.is_empty())
                    .unwrap_or("price_1OSg35IhmcH60VccJP5LQo03"),
                option_env!("NEXT_PUBLIC_STRIPE_YEARLY_PRICE_ID_02")
                    .filter(|id| !id.is_empty())
                    .unwrap_or("price_1QjJf8IhmcH60VccNUrF7YJf"),
                option_env!("NEXT_PUBLIC_STRIPE_YEARLY_PRICE_ID_BYOK")
                    .filter(|id| !id.is_empty())
                    .unwrap_or("price_1TMlqCIhmcH60VccI6VjW2FM"),
                option_env!("NEXT_PUBLIC_STRIPE_YEARLY_PRICE_ID_PRO")
                    .filter(|id| !id.is_empty())
                    .unwrap_or("price_1QjJf8IhmcH60VccNUrF7YJf"),
            ];
            if monthly.contains(&id) {
                Some(false)
            } else if yearly.contains(&id) {
                Some(true)
            } else {
                None
            }
        })
    }
    pub fn checkout(&self, plan: &str, yearly: bool) -> Result<Value> {
        ensure!(self.owner(), "Owner only");
        for key in ["id", "stripe_customer_id", "googleAccountId", "title"] {
            ensure!(
                self.team[key]
                    .as_str()
                    .is_some_and(|value| !value.is_empty()),
                "No billing account"
            );
        }
        let price = match (plan, yearly) {
            ("BYOK", false) => option_env!("NEXT_PUBLIC_STRIPE_MONTHLY_PRICE_ID_BYOK")
                .filter(|id| !id.is_empty())
                .unwrap_or("price_1TMlqCIhmcH60Vccgq4fpSWu"),
            ("BYOK", true) => option_env!("NEXT_PUBLIC_STRIPE_YEARLY_PRICE_ID_BYOK")
                .filter(|id| !id.is_empty())
                .unwrap_or("price_1TMlqCIhmcH60VccI6VjW2FM"),
            ("Pro", false) => option_env!("NEXT_PUBLIC_STRIPE_MONTHLY_PRICE_ID_PRO")
                .filter(|id| !id.is_empty())
                .unwrap_or("price_1QjJeDIhmcH60VcciHzZ3mTJ"),
            ("Pro", true) => option_env!("NEXT_PUBLIC_STRIPE_YEARLY_PRICE_ID_PRO")
                .filter(|id| !id.is_empty())
                .unwrap_or("price_1QjJf8IhmcH60VccNUrF7YJf"),
            _ => anyhow::bail!("Unknown plan"),
        };
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        for key in [
            "teamId",
            "totalSeats",
            "googleAccountId",
            "stripe_customer_id",
            "teamTitle",
        ] {
            let field = match key {
                "teamId" => "id",
                "teamTitle" => "title",
                key => key,
            };
            query.append_pair(
                key,
                &self.team[field]
                    .as_str()
                    .map(str::to_owned)
                    .unwrap_or_else(|| self.team[field].to_string()),
            );
        }
        let base = format!("https://app.hypertask.ai/pricing?{}", query.finish());
        Ok(
            json!({"teamTitle":self.team["title"],"googleAccountId":self.team["googleAccountId"],"quantity":self.team["totalSeats"],"stripe_customer_id":self.team["stripe_customer_id"],"teamId":self.team["id"],"priceId":price,"mode":"Normal","returnUrl":format!("{base}&success=1"),"cancelUrl":format!("{base}&preSelect=Upgrade&success=0")}),
        )
    }
}
