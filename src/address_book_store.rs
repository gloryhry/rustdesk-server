use crate::{address_book_codec, database::{ApiAddressBookEntry,Database}};
use hbb_common::{anyhow,ResultType};
use serde_json::{json,Map,Value};
use sqlx::{Connection,Sqlite,Transaction};
use std::{collections::HashSet,fmt,ops::DerefMut};

#[derive(Debug)]
pub enum BookError { Invalid(String), Conflict, Storage }
impl fmt::Display for BookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self { Self::Invalid(report) => write!(f,"{report}"), Self::Conflict => write!(f,"address_book_revision_conflict"), Self::Storage => write!(f,"address_book_storage_error") }
    }
}
impl std::error::Error for BookError {}

pub struct Book { pub guid: String, pub revision: i64, pub document: Map<String,Value> }
#[derive(Clone)]
pub struct AddressBookStore { db: Database }
impl AddressBookStore {
    pub fn new(db: Database) -> Self { Self { db } }
    pub async fn load(&self, user: &str) -> Result<Book,BookError> {
        let mut conn = self.db.book_connection().await.map_err(|_|BookError::Storage)?;
        let row: Option<(String,Option<String>,i64)> = sqlx::query_as("select data,guid,revision from api_address_book_snapshot where user_id=?")
            .bind(user).fetch_optional(conn.deref_mut()).await.map_err(|_|BookError::Storage)?;
        if let Some((data,Some(guid),revision)) = row {
            if uuid::Uuid::parse_str(&guid).is_err() || revision<1 { return Err(invalid("invalid_address_book_metadata")); }
            let mut document = parse(&data)?;
            let original = document.clone(); canonical(&mut document)?;
            if document!=original { return Err(invalid("address_book_requires_repair")); }
            return Ok(Book { guid,revision,document });
        }
        let mut tx = conn.begin().await.map_err(|_|BookError::Storage)?;
        sqlx::query("update api_schema_lock set id=id where id=1").execute(&mut tx).await.map_err(|_|BookError::Storage)?;
        let book = initialize(&mut tx,user).await.map_err(|error|BookError::Invalid(error.to_string()))?;
        tx.commit().await.map_err(|_|BookError::Storage)?;
        Ok(book)
    }
    pub async fn replace(&self, user: &str, expected: Option<i64>, mut document: Map<String,Value>) -> Result<Book,BookError> {
        let explicit_ids = document.get("peers").and_then(Value::as_array).into_iter().flatten()
            .filter(|peer|peer.get("entryId").or_else(||peer.get("guid")).is_some()).filter_map(peer_id).map(str::to_owned).collect::<HashSet<_>>();
        canonical(&mut document)?;
        let mut conn = self.db.book_connection().await.map_err(|_|BookError::Storage)?;
        let mut tx = conn.begin().await.map_err(|_|BookError::Storage)?;
        sqlx::query("update api_schema_lock set id=id where id=1").execute(&mut tx).await.map_err(|_|BookError::Storage)?;
        let current: Option<(Option<String>,i64,String)> = sqlx::query_as("select guid,revision,data from api_address_book_snapshot where user_id=?")
            .bind(user).fetch_optional(&mut tx).await.map_err(|_|BookError::Storage)?;
        let (guid,revision,previous) = if let Some((Some(guid),revision,data)) = current {
            if uuid::Uuid::parse_str(&guid).is_err() || revision<1 { return Err(invalid("invalid_address_book_metadata")); }
            let mut previous = parse(&data)?;
            let original = previous.clone(); canonical(&mut previous)?;
            if previous!=original { return Err(invalid("address_book_requires_repair")); }
            (guid,revision,previous)
        } else {
            let book = initialize(&mut tx,user).await.map_err(|error|BookError::Invalid(error.to_string()))?;
            (book.guid,book.revision,book.document)
        };
        if expected.is_some_and(|expected|expected!=revision) { return Err(BookError::Conflict); }
        // Entry identities belong to this book. A legacy upload without server
        // metadata keeps matching Peer identities; importing another book cannot
        // reuse its internal primary keys. Unknown connection fields are untouched.
        let previous_peers = previous.get("peers").and_then(Value::as_array).into_iter().flatten().collect::<Vec<_>>();
        let owned = previous_peers.iter().filter_map(|peer|peer.get("entryId").and_then(Value::as_str)).collect::<HashSet<_>>();
        if let Some(peers) = document.get_mut("peers").and_then(Value::as_array_mut) {
            for peer in peers {
                let id = peer_id(peer).unwrap_or_default().to_owned();
                let entry = peer.get("entryId").and_then(Value::as_str).unwrap_or_default();
                let matching = previous_peers.iter().find(|previous|peer_id(previous)==Some(id.as_str()))
                    .and_then(|peer|peer.get("entryId").and_then(Value::as_str));
                let entry = if !explicit_ids.contains(&id) { matching.map(str::to_owned).unwrap_or_else(||uuid::Uuid::new_v4().to_string()) }
                    else if owned.contains(entry) { entry.to_owned() } else { uuid::Uuid::new_v4().to_string() };
                peer["entryId"] = json!(entry);
                if peer.get("guid").is_some() { peer["guid"] = peer["entryId"].clone(); }
            }
        }
        canonical(&mut document)?;
        let data = serde_json::to_string(&document).map_err(|_|BookError::Storage)?;
        if data.len()>2*1024*1024 { return Err(invalid("address_book_too_large")); }
        let revision = revision.checked_add(1).ok_or(BookError::Storage)?;
        sqlx::query("update api_address_book_snapshot set data=?,revision=?,updated_at=current_timestamp where user_id=?")
            .bind(data).bind(revision).bind(user).execute(&mut tx).await.map_err(|_|BookError::Storage)?;
        rebuild_index(&mut tx,user,&document).await.map_err(|_|BookError::Storage)?;
        tx.commit().await.map_err(|_|BookError::Storage)?;
        Ok(Book { guid,revision,document })
    }
    pub async fn rebuild(&self, user: &str) -> Result<(),BookError> {
        let mut conn = self.db.book_connection().await.map_err(|_|BookError::Storage)?;
        let mut tx = conn.begin().await.map_err(|_|BookError::Storage)?;
        sqlx::query("update api_schema_lock set id=id where id=1").execute(&mut tx).await.map_err(|_|BookError::Storage)?;
        let data: String = sqlx::query_scalar("select data from api_address_book_snapshot where user_id=?").bind(user).fetch_one(&mut tx).await.map_err(|_|BookError::Storage)?;
        let mut document = parse(&data)?; let original = document.clone(); canonical(&mut document)?;
        if document!=original { return Err(invalid("address_book_requires_repair")); }
        rebuild_index(&mut tx,user,&document).await.map_err(|_|BookError::Storage)?;
        tx.commit().await.map_err(|_|BookError::Storage)?;
        Ok(())
    }
}

fn invalid(message: &str) -> BookError { BookError::Invalid(message.to_owned()) }
fn parse(data: &str) -> Result<Map<String,Value>,BookError> {
    match serde_json::from_str(data) { Ok(Value::Object(document)) => Ok(document), _ => Err(invalid("invalid_address_book_document")) }
}
fn peer_id(peer: &Value) -> Option<&str> { peer.get("peerId").or_else(||peer.get("peer_id")).or_else(||peer.get("id")).and_then(Value::as_str) }
fn validate_aliases(peer: &Value, keys: &[&str]) -> Result<(),BookError> {
    let mut selected = None;
    for key in keys {
        if let Some(value) = peer.get(*key) {
            let value = value.as_str().ok_or_else(||invalid("invalid_identity_alias"))?;
            if selected.is_some_and(|previous|previous!=value) { return Err(invalid("identity_alias_conflict")); }
            selected = Some(value);
        }
    }
    Ok(())
}
fn canonical(document: &mut Map<String,Value>) -> Result<(),BookError> {
    address_book_codec::normalize_relays(document).map_err(invalid)?;
    address_book_codec::normalize_colors(document).map_err(invalid)?;
    let tags = document.entry("tags".to_owned()).or_insert_with(||json!([]));
    if !tags.as_array().is_some_and(|tags|tags.iter().all(|tag|tag.as_str().or_else(||tag.get("name").and_then(Value::as_str)).is_some())) { return Err(invalid("invalid_address_book_tags")); }
    let peers = document.entry("peers".to_owned()).or_insert_with(||json!([])).as_array_mut().ok_or_else(||invalid("invalid_address_book_peers"))?;
    let mut ids = HashSet::new(); let mut entries = HashSet::new();
    for (position,peer) in peers.iter_mut().enumerate() {
        validate_aliases(peer,&["id","peerId","peer_id"])?;
        validate_aliases(peer,&["entryId","guid"])?;
        let id = peer_id(peer).filter(|id|!id.is_empty() && id.trim()==*id && id.chars().count()<=128).ok_or_else(||BookError::Invalid(format!("invalid_peer_id_at_index_{position}")))?.to_owned();
        if !ids.insert(id.clone()) { return Err(BookError::Invalid(format!("duplicate_peer_id:{id}"))); }
        let fields = peer.as_object_mut().ok_or_else(||invalid("invalid_peer_object"))?;
        let entry = fields.get("entryId").or_else(||fields.get("guid")).map(|value|value.as_str().map(str::to_owned).ok_or_else(||invalid("invalid_entry_id"))).transpose()?.unwrap_or_else(||uuid::Uuid::new_v4().to_string());
        if entry.is_empty() || entry.chars().count()>128 || !entries.insert(entry.clone()) { return Err(BookError::Invalid(format!("duplicate_or_invalid_entry_id:{entry}"))); }
        for key in ["username","hostname","alias","platform"] {
            if fields.get(key).is_some_and(|value|!value.is_string()) { return Err(BookError::Invalid(format!("invalid_peer_field:{id}:{key}"))); }
        }
        for key in ["createdAt","updatedAt"] {
            if let Some(value) = fields.get(key) {
                let value = value.as_str().ok_or_else(||invalid("invalid_address_book_timestamp"))?;
                if !value.is_empty() { timestamp(value)?; }
            }
        }
        if fields.get("tags").is_some_and(|value|!value.as_array().is_some_and(|tags|tags.iter().all(|tag|tag.as_str().or_else(||tag.get("name").and_then(Value::as_str)).is_some()))) { return Err(BookError::Invalid(format!("invalid_peer_tags:{id}"))); }
        fields.insert("id".to_owned(),json!(id)); fields.insert("peerId".to_owned(),json!(id)); fields.insert("entryId".to_owned(),json!(entry));
    }
    Ok(())
}

async fn rebuild_index(tx: &mut Transaction<'_,Sqlite>, user: &str, document: &Map<String,Value>) -> ResultType<()> {
    sqlx::query("delete from api_address_book_entry where user_id=?").bind(user).execute(&mut *tx).await?;
    let peers = document.get("peers").and_then(Value::as_array).ok_or_else(||anyhow::anyhow!("invalid_address_book_peers"))?;
    for peer in peers {
        let tags = peer.get("tags").and_then(Value::as_array).into_iter().flatten().filter_map(|tag|tag.as_str().or_else(||tag.get("name").and_then(Value::as_str))).collect::<Vec<_>>();
        let text = |key|peer.get(key).and_then(Value::as_str).unwrap_or_default();
        sqlx::query("insert into api_address_book_entry(id,user_id,peer_id,username,hostname,alias,platform,tags,force_always_relay,created_at,updated_at) values(?,?,?,?,?,?,?,?,?,coalesce(nullif(?,''),current_timestamp),coalesce(nullif(?,''),current_timestamp))")
            .bind(text("entryId")).bind(user).bind(text("peerId")).bind(text("username")).bind(text("hostname"))
            .bind(text("alias")).bind(text("platform")).bind(serde_json::to_string(&tags)?)
            .bind(i64::from(peer.get("forceAlwaysRelay").and_then(Value::as_bool).unwrap_or(false)))
            .bind(text("createdAt")).bind(text("updatedAt")).execute(&mut *tx).await?;
    }
    Ok(())
}

pub(crate) async fn migrate(tx: &mut Transaction<'_,Sqlite>) -> ResultType<()> {
    sqlx::query("alter table api_address_book_snapshot add column guid text;
        alter table api_address_book_snapshot add column revision integer not null default 0;
        create unique index api_address_book_guid on api_address_book_snapshot(guid)").execute(&mut *tx).await?;
    let users: Vec<String> = sqlx::query_scalar("select id from api_user order by id").fetch_all(&mut *tx).await?;
    for user in users { initialize(tx,&user).await.map_err(|error|anyhow::anyhow!("Address book migration account {user}: {error}"))?; }
    sqlx::query("insert into api_schema_migration(version,name) values(3,'authoritative_personal_address_books')").execute(&mut *tx).await?;
    Ok(())
}

async fn initialize(tx: &mut Transaction<'_,Sqlite>, user: &str) -> ResultType<Book> {
    let row: Option<(String,Option<String>,i64,String)> = sqlx::query_as("select data,guid,revision,updated_at from api_address_book_snapshot where user_id=?").bind(user).fetch_optional(&mut *tx).await?;
    let (mut document,snapshot_time) = if let Some((data,_,_,time)) = &row { (parse(data)?,time.as_str()) } else { (Map::new(),"") };
    if let Some((_,Some(guid),revision,_)) = row.as_ref() { if uuid::Uuid::parse_str(guid).is_err() || *revision<1 { return Err(invalid("invalid_address_book_metadata").into()); } canonical(&mut document)?; return Ok(Book { guid:guid.clone(),revision:*revision,document }); }
    let mut validated = document.clone(); canonical(&mut validated)?;
    address_book_codec::normalize_relays(&mut document).map_err(invalid)?;
    let snapshot_time = if snapshot_time.is_empty() { None } else { Some(timestamp(snapshot_time)?) };
    let indexed: Vec<ApiAddressBookEntry> = sqlx::query_as("select id,user_id,peer_id,username,hostname,alias,platform,tags,force_always_relay,created_at,updated_at from api_address_book_entry where user_id=? order by id").bind(user).fetch_all(&mut *tx).await?;
    let peers = document.entry("peers".to_owned()).or_insert_with(||json!([])).as_array_mut().ok_or_else(||invalid("invalid_address_book_peers"))?;
    for entry in indexed {
        let index_time = timestamp(&entry.updated_at)?;
        timestamp(&entry.created_at)?;
        let tags: Value = serde_json::from_str(&entry.tags)?;
        if !matches!(entry.force_always_relay,0|1) { return Err(invalid("invalid_index_relay").into()); }
        let known = json!({"id":entry.peer_id,"peerId":entry.peer_id,"entryId":entry.id,"username":entry.username,"hostname":entry.hostname,"alias":entry.alias,"platform":entry.platform,"tags":tags,"forceAlwaysRelay":entry.force_always_relay!=0,"createdAt":entry.created_at,"updatedAt":entry.updated_at});
        if let Some(existing) = peers.iter_mut().find(|peer|peer_id(peer)==Some(entry.peer_id.as_str())) {
            if existing.get("entryId").or_else(||existing.get("guid")).and_then(Value::as_str).is_some_and(|id|id!=entry.id) { return Err(anyhow::anyhow!("entry_identity_conflict:{}",entry.peer_id)); }
            let fields = existing.as_object_mut().ok_or_else(||invalid("invalid_peer_object"))?;
            if snapshot_time.map_or(true,|time|index_time>time) {
                if let Some(known) = known.as_object() { fields.extend(known.clone()); }
                if fields.contains_key("peer_id") { fields.insert("peer_id".to_owned(),json!(entry.peer_id)); }
            }
            fields.insert("entryId".to_owned(),json!(entry.id));
        } else { peers.push(known); }
    }
    canonical(&mut document)?;
    let guid = uuid::Uuid::new_v4().to_string(); let revision = 1;
    sqlx::query("insert into api_address_book_snapshot(user_id,data,guid,revision) values(?,?,?,?) on conflict(user_id) do update set data=excluded.data,guid=excluded.guid,revision=excluded.revision")
        .bind(user).bind(serde_json::to_string(&document)?).bind(&guid).bind(revision).execute(&mut *tx).await?;
    rebuild_index(tx,user,&document).await?;
    Ok(Book { guid,revision,document })
}

fn timestamp(value: &str) -> Result<chrono::NaiveDateTime,BookError> {
    chrono::NaiveDateTime::parse_from_str(value,"%Y-%m-%d %H:%M:%S%.f")
        .or_else(|_|chrono::DateTime::parse_from_rfc3339(value).map(|time|time.naive_utc()))
        .map_err(|_|invalid("invalid_address_book_timestamp"))
}
