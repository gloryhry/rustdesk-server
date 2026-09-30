use super::*;
use serde_json::{Map,Value};

// RustDesk 1.4.9 uses book GUIDs here; Web entry IDs use separate routes.
async fn owned(state: &ApiState, headers: &HeaderMap, guid: &str) -> Result<(String,Book),Response> {
    let principal = authorize(state,headers).await.map_err(|error|auth_error_response(error,true))?;
    let book = address_books(state).load(&principal.user_id).await.map_err(book_error_response)?;
    if book.guid!=guid { return Err(not_found("address_book_not_found")); }
    Ok((principal.user_id,book))
}
fn not_found(message: &str) -> Response { (StatusCode::NOT_FOUND,Json(json!({"error":message}))).into_response() }
fn conflict(message: &str) -> Response { (StatusCode::CONFLICT,Json(json!({"error":message}))).into_response() }
fn payload_revision(headers: &HeaderMap, payload: &Value, current: i64) -> Result<i64,Response> {
    let revision = payload.get("revision").map(|value|value.as_i64().ok_or_else(||invalid_list_query("invalid_address_book_revision"))).transpose()?;
    book_revision(headers,revision,current).map_err(book_error_response)
}
async fn save(state: &ApiState, user: &str, revision: i64, document: Map<String,Value>) -> Response {
    match address_books(state).replace(user,Some(revision),document).await {
        Ok(_) => StatusCode::OK.into_response(), Err(error) => book_error_response(error),
    }
}

pub(super) async fn personal(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap) -> Response {
    let principal = match authorize(&state,&headers).await { Ok(principal) => principal, Err(error) => return auth_error_response(error,true) };
    match address_books(&state).load(&principal.user_id).await {
        Ok(book) => Json(json!({"guid":book.guid})).into_response(), Err(error) => book_error_response(error),
    }
}
pub(super) async fn settings(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap) -> Response {
    if let Err(error) = authorize(&state,&headers).await { return auth_error_response(error,true); }
    // No per-book Peer count limit or shared books are configured in this service.
    Json(json!({"max_peer_one_ab":0})).into_response()
}
pub(super) async fn shared_profiles(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Query(query): Query<crate::pagination::ListQuery>) -> Response {
    if let Err(error) = authorize(&state,&headers).await { return auth_error_response(error,true); }
    if let Err(error) = query.parse() { return invalid_list_query(error); }
    Json(json!({"total":0,"data":[]})).into_response()
}
#[derive(Deserialize)]
pub(super) struct PeerQuery {
    ab: String,
    #[serde(flatten)]
    page: crate::pagination::ListQuery,
}
pub(super) async fn peers(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Query(query): Query<PeerQuery>) -> Response {
    let (_,book) = match owned(&state,&headers,&query.ab).await { Ok(book) => book, Err(response) => return response };
    let page = match query.page.parse() { Ok(page) => page, Err(error) => return invalid_list_query(error) };
    if page.status.is_some() { return invalid_list_query("unsupported_status_filter"); }
    let mut document = book.document; crate::address_book_codec::official_relays(&mut document);
    let mut peers = document.get("peers").and_then(Value::as_array).cloned().unwrap_or_default();
    peers.retain(|peer|page.name.as_ref().map_or(true,|name|["id","alias","username","hostname"].iter().any(|key|peer.get(*key).and_then(Value::as_str).is_some_and(|value|value.contains(name)))));
    peers.sort_by(|left,right|snapshot_peer_id(left).cmp(&snapshot_peer_id(right)));
    let total = peers.len();
    let offset = usize::try_from(page.offset).unwrap_or(usize::MAX);
    let mut data = peers.into_iter().skip(offset).take(page.limit as usize).collect::<Vec<_>>();
    for peer in &mut data {
        if let Some(fields) = peer.as_object_mut() { for key in ["entryId","guid","peerId","peer_id"] { fields.remove(key); } }
    }
    Json(json!({"total":total,"data":data})).into_response()
}
pub(super) async fn tags(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Path(guid): Path<String>) -> Response {
    let (_,book) = match owned(&state,&headers,&guid).await { Ok(book) => book, Err(response) => return response };
    Json(crate::address_book_codec::official_tag_values(&book.document)).into_response()
}

fn peer_payload(payload: &Value) -> Result<(String,Map<String,Value>),Response> {
    let mut fields = payload.as_object().cloned().ok_or_else(||invalid_list_query("invalid_peer_object"))?;
    let id = fields.get("id").and_then(Value::as_str).filter(|id|!id.is_empty() && id.trim()==*id && id.chars().count()<=128)
        .ok_or_else(||invalid_list_query("invalid_peer_id"))?.to_owned();
    if fields.keys().any(|key|matches!(key.as_str(),"entryId"|"guid"|"peerId"|"peer_id"|"user_id"|"userId"|"createdAt"|"updatedAt")) { return Err(invalid_list_query("reserved_address_book_field")); }
    fields.remove("revision");
    Ok((id,fields))
}
pub(super) async fn add_peer(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Path(guid): Path<String>, Json(payload): Json<Value>) -> Response {
    let _guard = state.tag_lock.lock().await;
    let (user,mut book) = match owned(&state,&headers,&guid).await { Ok(book) => book, Err(response) => return response };
    let revision = match payload_revision(&headers,&payload,book.revision) { Ok(revision) => revision, Err(response) => return response };
    let (id,mut fields) = match peer_payload(&payload) { Ok(fields) => fields, Err(response) => return response };
    let peers = match book.document.get_mut("peers").and_then(Value::as_array_mut) { Some(peers) => peers, None => return invalid_list_query("invalid_address_book_peers") };
    if peers.iter().any(|peer|snapshot_peer_id(peer)==Some(id.as_str())) { return conflict("address_book_entry_conflict"); }
    let now = chrono::Utc::now().to_rfc3339(); fields.insert("createdAt".to_owned(),json!(now)); fields.insert("updatedAt".to_owned(),json!(now));
    peers.push(Value::Object(fields)); save(&state,&user,revision,book.document).await
}
pub(super) async fn update_peer(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Path(guid): Path<String>, Json(payload): Json<Value>) -> Response {
    let _guard = state.tag_lock.lock().await;
    let (user,mut book) = match owned(&state,&headers,&guid).await { Ok(book) => book, Err(response) => return response };
    let revision = match payload_revision(&headers,&payload,book.revision) { Ok(revision) => revision, Err(response) => return response };
    let (id,fields) = match peer_payload(&payload) { Ok(fields) => fields, Err(response) => return response };
    let peer = book.document.get_mut("peers").and_then(Value::as_array_mut).and_then(|peers|peers.iter_mut().find(|peer|snapshot_peer_id(peer)==Some(id.as_str()))).and_then(Value::as_object_mut);
    let peer = match peer { Some(peer) => peer, None => return not_found("address_book_entry_not_found") };
    peer.extend(fields); peer.insert("updatedAt".to_owned(),json!(chrono::Utc::now().to_rfc3339()));
    save(&state,&user,revision,book.document).await
}
fn identifiers(payload: Value, max_length: usize) -> Result<Vec<String>,Response> {
    let values = payload.as_array().ok_or_else(||invalid_list_query("expected_identifier_array"))?;
    values.iter().map(|value|value.as_str().filter(|id|!id.is_empty() && id.trim()==*id && id.chars().count()<=max_length)
        .map(str::to_owned).ok_or_else(||invalid_list_query("invalid_identifier"))).collect()
}
pub(super) async fn delete_peers(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Path(guid): Path<String>, Query(query): Query<BookRevision>, Json(payload): Json<Value>) -> Response {
    let _guard = state.tag_lock.lock().await;
    let (user,mut book) = match owned(&state,&headers,&guid).await { Ok(book) => book, Err(response) => return response };
    let revision = match book_revision(&headers,query.revision,book.revision) { Ok(revision) => revision, Err(error) => return book_error_response(error) };
    let ids = match identifiers(payload,128) { Ok(ids) => ids, Err(response) => return response };
    if let Some(peers) = book.document.get_mut("peers").and_then(Value::as_array_mut) { peers.retain(|peer|!snapshot_peer_id(peer).is_some_and(|id|ids.iter().any(|target|target==id))); }
    save(&state,&user,revision,book.document).await
}

fn name(value: Option<&Value>) -> Result<String,Response> {
    value.and_then(Value::as_str).filter(|name|!name.is_empty() && name.trim()==*name && name.chars().count()<=64)
        .map(str::to_owned).ok_or_else(||invalid_list_query("invalid_tag"))
}
fn color(payload: &Value) -> Result<u32,Response> {
    payload.get("color").and_then(Value::as_u64).and_then(|color|u32::try_from(color).ok()).ok_or_else(||invalid_list_query("invalid_tag_color"))
}
async fn write_tag(state: &ApiState, headers: &HeaderMap, guid: &str, payload: Value, create: bool) -> Response {
    let _guard = state.tag_lock.lock().await;
    let (user,mut book) = match owned(state,headers,guid).await { Ok(book) => book, Err(response) => return response };
    let revision = match payload_revision(headers,&payload,book.revision) { Ok(revision) => revision, Err(response) => return response };
    let name = match name(payload.get("name")) { Ok(name) => name, Err(response) => return response };
    let color = match color(&payload) { Ok(color) => color, Err(response) => return response };
    let tags = match book.document.get_mut("tags").and_then(Value::as_array_mut) { Some(tags) => tags, None => return invalid_list_query("invalid_address_book_tags") };
    let existing = tags.iter().find_map(|tag|tag_name(tag).filter(|tag|tag.eq_ignore_ascii_case(&name))).map(str::to_owned);
    let key = match (create,existing) {
        (true,Some(_)) => return conflict("tag_name_conflict"),
        (false,None) => return not_found("tag_not_found"),
        (true,None) => { tags.push(json!(name)); name }, (false,Some(existing)) => existing,
    };
    book.document["tag_colors"][&key] = json!(color); save(state,&user,revision,book.document).await
}
pub(super) async fn add_tag(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Path(guid): Path<String>, Json(payload): Json<Value>) -> Response {
    write_tag(&state,&headers,&guid,payload,true).await
}
pub(super) async fn update_tag(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Path(guid): Path<String>, Json(payload): Json<Value>) -> Response {
    write_tag(&state,&headers,&guid,payload,false).await
}
pub(super) async fn rename_tag(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Path(guid): Path<String>, Json(payload): Json<Value>) -> Response {
    let _guard = state.tag_lock.lock().await;
    let (user,mut book) = match owned(&state,&headers,&guid).await { Ok(book) => book, Err(response) => return response };
    let revision = match payload_revision(&headers,&payload,book.revision) { Ok(revision) => revision, Err(response) => return response };
    let (old,new) = match (name(payload.get("old")),name(payload.get("new"))) { (Ok(old),Ok(new)) => (old,new), _ => return invalid_list_query("invalid_tag") };
    let tags = match book.document.get_mut("tags").and_then(Value::as_array_mut) { Some(tags) => tags, None => return invalid_list_query("invalid_address_book_tags") };
    let position = match tags.iter().position(|tag|tag_name(tag).is_some_and(|tag|tag.eq_ignore_ascii_case(&old))) { Some(position) => position, None => return not_found("tag_not_found") };
    if tags.iter().enumerate().any(|(i,tag)|i!=position && tag_name(tag).is_some_and(|tag|tag.eq_ignore_ascii_case(&new))) { return conflict("tag_name_conflict"); }
    let old = tag_name(&tags[position]).unwrap_or_default().to_owned();
    if tags[position].is_object() { tags[position]["name"] = json!(new); } else { tags[position] = json!(new); }
    if let Some(colors) = book.document.get_mut("tag_colors").and_then(Value::as_object_mut) { if let Some(color) = colors.remove(&old) { colors.insert(new.clone(),color); } }
    if let Err(error) = crate::address_book_codec::rewrite_tag_refs(&mut book.document,&old,Some(&new)) { return invalid_list_query(error); }
    save(&state,&user,revision,book.document).await
}
pub(super) async fn delete_tags(Extension(state): Extension<Arc<ApiState>>, headers: HeaderMap, Path(guid): Path<String>, Query(query): Query<BookRevision>, Json(payload): Json<Value>) -> Response {
    let _guard = state.tag_lock.lock().await;
    let (user,mut book) = match owned(&state,&headers,&guid).await { Ok(book) => book, Err(response) => return response };
    let revision = match book_revision(&headers,query.revision,book.revision) { Ok(revision) => revision, Err(error) => return book_error_response(error) };
    let names = match identifiers(payload,64) { Ok(names) => names, Err(response) => return response };
    for name in names {
        if let Some(tags) = book.document.get_mut("tags").and_then(Value::as_array_mut) { tags.retain(|tag|!tag_name(tag).is_some_and(|tag|tag.eq_ignore_ascii_case(&name))); }
        if let Some(colors) = book.document.get_mut("tag_colors").and_then(Value::as_object_mut) { colors.retain(|key,_|!key.eq_ignore_ascii_case(&name)); }
        if let Err(error) = crate::address_book_codec::rewrite_tag_refs(&mut book.document,&name,None) { return invalid_list_query(error); }
    }
    save(&state,&user,revision,book.document).await
}
