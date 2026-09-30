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
            "insert into api_device(id, user_id, uuid, name, os, device_type, info) values(?, ?, ?, ?, ?, ?, ?) on conflict(user_id, uuid) do update set id = excluded.id, name = excluded.name, os = excluded.os, device_type = excluded.device_type, info = excluded.info, status = 1, last_seen_at = current_timestamp, updated_at = current_timestamp",
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

    pub async fn list_api_devices(&self, user_id: &str) -> ResultType<Vec<ApiDevice>> {
        Ok(sqlx::query_as::<_, ApiDevice>(
            "select id, user_id, uuid, name, os, device_type, info, status, last_seen_at, created_at, updated_at from api_device where user_id = ? order by last_seen_at desc",
        )
        .bind(user_id)
        .fetch_all(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn list_all_api_devices(&self) -> ResultType<Vec<ApiDevice>> {
        Ok(sqlx::query_as::<_, ApiDevice>(
            "select id, user_id, uuid, name, os, device_type, info, status, last_seen_at, created_at, updated_at from api_device order by last_seen_at desc",
        )
        .fetch_all(self.pool.get().await?.deref_mut())
        .await?)
    }

    pub async fn delete_api_device(&self, id: &str) -> ResultType<bool> {
        let result = sqlx::query("delete from api_device where id = ?")
            .bind(id)
            .execute(self.pool.get().await?.deref_mut())
            .await?;
        Ok(result.rows_affected() > 0)
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
            "select m.group_id, m.device_id from api_device_group_device m inner join api_device_group g on g.id = m.group_id inner join api_device d on d.id = m.device_id where g.created_by = ? and (? = 1 or d.user_id = ?) order by m.group_id, m.device_id",
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
            "insert into api_device_group_device(group_id, device_id) select g.id, d.id from api_device_group g inner join api_device d on d.id = ? where g.id = ? and g.created_by = ? and (? = 1 or d.user_id = ?) on conflict(group_id, device_id) do update set device_id = excluded.device_id",
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

    pub async fn delete_api_address_book_entry_by_key(
        &self,
        key: &str,
        user_id: &str,
    ) -> ResultType<bool> {
        let result = sqlx::query(
            "delete from api_address_book_entry where user_id = ? and (id = ? or peer_id = ?)",
        )
        .bind(user_id)
        .bind(key)
        .bind(key)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn upsert_api_address_book_entry(
        &self,
        entry: &ApiAddressBookEntry,
    ) -> ResultType<()> {
        sqlx::query(
            "insert into api_address_book_entry(id, user_id, peer_id, username, hostname, alias, platform, tags, force_always_relay) values(?, ?, ?, ?, ?, ?, ?, ?, ?) on conflict(user_id, peer_id) do update set id = excluded.id, username = excluded.username, hostname = excluded.hostname, alias = excluded.alias, platform = excluded.platform, tags = excluded.tags, force_always_relay = excluded.force_always_relay, updated_at = current_timestamp",
        )
        .bind(&entry.id)
        .bind(&entry.user_id)
        .bind(&entry.peer_id)
        .bind(&entry.username)
        .bind(&entry.hostname)
        .bind(&entry.alias)
        .bind(&entry.platform)
        .bind(&entry.tags)
        .bind(entry.force_always_relay)
        .execute(self.pool.get().await?.deref_mut())
        .await?;
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
