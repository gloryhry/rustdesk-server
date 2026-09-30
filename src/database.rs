use async_trait::async_trait;
use hbb_common::{log, ResultType};
use sqlx::{
    sqlite::SqliteConnectOptions, ConnectOptions, Connection, Error as SqlxError, SqliteConnection,
};
use std::{ops::DerefMut, str::FromStr, time::Duration};
//use sqlx::postgres::PgPoolOptions;
//use sqlx::mysql::MySqlPoolOptions;

type Pool = deadpool::managed::Pool<DbPool>;

pub struct DbPool {
    url: String,
}

#[async_trait]
impl deadpool::managed::Manager for DbPool {
    type Type = SqliteConnection;
    type Error = SqlxError;
    async fn create(&self) -> Result<SqliteConnection, SqlxError> {
        let mut opt = SqliteConnectOptions::from_str(&self.url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));
        opt.log_statements(log::LevelFilter::Debug);
        SqliteConnection::connect_with(&opt).await
    }
    async fn recycle(
        &self,
        obj: &mut SqliteConnection,
    ) -> deadpool::managed::RecycleResult<SqlxError> {
        Ok(obj.ping().await?)
    }
}

#[derive(Clone)]
pub struct Database {
    pool: Pool,
}

#[derive(Default)]
pub struct Peer {
    pub guid: Vec<u8>,
    pub id: String,
    pub uuid: Vec<u8>,
    pub pk: Vec<u8>,
    pub user: Option<Vec<u8>>,
    pub info: String,
    pub status: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiUser {
    pub id: String,
    pub username: String,
    pub email: String,
    pub nickname: String,
    pub avatar: String,
    pub password_hash: String,
    pub is_admin: i64,
    pub status: i64,
    pub token_version: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ApiUserGroup {
    pub id: String,
    pub name: String,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ApiUserGroupMember {
    pub group_id: String,
    pub user_id: String,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ApiDeviceGroup {
    pub id: String,
    pub name: String,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ApiDeviceGroupMember {
    pub group_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ApiDevice {
    pub id: String,
    pub user_id: String,
    pub uuid: String,
    pub name: String,
    pub os: String,
    pub device_type: String,
    pub info: String,
    pub status: i64,
    pub last_seen_at: String,
    pub created_at: String,
    pub updated_at: String,
    pub peer_id: String,
    pub verified: bool,
    pub registered_at_ms: i64,
    pub online: bool,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ApiAddressBookEntry {
    pub id: String,
    pub user_id: String,
    pub peer_id: String,
    pub username: String,
    pub hostname: String,
    pub alias: String,
    pub platform: String,
    pub tags: String,
    pub force_always_relay: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ApiSessionView {
    pub id: String,
    pub user_id: String,
    pub username: String,
    pub device_id: String,
    pub device_uuid: String,
    pub device_name: String,
    pub device_os: String,
    pub device_type: String,
    pub expires_at: i64,
    pub revoked_at: Option<i64>,
    pub created_at: String,
    pub last_used_at: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiIdentity {
    pub provider: String,
    pub subject: String,
    pub user_id: String,
}

impl Database {
    pub async fn new(url: &str) -> ResultType<Database> {
        let n: usize = crate::common::get_arg_or("MAX_DATABASE_CONNECTIONS", "1".to_owned())
            .parse()
            .unwrap_or(1);
        log::debug!("MAX_DATABASE_CONNECTIONS={}", n);
        let pool = Pool::new(
            DbPool {
                url: url.to_owned(),
            },
            n,
        );
        let _ = pool.get().await?; // test
        let db = Database { pool };
        db.create_tables().await?;
        Ok(db)
    }

    async fn create_tables(&self) -> ResultType<()> {
        sqlx::query!(
            "
            create table if not exists peer (
                guid blob primary key not null,
                id varchar(100) not null,
                uuid blob not null,
                pk blob not null,
                created_at datetime not null default(current_timestamp),
                user blob,
                status tinyint,
                note varchar(300),
                info text not null
            ) without rowid;
            create unique index if not exists index_peer_id on peer (id);
            create index if not exists index_peer_user on peer (user);
            create index if not exists index_peer_created_at on peer (created_at);
            create index if not exists index_peer_status on peer (status);
        "
        )
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        let mut conn = self.pool.get().await?;
        sqlx::query(
            "
            create table if not exists api_user (
                id text primary key not null,
                username text not null collate nocase unique,
                email text not null default '',
                nickname text not null default '',
                avatar text not null default '',
                password_hash text not null,
                is_admin integer not null default 0,
                status integer not null default 1,
                token_version integer not null default 0,
                created_at datetime not null default(current_timestamp),
                updated_at datetime not null default(current_timestamp)
            )
            "
        )
        .execute(conn.deref_mut())
        .await?;
        sqlx::query("create index if not exists index_api_user_status on api_user (status)")
            .execute(conn.deref_mut())
            .await?;
        sqlx::query("create index if not exists index_api_user_created_at on api_user (created_at)")
            .execute(conn.deref_mut())
            .await?;
        sqlx::query(
            "
            create table if not exists api_identity (
                provider text not null,
                subject text not null,
                user_id text not null,
                created_at datetime not null default(current_timestamp),
                primary key(provider, subject),
                foreign key(user_id) references api_user(id) on delete cascade
            )
            "
        )
        .execute(conn.deref_mut())
        .await?;
        sqlx::query("create index if not exists index_api_identity_user on api_identity (user_id)")
            .execute(conn.deref_mut())
            .await?;
        sqlx::query(
            "
            create table if not exists api_session (
                id text primary key not null,
                user_id text not null,
                device_id text not null default '',
                device_uuid text not null default '',
                device_name text not null default '',
                device_os text not null default '',
                device_type text not null default '',
                expires_at integer not null,
                revoked_at integer,
                created_at datetime not null default(current_timestamp),
                last_used_at datetime not null default(current_timestamp),
                foreign key(user_id) references api_user(id) on delete cascade
            )
            "
        )
        .execute(conn.deref_mut())
        .await?;
        sqlx::query("create index if not exists index_api_session_user on api_session (user_id)")
            .execute(conn.deref_mut())
            .await?;
        sqlx::query(
            "
            create table if not exists api_device (
                id text primary key not null,
                user_id text not null,
                uuid text not null default '',
                name text not null default '',
                os text not null default '',
                device_type text not null default '',
                info text not null default '',
                status integer not null default 1,
                last_seen_at datetime not null default(current_timestamp),
                created_at datetime not null default(current_timestamp),
                updated_at datetime not null default(current_timestamp),
                unique(user_id, uuid),
                foreign key(user_id) references api_user(id) on delete cascade
            )
            "
        )
        .execute(conn.deref_mut())
        .await?;
        sqlx::query("create index if not exists index_api_device_user on api_device (user_id)")
            .execute(conn.deref_mut())
            .await?;
        sqlx::query(
            "
            create table if not exists api_address_book_snapshot (
                user_id text primary key not null,
                data text not null,
                updated_at datetime not null default(current_timestamp),
                foreign key(user_id) references api_user(id) on delete cascade
            )
            "
        )
        .execute(conn.deref_mut())
        .await?;
        sqlx::query(
            "
            create table if not exists api_user_group (
                id text primary key not null,
                name text not null,
                created_by text not null,
                created_at datetime not null default(current_timestamp),
                unique(created_by, name),
                foreign key(created_by) references api_user(id) on delete cascade
            )
            "
        )
        .execute(conn.deref_mut())
        .await?;
        sqlx::query(
            "
            create table if not exists api_user_group_member (
                group_id text not null,
                user_id text not null,
                created_at datetime not null default(current_timestamp),
                primary key(group_id, user_id),
                foreign key(group_id) references api_user_group(id) on delete cascade,
                foreign key(user_id) references api_user(id) on delete cascade
            )
            "
        )
        .execute(conn.deref_mut())
        .await?;
        sqlx::query(
            "
            create table if not exists api_device_group (
                id text primary key not null,
                name text not null,
                created_by text not null,
                created_at datetime not null default(current_timestamp),
                unique(created_by, name),
                foreign key(created_by) references api_user(id) on delete cascade
            )
            "
        )
        .execute(conn.deref_mut())
        .await?;
        // Legacy peer-GUID grouping storage. API-account device memberships use
        // api_device_group_device because api_device identifiers are text IDs.
        sqlx::query(
            "
            create table if not exists api_device_group_member (
                group_id text not null,
                peer_guid blob not null,
                created_at datetime not null default(current_timestamp),
                primary key(group_id, peer_guid),
                foreign key(group_id) references api_device_group(id) on delete cascade
            )
            "
        )
        .execute(conn.deref_mut())
        .await?;
        sqlx::query(
            "
            create table if not exists api_device_group_device (
                group_id text not null,
                device_id text not null,
                created_at datetime not null default(current_timestamp),
                primary key(group_id, device_id),
                foreign key(group_id) references api_device_group(id) on delete cascade,
                foreign key(device_id) references api_device(id) on update cascade on delete cascade
            )
            "
        )
        .execute(conn.deref_mut())
        .await?;
        sqlx::query(
            "
            create table if not exists api_address_book_entry (
                id text primary key not null,
                user_id text not null,
                peer_id text not null default '',
                username text not null default '',
                hostname text not null default '',
                alias text not null default '',
                platform text not null default '',
                tags text not null default '[]',
                force_always_relay integer not null default 0,
                created_at datetime not null default(current_timestamp),
                updated_at datetime not null default(current_timestamp),
                unique(user_id, peer_id),
                foreign key(user_id) references api_user(id) on delete cascade
            )
            "
        )
        .execute(conn.deref_mut())
        .await?;
        drop(conn);
        self.apply_api_migrations().await?;
        Ok(())
    }

    async fn apply_api_migrations(&self) -> ResultType<()> {
        let deadline = hbb_common::tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            match self.apply_api_migrations_once().await {
                Ok(()) => return Ok(()),
                Err(error) => {
                    let busy = error.downcast_ref::<SqlxError>().and_then(|error| match error {
                        SqlxError::Database(error) => error.code(),
                        _ => None,
                    }).and_then(|code| code.parse::<u32>().ok()).is_some_and(|code| matches!(code & 255, 5 | 6));
                    // Deferred SQLite transactions can lose a read-to-write upgrade race.
                    // The failed transaction has rolled back; retry the entire version check.
                    if !busy || hbb_common::tokio::time::Instant::now() >= deadline { return Err(error); }
                    hbb_common::tokio::time::sleep(Duration::from_millis(20)).await;
                }
            }
        }
    }

    async fn apply_api_migrations_once(&self) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        let mut tx = conn.begin().await?;
        sqlx::query("create table if not exists api_schema_lock (id integer primary key)").execute(&mut tx).await?;
        sqlx::query("insert or ignore into api_schema_lock(id) values(1)").execute(&mut tx).await?;
        // Acquire SQLite's writer lock before reading migration versions. The transaction rolls back on cancellation.
        sqlx::query("update api_schema_lock set id = id where id = 1").execute(&mut tx).await?;
        sqlx::query("create table if not exists api_schema_migration(version integer primary key, name text not null, applied_at text not null default current_timestamp)").execute(&mut tx).await?;
        let applied: Option<i64> = sqlx::query_scalar("select version from api_schema_migration where version = 1").fetch_optional(&mut tx).await?;
        if applied.is_none() {
            sqlx::query("create table api_oauth_provider (
                id text primary key, name text not null unique, namespace text not null unique,
                authority text not null, source text not null, config text not null,
                encrypted_secret text not null, enabled integer not null, deleted integer not null,
                revision text not null)").execute(&mut tx).await?;
            // Reserve historical identity names so a new database provider cannot take over old identities.
            sqlx::query("insert into api_oauth_provider(id,name,namespace,authority,source,config,encrypted_secret,enabled,deleted,revision)
                select 'legacy:' || provider,provider,provider,'','legacy','{}','',0,1,'legacy' from api_identity group by provider").execute(&mut tx).await?;
            sqlx::query("insert into api_schema_migration(version,name) values(1,'oauth_provider_registry')").execute(&mut tx).await?;
        }
        let applied: Option<i64> = sqlx::query_scalar("select version from api_schema_migration where version = 2").fetch_optional(&mut tx).await?;
        if applied.is_none() {
            sqlx::query("alter table api_device add column peer_guid blob;
                alter table api_device add column verified integer not null default 0;
                alter table api_device add column verified_uuid blob;
                alter table api_device add column verified_pk blob;
                create unique index api_device_verified_peer on api_device(peer_guid) where verified=1;
                create table peer_registration(peer_guid blob primary key, uuid blob not null, pk blob not null, registered_at_ms integer not null,
                    foreign key(peer_guid) references peer(guid) on delete cascade);
                create table api_device_report(peer_guid blob primary key, uuid blob not null, pk blob not null,
                    sysinfo text, sysinfo_at_ms integer not null default 0, heartbeat text, heartbeat_at_ms integer not null default 0,
                    foreign key(peer_guid) references peer(guid) on delete cascade);
                create table api_device_binding_audit(id text primary key, actor_id text not null, action text not null,
                    device_id text not null, peer_id text not null, owner_id text not null, pk_fingerprint text not null,
                    recorded_at_ms integer not null);
                insert into api_schema_migration(version,name) values(2,'untrusted_device_reports_and_verified_bindings')")
                .execute(&mut tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn oauth_provider_records(&self) -> ResultType<Vec<crate::oauth_admin::ProviderRecord>> {
        Ok(sqlx::query_as("select id,name,namespace,authority,source,config,encrypted_secret,enabled,deleted,revision from api_oauth_provider order by name")
            .fetch_all(self.pool.get().await?.deref_mut()).await?)
    }

    pub(crate) async fn insert_oauth_provider(&self, row: &crate::oauth_admin::ProviderRecord) -> ResultType<()> {
        sqlx::query("insert into api_oauth_provider(id,name,namespace,authority,source,config,encrypted_secret,enabled,deleted,revision) values(?,?,?,?,?,?,?,?,?,?)")
            .bind(&row.id).bind(&row.name).bind(&row.namespace).bind(&row.authority).bind(&row.source)
            .bind(&row.config).bind(&row.encrypted_secret).bind(row.enabled).bind(row.deleted).bind(&row.revision)
            .execute(self.pool.get().await?.deref_mut()).await?;
        Ok(())
    }

    pub(crate) async fn update_oauth_provider(&self, row: &crate::oauth_admin::ProviderRecord, previous: &str) -> ResultType<bool> {
        Ok(sqlx::query("update api_oauth_provider set config=?, encrypted_secret=?, enabled=?, deleted=?, revision=? where id=? and revision=?")
            .bind(&row.config).bind(&row.encrypted_secret).bind(row.enabled).bind(row.deleted).bind(&row.revision)
            .bind(&row.id).bind(previous).execute(self.pool.get().await?.deref_mut()).await?.rows_affected() == 1)
    }

    pub(crate) async fn register_environment_provider(&self, row: &crate::oauth_admin::ProviderRecord) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        let mut tx = conn.begin().await?;
        // INSERT obtains the writer lock before examining an existing registry entry.
        sqlx::query("insert or ignore into api_oauth_provider(id,name,namespace,authority,source,config,encrypted_secret,enabled,deleted,revision) values(?,?,?,?,?,?,?,?,?,?)")
            .bind(&row.id).bind(&row.name).bind(&row.namespace).bind(&row.authority).bind(&row.source)
            .bind(&row.config).bind(&row.encrypted_secret).bind(row.enabled).bind(row.deleted).bind(&row.revision)
            .execute(&mut tx).await?;
        let existing: crate::oauth_admin::ProviderRecord = sqlx::query_as("select id,name,namespace,authority,source,config,encrypted_secret,enabled,deleted,revision from api_oauth_provider where name=?")
            .bind(&row.name).fetch_one(&mut tx).await?;
        if existing.source != "legacy" && (existing.source != "environment" || existing.authority != row.authority) {
            hbb_common::bail!("OAuth environment provider name or identity authority conflicts with registry");
        }
        sqlx::query("update api_oauth_provider set source='environment',authority=?,config=?,enabled=1,deleted=0,revision=? where id=?")
            .bind(&row.authority).bind(&row.config).bind(&row.revision).bind(&existing.id).execute(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_peer(&self, id: &str) -> ResultType<Option<Peer>> {
        Ok(sqlx::query_as!(
            Peer,
            "select guid, id, uuid, pk, user, status, info from peer where id = ?",
            id
        )
        .fetch_optional(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn insert_peer(
        &self,
        id: &str,
        uuid: &[u8],
        pk: &[u8],
        info: &str,
    ) -> ResultType<Vec<u8>> {
        let guid = uuid::Uuid::new_v4().as_bytes().to_vec();
        sqlx::query!(
            "insert into peer(guid, id, uuid, pk, info) values(?, ?, ?, ?, ?)",
            guid,
            id,
            uuid,
            pk,
            info
        )
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(guid)
    }

    pub async fn update_pk(
        &self,
        guid: &Vec<u8>,
        id: &str,
        pk: &[u8],
        info: &str,
    ) -> ResultType<()> {
        sqlx::query!(
            "update peer set id=?, pk=?, info=? where guid=?",
            id,
            pk,
            info,
            guid
        )
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(())
    }

    pub async fn get_api_user_by_identity(
        &self,
        provider: &str,
        subject: &str,
    ) -> ResultType<Option<ApiUser>> {
        Ok(sqlx::query_as::<_, ApiUser>(
            "select u.id, u.username, u.email, u.nickname, u.avatar, u.password_hash, u.is_admin, u.status, u.token_version, u.created_at, u.updated_at from api_user u inner join api_identity i on i.user_id = u.id where i.provider = ? and i.subject = ?",
        )
        .bind(provider)
        .bind(subject)
        .fetch_optional(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn create_api_user_with_identity(
        &self,
        id: &str,
        username: &str,
        email: &str,
        password_hash: &str,
        provider: &str,
        subject: &str,
    ) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        let mut tx = conn.begin().await?;
        sqlx::query("insert into api_user(id, username, email, password_hash, is_admin) values(?, ?, ?, ?, 0)")
            .bind(id)
            .bind(username)
            .bind(email)
            .bind(password_hash)
            .execute(&mut *tx)
            .await?;
        sqlx::query("insert into api_identity(provider, subject, user_id) values(?, ?, ?)")
            .bind(provider)
            .bind(subject)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn get_api_user_by_username(&self, username: &str) -> ResultType<Option<ApiUser>> {
        Ok(sqlx::query_as::<_, ApiUser>(
            "select id, username, email, nickname, avatar, password_hash, is_admin, status, token_version, created_at, updated_at from api_user where username = ? collate nocase",
        )
        .bind(username)
        .fetch_optional(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn get_api_user_by_id(&self, id: &str) -> ResultType<Option<ApiUser>> {
        Ok(sqlx::query_as::<_, ApiUser>(
            "select id, username, email, nickname, avatar, password_hash, is_admin, status, token_version, created_at, updated_at from api_user where id = ?",
        )
        .bind(id)
        .fetch_optional(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn create_api_user(
        &self,
        id: &str,
        username: &str,
        email: &str,
        password_hash: &str,
        make_first_user_admin: bool,
    ) -> ResultType<()> {
        sqlx::query(
            "insert into api_user(id, username, email, password_hash, is_admin) values(?, ?, ?, ?, case when ? = 1 and not exists(select 1 from api_user) then 1 else 0 end)",
        )
        .bind(id)
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(if make_first_user_admin { 1 } else { 0 })
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(())
    }

    pub async fn api_user_count(&self) -> ResultType<i64> {
        let row = sqlx::query("select count(*) as count from api_user")
            .fetch_one(self.pool.get().await?.deref_mut())
            .await?;
        use sqlx::Row as _;
        Ok(row.try_get::<i64, _>("count")?)
    }

    pub async fn api_admin_count(&self) -> ResultType<i64> {
        let row = sqlx::query("select count(*) as count from api_user where is_admin = 1 and status = 1")
            .fetch_one(self.pool.get().await?.deref_mut())
            .await?;
        use sqlx::Row as _;
        Ok(row.try_get::<i64, _>("count")?)
    }

    pub async fn list_api_users(&self) -> ResultType<Vec<ApiUser>> {
        Ok(sqlx::query_as::<_, ApiUser>(
            "select id, username, email, nickname, avatar, password_hash, is_admin, status, token_version, created_at, updated_at from api_user order by created_at desc",
        )
        .fetch_all(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn upsert_api_device(
        &self,
        id: &str,
        user_id: &str,
        uuid: &str,
        name: &str,
        os: &str,
        device_type: &str,
        info: &str,
    ) -> ResultType<()> {
        sqlx::query(
            "insert into api_device(id, user_id, uuid, name, os, device_type, info) values(?, ?, ?, ?, ?, ?, ?) on conflict(user_id, uuid) do update set name = excluded.name, os = excluded.os, device_type = excluded.device_type, info = excluded.info, updated_at = current_timestamp where api_device.verified=0",
        )
        .bind(id)
        .bind(user_id)
        .bind(uuid)
        .bind(name)
        .bind(os)
        .bind(device_type)
        .bind(info)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(())
    }

    pub async fn touch_api_device(&self, user_id: &str, id: &str) -> ResultType<()> {
        sqlx::query("update api_device set last_seen_at = current_timestamp, updated_at = current_timestamp where user_id = ? and id = ?")
            .bind(user_id)
            .bind(id)
            .execute(self.pool.get().await?.deref_mut())
            .await?;
        Ok(())
    }

    pub(crate) async fn official_peers(&self, viewer: &str, allow_all: bool, page: &crate::pagination::PageRequest) -> ResultType<crate::pagination::Page<crate::official_peer::PeerRecord>> {
        let now = crate::device_registry::now_ms();
        let base = "from api_device d inner join peer p on p.guid=d.peer_guid inner join api_user u on u.id=d.user_id
            left join api_device_report t on t.peer_guid=p.guid and t.pk=p.pk and t.uuid=p.uuid
            left join peer_registration r on r.peer_guid=p.guid and r.pk=p.pk and r.uuid=p.uuid
            where d.verified=1 and d.verified_pk=p.pk and d.verified_uuid=p.uuid and (?=1 or d.user_id=?)
                and (? is null or d.status=?) and (? is null or p.id like ? escape '\\' or d.name like ? escape '\\')";
        let mut conn = self.pool.get().await?;
        let mut tx = conn.begin().await?;
        let count_sql = format!("select count(*) {base}");
        let total: i64 = sqlx::query_scalar(&count_sql).bind(allow_all).bind(viewer).bind(page.status).bind(page.status)
            .bind(&page.name_pattern).bind(&page.name_pattern).bind(&page.name_pattern).fetch_one(&mut tx).await?;
        let data_sql = format!("select p.id as peer_id,d.user_id,u.username as user_name,d.name,d.os,d.info as legacy_info,t.sysinfo as reported_info,
            d.status,coalesce(r.registered_at_ms,0) as registered_at_ms,
            (coalesce(r.registered_at_ms,0)>? and coalesce(r.registered_at_ms,0)<=?) as online,
            (select min(g.name) from api_device_group g inner join api_device_group_device m on m.group_id=g.id where m.device_id=d.id and g.created_by=?) as device_group_name
            {base} order by p.id,d.id limit ? offset ?");
        let data = sqlx::query_as(&data_sql).bind(now-crate::device_registry::REGISTRATION_TIMEOUT_MS).bind(now).bind(viewer)
            .bind(allow_all).bind(viewer).bind(page.status).bind(page.status)
            .bind(&page.name_pattern).bind(&page.name_pattern).bind(&page.name_pattern).bind(page.limit).bind(page.offset).fetch_all(&mut tx).await?;
        tx.commit().await?;
        Ok(crate::pagination::Page { total:usize::try_from(total)?,data,code:0 })
    }

    pub(crate) async fn paged_api_users(&self, page: &crate::pagination::PageRequest) -> ResultType<crate::pagination::Page<ApiUser>> {
        let mut conn = self.pool.get().await?;
        let mut tx = conn.begin().await?;
        let total: i64 = sqlx::query_scalar("select count(*) from api_user where (? is null or status=?) and (? is null or username like ? escape '\\')")
            .bind(page.status).bind(page.status).bind(&page.name_pattern).bind(&page.name_pattern).fetch_one(&mut tx).await?;
        let data = sqlx::query_as("select id,username,email,nickname,avatar,password_hash,is_admin,status,token_version,created_at,updated_at
            from api_user where (? is null or status=?) and (? is null or username like ? escape '\\') order by username,id limit ? offset ?")
            .bind(page.status).bind(page.status).bind(&page.name_pattern).bind(&page.name_pattern).bind(page.limit).bind(page.offset).fetch_all(&mut tx).await?;
        tx.commit().await?;
        Ok(crate::pagination::Page { total:usize::try_from(total)?,data,code:0 })
    }

    pub(crate) async fn paged_accessible_device_groups(&self, viewer: &str, page: &crate::pagination::PageRequest) -> ResultType<crate::pagination::Page<ApiDeviceGroup>> {
        let mut conn = self.pool.get().await?;
        let mut tx = conn.begin().await?;
        let total: i64 = sqlx::query_scalar("select count(*) from api_device_group where created_by=? and (? is null or name like ? escape '\\')")
            .bind(viewer).bind(&page.name_pattern).bind(&page.name_pattern).fetch_one(&mut tx).await?;
        let data = sqlx::query_as("select id,name,created_by,created_at from api_device_group where created_by=? and (? is null or name like ? escape '\\') order by name,id limit ? offset ?")
            .bind(viewer).bind(&page.name_pattern).bind(&page.name_pattern).bind(page.limit).bind(page.offset).fetch_all(&mut tx).await?;
        tx.commit().await?;
        Ok(crate::pagination::Page { total:usize::try_from(total)?,data,code:0 })
    }

    pub async fn list_api_devices(&self, user_id: &str) -> ResultType<Vec<ApiDevice>> {
        self.api_devices(Some(user_id)).await
    }

    pub async fn list_all_api_devices(&self) -> ResultType<Vec<ApiDevice>> {
        self.api_devices(None).await
    }

    async fn api_devices(&self, user_id: Option<&str>) -> ResultType<Vec<ApiDevice>> {
        let now = crate::device_registry::now_ms();
        Ok(sqlx::query_as::<_,ApiDevice>("select d.id,d.user_id,d.uuid,d.name,d.os,d.device_type,d.info,d.status,d.last_seen_at,d.created_at,d.updated_at,
            coalesce(p.id,'') as peer_id, coalesce((d.verified=1 and p.pk=d.verified_pk and p.uuid=d.verified_uuid),0) as verified,
            coalesce(r.registered_at_ms,0) as registered_at_ms,
            (coalesce(r.registered_at_ms,0)>? and coalesce(r.registered_at_ms,0)<=?) as online
            from api_device d left join peer p on p.guid=d.peer_guid
            left join peer_registration r on r.peer_guid=p.guid and r.uuid=p.uuid and r.pk=p.pk
            where (? is null or (d.user_id=? and d.verified=1 and p.pk=d.verified_pk and p.uuid=d.verified_uuid))
            order by d.id")
            .bind(now-crate::device_registry::REGISTRATION_TIMEOUT_MS).bind(now).bind(user_id).bind(user_id)
            .fetch_all(self.pool.get().await?.deref_mut()).await?)
    }

    pub async fn delete_api_device(&self, id: &str, actor: &str) -> ResultType<bool> {
        let mut conn = self.pool.get().await?;
        let mut tx = conn.begin().await?;
        sqlx::query("update api_schema_lock set id=id where id=1").execute(&mut tx).await?;
        let row: Option<(String,String,Option<Vec<u8>>)> = sqlx::query_as("select d.user_id,coalesce(p.id,''),d.verified_pk from api_device d left join peer p on p.guid=d.peer_guid where d.id=?")
            .bind(id).fetch_optional(&mut tx).await?;
        let (owner,peer_id,pk) = match row { Some(row) => row, None => return Ok(false) };
        let fingerprint = pk.map(|pk|crate::device_registry::key_fingerprint(&pk)).unwrap_or_default();
        sqlx::query("insert into api_device_binding_audit(id,actor_id,action,device_id,peer_id,owner_id,pk_fingerprint,recorded_at_ms) values(?,?,'delete',?,?,?,?,?)")
            .bind(uuid::Uuid::new_v4().to_string()).bind(actor).bind(id).bind(peer_id).bind(owner).bind(fingerprint).bind(crate::device_registry::now_ms())
            .execute(&mut tx).await?;
        sqlx::query("delete from api_device where id=?").bind(id).execute(&mut tx).await?;
        tx.commit().await?;
        Ok(true)
    }

    pub async fn update_api_user_password(&self, id: &str, password_hash: &str) -> ResultType<()> {
        sqlx::query("update api_user set password_hash = ?, updated_at = current_timestamp where id = ?")
            .bind(password_hash)
            .bind(id)
            .execute(self.pool.get().await?.deref_mut())
            .await?;
        Ok(())
    }

    pub async fn list_api_user_groups(&self, user_id: &str) -> ResultType<Vec<ApiUserGroup>> {
        Ok(sqlx::query_as::<_, ApiUserGroup>(
            "select id, name, created_by, created_at from api_user_group where created_by = ? or id in (select group_id from api_user_group_member where user_id = ?) order by name",
        )
        .bind(user_id)
        .bind(user_id)
        .fetch_all(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn list_api_user_group_members(
        &self,
        user_id: &str,
        is_admin: bool,
    ) -> ResultType<Vec<ApiUserGroupMember>> {
        let query = if is_admin {
            "select group_id, user_id from api_user_group_member order by group_id, user_id"
        } else {
            "select group_id, user_id from api_user_group_member where user_id = ? or group_id in (select id from api_user_group where created_by = ?) order by group_id, user_id"
        };
        let mut query = sqlx::query_as::<_, ApiUserGroupMember>(query);
        if !is_admin {
            query = query.bind(user_id).bind(user_id);
        }
        Ok(query.fetch_all(self.pool.get().await?.deref_mut()).await?)
    }

    pub async fn add_api_user_group_member(
        &self,
        group_id: &str,
        user_id: &str,
        actor_id: &str,
        is_admin: bool,
    ) -> ResultType<bool> {
        let result = sqlx::query(
            "insert or ignore into api_user_group_member(group_id, user_id) select ?, ? where exists(select 1 from api_user where id = ?) and (? = 1 or exists(select 1 from api_user_group where id = ? and created_by = ?))",
        )
        .bind(group_id)
        .bind(user_id)
        .bind(user_id)
        .bind(if is_admin { 1 } else { 0 })
        .bind(group_id)
        .bind(actor_id)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn remove_api_user_group_member(
        &self,
        group_id: &str,
        user_id: &str,
        actor_id: &str,
        is_admin: bool,
    ) -> ResultType<bool> {
        let result = sqlx::query(
            "delete from api_user_group_member where group_id = ? and user_id = ? and (? = 1 or exists(select 1 from api_user_group where id = ? and created_by = ?))",
        )
        .bind(group_id)
        .bind(user_id)
        .bind(if is_admin { 1 } else { 0 })
        .bind(group_id)
        .bind(actor_id)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn create_api_user_group(
        &self,
        id: &str,
        name: &str,
        created_by: &str,
    ) -> ResultType<()> {
        sqlx::query("insert into api_user_group(id, name, created_by) values(?, ?, ?)")
            .bind(id)
            .bind(name)
            .bind(created_by)
            .execute(self.pool.get().await?.deref_mut())
            .await?;
        Ok(())
    }

    pub async fn delete_api_user_group(&self, id: &str, created_by: &str) -> ResultType<bool> {
        let result = sqlx::query("delete from api_user_group where id = ? and created_by = ?")
            .bind(id)
            .bind(created_by)
            .execute(self.pool.get().await?.deref_mut())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list_api_device_groups(&self, user_id: &str) -> ResultType<Vec<ApiDeviceGroup>> {
        Ok(sqlx::query_as::<_, ApiDeviceGroup>(
            "select id, name, created_by, created_at from api_device_group where created_by = ? order by name",
        )
        .bind(user_id)
        .fetch_all(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn create_api_device_group(
        &self,
        id: &str,
        name: &str,
        created_by: &str,
    ) -> ResultType<()> {
        sqlx::query("insert into api_device_group(id, name, created_by) values(?, ?, ?)")
            .bind(id)
            .bind(name)
            .bind(created_by)
            .execute(self.pool.get().await?.deref_mut())
            .await?;
        Ok(())
    }

    pub async fn list_api_device_group_members(
        &self,
        user_id: &str,
        allow_all_devices: bool,
    ) -> ResultType<Vec<ApiDeviceGroupMember>> {
        Ok(sqlx::query_as::<_, ApiDeviceGroupMember>(
            "select m.group_id, m.device_id from api_device_group_device m inner join api_device_group g on g.id = m.group_id inner join api_device d on d.id = m.device_id where g.created_by = ? and (? = 1 or (d.user_id = ? and d.verified=1 and exists(select 1 from peer p where p.guid=d.peer_guid and p.pk=d.verified_pk and p.uuid=d.verified_uuid))) order by m.group_id, m.device_id",
        )
        .bind(user_id)
        .bind(if allow_all_devices { 1 } else { 0 })
        .bind(user_id)
        .fetch_all(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn add_api_device_group_member(
        &self,
        group_id: &str,
        device_id: &str,
        user_id: &str,
        allow_all_devices: bool,
    ) -> ResultType<bool> {
        let result = sqlx::query(
            "insert into api_device_group_device(group_id, device_id) select g.id, d.id from api_device_group g inner join api_device d on d.id = ? where g.id = ? and g.created_by = ? and (? = 1 or (d.user_id = ? and d.verified=1 and exists(select 1 from peer p where p.guid=d.peer_guid and p.pk=d.verified_pk and p.uuid=d.verified_uuid))) on conflict(group_id, device_id) do update set device_id = excluded.device_id",
        )
        .bind(device_id)
        .bind(group_id)
        .bind(user_id)
        .bind(if allow_all_devices { 1 } else { 0 })
        .bind(user_id)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn remove_api_device_group_member(
        &self,
        group_id: &str,
        device_id: &str,
        user_id: &str,
        allow_all_devices: bool,
    ) -> ResultType<bool> {
        let result = sqlx::query(
            "delete from api_device_group_device where group_id = ? and device_id = ? and group_id in (select id from api_device_group where created_by = ?) and device_id in (select id from api_device where ? = 1 or user_id = ?)",
        )
        .bind(group_id)
        .bind(device_id)
        .bind(user_id)
        .bind(if allow_all_devices { 1 } else { 0 })
        .bind(user_id)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_api_device_group(&self, id: &str, created_by: &str) -> ResultType<bool> {
        let result = sqlx::query("delete from api_device_group where id = ? and created_by = ?")
            .bind(id)
            .bind(created_by)
            .execute(self.pool.get().await?.deref_mut())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn set_api_user_status(&self, id: &str, status: i64) -> ResultType<bool> {
        let result = sqlx::query(
            "update api_user set status = ?, updated_at = current_timestamp where id = ? and not (is_admin = 1 and ? = 0 and (select count(*) from api_user where is_admin = 1 and status = 1) <= 1)",
        )
        .bind(status)
        .bind(id)
        .bind(status)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_api_user(&self, id: &str) -> ResultType<bool> {
        let result = sqlx::query(
            "delete from api_user where id = ? and not (is_admin = 1 and (select count(*) from api_user where is_admin = 1 and status = 1) <= 1)",
        )
        .bind(id)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn create_api_session(
        &self,
        id: &str,
        user_id: &str,
        device_id: &str,
        device_uuid: &str,
        device_name: &str,
        device_os: &str,
        device_type: &str,
        expires_at: i64,
    ) -> ResultType<()> {
        sqlx::query(
            "insert into api_session(id, user_id, device_id, device_uuid, device_name, device_os, device_type, expires_at) values(?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(user_id)
        .bind(device_id)
        .bind(device_uuid)
        .bind(device_name)
        .bind(device_os)
        .bind(device_type)
        .bind(expires_at)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(())
    }

    pub async fn list_api_sessions(&self) -> ResultType<Vec<ApiSessionView>> {
        Ok(sqlx::query_as::<_, ApiSessionView>(
            "select s.id, s.user_id, u.username, s.device_id, s.device_uuid, s.device_name, s.device_os, s.device_type, s.expires_at, s.revoked_at, s.created_at, s.last_used_at from api_session s inner join api_user u on u.id = s.user_id order by s.created_at desc",
        )
        .fetch_all(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn revoke_api_session_by_id(&self, id: &str, now: i64) -> ResultType<bool> {
        let result = sqlx::query(
            "update api_session set revoked_at = coalesce(revoked_at, ?) where id = ?",
        )
        .bind(now)
        .bind(id)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn is_api_session_active(
        &self,
        id: &str,
        user_id: &str,
        now: i64,
    ) -> ResultType<bool> {
        let row = sqlx::query(
            "select 1 as active from api_session where id = ? and user_id = ? and revoked_at is null and expires_at > ?",
        )
        .bind(id)
        .bind(user_id)
        .bind(now)
        .fetch_optional(self.pool.get().await?.deref_mut())
        .await?;
        Ok(row.is_some())
    }

    pub async fn touch_api_session(&self, id: &str, user_id: &str) -> ResultType<()> {
        sqlx::query(
            "update api_session set last_used_at = current_timestamp where id = ? and user_id = ? and revoked_at is null",
        )
        .bind(id)
        .bind(user_id)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(())
    }

    pub async fn revoke_api_session(&self, id: &str, user_id: &str, now: i64) -> ResultType<()> {
        sqlx::query(
            "update api_session set revoked_at = ?, last_used_at = current_timestamp where id = ? and user_id = ? and revoked_at is null",
        )
        .bind(now)
        .bind(id)
        .bind(user_id)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(())
    }

    pub async fn list_api_address_book_entries(
        &self,
        user_id: &str,
    ) -> ResultType<Vec<ApiAddressBookEntry>> {
        Ok(sqlx::query_as::<_, ApiAddressBookEntry>(
            "select id, user_id, peer_id, username, hostname, alias, platform, tags, force_always_relay, created_at, updated_at from api_address_book_entry where user_id = ? order by updated_at desc, id",
        )
        .bind(user_id)
        .fetch_all(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn get_api_address_book_entry(
        &self,
        user_id: &str,
        peer_id: &str,
    ) -> ResultType<Option<ApiAddressBookEntry>> {
        Ok(sqlx::query_as::<_, ApiAddressBookEntry>(
            "select id, user_id, peer_id, username, hostname, alias, platform, tags, force_always_relay, created_at, updated_at from api_address_book_entry where user_id = ? and peer_id = ?",
        )
        .bind(user_id)
        .bind(peer_id)
        .fetch_optional(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn delete_address_book_peer_and_snapshot(&self, user_id: &str, peer_id: &str, snapshot: &str) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        let mut tx = conn.begin().await?;
        sqlx::query("delete from api_address_book_entry where user_id=? and peer_id=?")
            .bind(user_id).bind(peer_id).execute(&mut tx).await?;
        sqlx::query("insert into api_address_book_snapshot(user_id,data) values(?,?) on conflict(user_id) do update set data=excluded.data,updated_at=current_timestamp")
            .bind(user_id).bind(snapshot).execute(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn save_address_book_tag_change(&self, user_id: &str, snapshot: &str, old: &str, new: Option<&str>) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        let mut tx = conn.begin().await?;
        let entries: Vec<(String,String)> = sqlx::query_as("select id,tags from api_address_book_entry where user_id=?").bind(user_id).fetch_all(&mut tx).await?;
        for (id,raw) in entries {
            let tags: Vec<String> = serde_json::from_str(&raw)?;
            let updated: Vec<String> = tags.iter().filter_map(|tag| {
                if tag.eq_ignore_ascii_case(old) { new.map(str::to_owned) } else { Some(tag.clone()) }
            }).collect();
            if updated!=tags {
                sqlx::query("update api_address_book_entry set tags=?,updated_at=current_timestamp where id=? and user_id=?")
                    .bind(serde_json::to_string(&updated)?).bind(id).bind(user_id).execute(&mut tx).await?;
            }
        }
        sqlx::query("insert into api_address_book_snapshot(user_id,data) values(?,?) on conflict(user_id) do update set data=excluded.data,updated_at=current_timestamp")
            .bind(user_id).bind(snapshot).execute(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn upsert_api_address_book_entry(
        &self,
        entry: &ApiAddressBookEntry,
        snapshot: &str,
    ) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        let mut tx = conn.begin().await?;
        let updated = sqlx::query("update api_address_book_entry set peer_id=?,username=?,hostname=?,alias=?,platform=?,tags=?,force_always_relay=?,updated_at=current_timestamp where user_id=? and id=?")
            .bind(&entry.peer_id).bind(&entry.username).bind(&entry.hostname).bind(&entry.alias).bind(&entry.platform)
            .bind(&entry.tags).bind(entry.force_always_relay).bind(&entry.user_id).bind(&entry.id).execute(&mut tx).await?;
        if updated.rows_affected()==0 {
            sqlx::query("insert into api_address_book_entry(id,user_id,peer_id,username,hostname,alias,platform,tags,force_always_relay) values(?,?,?,?,?,?,?,?,?)")
                .bind(&entry.id).bind(&entry.user_id).bind(&entry.peer_id).bind(&entry.username).bind(&entry.hostname)
                .bind(&entry.alias).bind(&entry.platform).bind(&entry.tags).bind(entry.force_always_relay).execute(&mut tx).await?;
        }
        sqlx::query("insert into api_address_book_snapshot(user_id,data) values(?,?) on conflict(user_id) do update set data=excluded.data,updated_at=current_timestamp")
            .bind(&entry.user_id).bind(snapshot).execute(&mut tx).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_api_address_book_entry(&self, id: &str, user_id: &str) -> ResultType<bool> {
        let result = sqlx::query("delete from api_address_book_entry where id = ? and user_id = ?")
            .bind(id)
            .bind(user_id)
            .execute(self.pool.get().await?.deref_mut())
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn clear_api_address_book_entries(&self, user_id: &str) -> ResultType<()> {
        sqlx::query("delete from api_address_book_entry where user_id = ?")
            .bind(user_id)
            .execute(self.pool.get().await?.deref_mut())
            .await?;
        Ok(())
    }

    pub async fn get_api_address_book(&self, user_id: &str) -> ResultType<Option<String>> {
        let row = sqlx::query("select data from api_address_book_snapshot where user_id = ?")
            .bind(user_id)
            .fetch_optional(self.pool.get().await?.deref_mut())
            .await?;
        use sqlx::Row as _;
        Ok(row
            .map(|row| row.try_get::<String, _>("data"))
            .transpose()?)
    }

    pub async fn upsert_api_address_book(&self, user_id: &str, data: &str) -> ResultType<()> {
        sqlx::query(
            "insert into api_address_book_snapshot(user_id, data) values(?, ?) on conflict(user_id) do update set data = excluded.data, updated_at = current_timestamp",
        )
        .bind(user_id)
        .bind(data)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(())
    }

    pub async fn increment_api_user_token_version(&self, id: &str) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        let mut tx = conn.begin().await?;
        sqlx::query("update api_user set token_version = token_version + 1, updated_at = current_timestamp where id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("update api_session set revoked_at = coalesce(revoked_at, strftime('%s','now')) where user_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use hbb_common::tokio;
    #[test]
    fn test_insert() {
        insert();
    }

    #[tokio::main(flavor = "multi_thread")]
    async fn insert() {
        let directory = std::env::temp_dir().join(format!("rustdesk-peer-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let database_path = directory.join("test.sqlite3");
        let db = super::Database::new(database_path.to_str().unwrap()).await.unwrap();
        let mut jobs = vec![];
        for i in 0..10000 {
            let cloned = db.clone();
            let id = i.to_string();
            let a = tokio::spawn(async move {
                let empty_vec = Vec::new();
                cloned
                    .insert_peer(&id, &empty_vec, &empty_vec, "")
                    .await
                    .unwrap();
            });
            jobs.push(a);
        }
        for i in 0..10000 {
            let cloned = db.clone();
            let id = i.to_string();
            let a = tokio::spawn(async move {
                cloned.get_peer(&id).await.unwrap();
            });
            jobs.push(a);
        }
        for result in hbb_common::futures::future::join_all(jobs).await {
            result.expect("database task must succeed");
        }
        drop(db);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

impl Database {
    pub async fn save_peer_registrations(&self, observations: &[crate::device_registry::RegistrationObservation]) -> ResultType<()> {
        let mut conn = self.pool.get().await?;
        let mut tx = conn.begin().await?;
        for observation in observations {
            sqlx::query("insert into peer_registration(peer_guid,uuid,pk,registered_at_ms)
                select guid,uuid,pk,? from peer where guid=? and uuid=? and pk=?
                on conflict(peer_guid) do update set uuid=excluded.uuid,pk=excluded.pk,
                registered_at_ms=case when peer_registration.uuid=excluded.uuid and peer_registration.pk=excluded.pk
                    then max(peer_registration.registered_at_ms,excluded.registered_at_ms) else excluded.registered_at_ms end")
                .bind(observation.registered_at_ms).bind(&observation.guid).bind(&observation.uuid).bind(&observation.pk)
                .execute(&mut tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub(crate) async fn save_device_report(&self, value: &serde_json::Value, heartbeat: bool) -> Result<bool,crate::device_registry::RegistryError> {
        use crate::device_registry::{report_identity,now_ms,RegistryError,REPORT_INTERVAL_MS,MAX_REPORTS};
        let (id,uuid) = report_identity(value)?;
        // Unknown IDs and mismatched UUIDs never acquire a SQLite writer lock.
        let peer = self.get_peer(&id).await.map_err(|_| RegistryError::Storage)?.ok_or(RegistryError::NotFound)?;
        if peer.uuid != uuid { return Err(RegistryError::NotFound); }
        let now = now_ms();
        let serialized = serde_json::to_string(value).map_err(|_| RegistryError::Storage)?;
        let mut conn = self.pool.get().await.map_err(|_| RegistryError::Storage)?;
        let mut tx = conn.begin().await.map_err(|_| RegistryError::Storage)?;
        sqlx::query("update api_schema_lock set id=id where id=1").execute(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        let current: Option<(Vec<u8>,Vec<u8>)> = sqlx::query_as("select guid,pk from peer where id=? and uuid=?")
            .bind(&id).bind(&uuid).fetch_optional(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        let (guid,pk) = current.ok_or(RegistryError::NotFound)?;
        let existing: Option<(Vec<u8>,Vec<u8>,Option<String>,i64,i64)> = sqlx::query_as("select uuid,pk,sysinfo,sysinfo_at_ms,heartbeat_at_ms from api_device_report where peer_guid=?")
            .bind(&guid).fetch_optional(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        let same_identity = existing.as_ref().is_some_and(|(old_uuid,old_pk,_,_,_)| *old_uuid == uuid && *old_pk == pk);
        if let Some((_,_,_,sysinfo_at,heartbeat_at)) = existing.as_ref().filter(|_| same_identity) {
            let previous = if heartbeat { *heartbeat_at } else { *sysinfo_at };
            if previous > 0 && now.saturating_sub(previous) < REPORT_INTERVAL_MS { return Err(RegistryError::RateLimited); }
        }
        if existing.is_none() {
            let count: i64 = sqlx::query_scalar("select count(*) from api_device_report").fetch_one(&mut tx).await.map_err(|_| RegistryError::Storage)?;
            if count >= MAX_REPORTS { return Err(RegistryError::Capacity); }
        }
        if !same_identity {
            sqlx::query("insert into api_device_report(peer_guid,uuid,pk) values(?,?,?) on conflict(peer_guid) do update set
                uuid=excluded.uuid,pk=excluded.pk,sysinfo=null,sysinfo_at_ms=0,heartbeat=null,heartbeat_at_ms=0")
                .bind(&guid).bind(&uuid).bind(&pk).execute(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        }
        let sql = if heartbeat { "update api_device_report set heartbeat=?,heartbeat_at_ms=? where peer_guid=?" }
            else { "update api_device_report set sysinfo=?,sysinfo_at_ms=? where peer_guid=?" };
        sqlx::query(sql).bind(serialized).bind(now).bind(&guid).execute(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        let needs_sysinfo = heartbeat && (!same_identity || existing.as_ref().map_or(true,|(_,_,sysinfo,_,_)| sysinfo.is_none()));
        tx.commit().await.map_err(|_| RegistryError::Storage)?;
        Ok(needs_sysinfo)
    }

    pub(crate) async fn registered_devices(&self, peer_id: Option<&str>) -> ResultType<Vec<crate::device_registry::RegisteredDeviceView>> {
        use crate::device_registry::{RegisteredDeviceView,now_ms,REGISTRATION_TIMEOUT_MS,key_fingerprint};
        let rows: Vec<(String,Vec<u8>,Vec<u8>,i64,Option<String>,Option<String>,i64,Option<String>,Option<String>)> = sqlx::query_as(
            "select p.id,p.uuid,p.pk,coalesce(r.registered_at_ms,0),d.id,d.user_id,
                coalesce((d.verified=1 and d.verified_pk=p.pk and d.verified_uuid=p.uuid),0),t.sysinfo,t.heartbeat
                from peer p left join peer_registration r on r.peer_guid=p.guid and r.pk=p.pk and r.uuid=p.uuid
                left join api_device d on d.peer_guid=p.guid and d.verified=1
                left join api_device_report t on t.peer_guid=p.guid and t.pk=p.pk and t.uuid=p.uuid where (? is null or p.id=?) order by p.id limit 100")
            .bind(peer_id).bind(peer_id).fetch_all(self.pool.get().await?.deref_mut()).await?;
        let now = now_ms();
        rows.into_iter().map(|(peer_id,uuid,pk,registered_at_ms,device_id,owner_id,verified,sysinfo,heartbeat)| {
            Ok(RegisteredDeviceView { peer_id,uuid:base64::encode(uuid),pk_fingerprint:key_fingerprint(&pk),registered_at_ms,
                online:registered_at_ms > now-REGISTRATION_TIMEOUT_MS && registered_at_ms <= now,
                device_id,owner_id,verified:verified==1,
                untrusted_sysinfo:sysinfo.map(|value|serde_json::from_str(&value)).transpose()?,
                untrusted_heartbeat:heartbeat.map(|value|serde_json::from_str(&value)).transpose()? })
        }).collect()
    }

    pub(crate) async fn bind_api_device(&self, actor: &str, request: &crate::device_registry::BindDeviceRequest) -> Result<String,crate::device_registry::RegistryError> {
        use crate::device_registry::{RegistryError,key_fingerprint,now_ms};
        if request.peer_id.is_empty() || request.peer_id.len()>128 || request.user_id.len()>128 || request.pk_fingerprint.len()>128 {
            return Err(RegistryError::Invalid("invalid_device_binding"));
        }
        let mut conn = self.pool.get().await.map_err(|_| RegistryError::Storage)?;
        let mut tx = conn.begin().await.map_err(|_| RegistryError::Storage)?;
        sqlx::query("update api_schema_lock set id=id where id=1").execute(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        let peer: Option<(Vec<u8>,Vec<u8>,Vec<u8>)> = sqlx::query_as("select guid,uuid,pk from peer where id=?")
            .bind(&request.peer_id).fetch_optional(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        let (guid,uuid,pk) = peer.ok_or(RegistryError::NotFound)?;
        if pk.len()!=32 || key_fingerprint(&pk)!=request.pk_fingerprint { return Err(RegistryError::Conflict); }
        let user: Option<String> = sqlx::query_scalar("select id from api_user where id=? and status=1")
            .bind(&request.user_id).fetch_optional(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        if user.is_none() { return Err(RegistryError::NotFound); }
        let device: Option<String> = sqlx::query_scalar("select id from api_device where peer_guid=? and verified=1")
            .bind(&guid).fetch_optional(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        let canonical_uuid = base64::encode(&uuid);
        let legacy: Option<(String,Option<Vec<u8>>,i64)> = sqlx::query_as("select id,peer_guid,verified from api_device where user_id=? and uuid=?")
            .bind(&request.user_id).bind(&canonical_uuid).fetch_optional(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        if legacy.as_ref().is_some_and(|(_,old_guid,verified)| *verified==1 && old_guid.as_ref()!=Some(&guid)) { return Err(RegistryError::Conflict); }
        let legacy = legacy.map(|(id,_,_)|id);
        if device.is_some() && legacy.is_some() && device!=legacy { return Err(RegistryError::Conflict); }
        let id = device.or(legacy).unwrap_or_else(||uuid::Uuid::new_v4().to_string());
        // Account association is created only by this explicit, fingerprint-checked administrator operation.
        sqlx::query("insert into api_device(id,user_id,uuid,peer_guid,verified,verified_uuid,verified_pk) values(?,?,?,?,1,?,?)
            on conflict(id) do update set user_id=excluded.user_id,uuid=excluded.uuid,peer_guid=excluded.peer_guid,
                verified=1,verified_uuid=excluded.verified_uuid,verified_pk=excluded.verified_pk,updated_at=current_timestamp")
            .bind(&id).bind(&request.user_id).bind(&canonical_uuid).bind(&guid).bind(&uuid).bind(&pk)
            .execute(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        sqlx::query("insert into api_device_binding_audit(id,actor_id,action,device_id,peer_id,owner_id,pk_fingerprint,recorded_at_ms) values(?,?,'bind',?,?,?,?,?)")
            .bind(uuid::Uuid::new_v4().to_string()).bind(actor).bind(&id).bind(&request.peer_id).bind(&request.user_id).bind(&request.pk_fingerprint).bind(now_ms())
            .execute(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        tx.commit().await.map_err(|_| RegistryError::Storage)?;
        Ok(id)
    }

    pub(crate) async fn unbind_api_device(&self, actor: &str, id: &str) -> Result<(),crate::device_registry::RegistryError> {
        use crate::device_registry::{RegistryError,key_fingerprint,now_ms};
        let mut conn = self.pool.get().await.map_err(|_| RegistryError::Storage)?;
        let mut tx = conn.begin().await.map_err(|_| RegistryError::Storage)?;
        sqlx::query("update api_schema_lock set id=id where id=1").execute(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        let device: Option<(String,String,Vec<u8>)> = sqlx::query_as("select coalesce(p.id,''),d.user_id,d.verified_pk from api_device d left join peer p on p.guid=d.peer_guid where d.id=? and d.verified=1")
            .bind(id).fetch_optional(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        let (peer_id,owner,pk) = device.ok_or(RegistryError::NotFound)?;
        sqlx::query("update api_device set verified=0,updated_at=current_timestamp where id=?").bind(id).execute(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        sqlx::query("insert into api_device_binding_audit(id,actor_id,action,device_id,peer_id,owner_id,pk_fingerprint,recorded_at_ms) values(?,?,'unbind',?,?,?,?,?)")
            .bind(uuid::Uuid::new_v4().to_string()).bind(actor).bind(id).bind(peer_id).bind(owner).bind(key_fingerprint(&pk)).bind(now_ms())
            .execute(&mut tx).await.map_err(|_| RegistryError::Storage)?;
        tx.commit().await.map_err(|_| RegistryError::Storage)?;
        Ok(())
    }
}
