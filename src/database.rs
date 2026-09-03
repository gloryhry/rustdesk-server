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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiUserGroup {
    pub id: String,
    pub name: String,
    pub created_by: String,
    pub created_at: String,
}

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

    pub async fn delete_api_user_group(&self, id: &str, created_by: &str) -> ResultType<()> {
        sqlx::query("delete from api_user_group where id = ? and created_by = ?")
            .bind(id)
            .bind(created_by)
            .execute(self.pool.get().await?.deref_mut())
            .await?;
        Ok(())
    }

    pub async fn set_api_user_status(&self, id: &str, status: i64) -> ResultType<()> {
        sqlx::query("update api_user set status = ?, updated_at = current_timestamp where id = ?")
            .bind(status)
            .bind(id)
            .execute(self.pool.get().await?.deref_mut())
            .await?;
        Ok(())
    }

    pub async fn delete_api_user(&self, id: &str) -> ResultType<()> {
        sqlx::query("delete from api_user where id = ?")
            .bind(id)
            .execute(self.pool.get().await?.deref_mut())
            .await?;
        Ok(())
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
        let db = super::Database::new("test.sqlite3").await.unwrap();
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
        hbb_common::futures::future::join_all(jobs).await;
    }
}
