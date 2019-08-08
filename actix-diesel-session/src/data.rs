use crate::schema::user_sessions;
use serde_json::Value;
#[derive(Insertable, AsChangeset, Queryable, Associations, Serialize, Deserialize, Debug, Clone)]
#[table_name = "user_sessions"]
pub struct UserSession {
    pub id: i32,
    pub skey: String,
    pub sdata: Option<Value>,
}
