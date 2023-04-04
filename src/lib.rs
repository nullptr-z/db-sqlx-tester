use std::{path::Path, thread};

use sqlx::{migrate::Migrator, query, Connection, Executor, PgConnection, PgPool};
use tokio::runtime::Runtime;
use uuid::Uuid;

pub struct TestDb {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: String,
}

impl TestDb {
    /** 执行一系列访问数据库服务所需参数。从 migration_path路径读取 SQL 文件 */
    pub fn new(
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
        password: impl Into<String>,
        migration_path: impl Into<String>,
    ) -> Self {
        let host = host.into();
        let user = user.into();
        let password = password.into();
        let uuid = Uuid::new_v4();
        let dbname = format!("test_{}", uuid); // 生成一个随机库名，用于测试
        let dbname_cloned = dbname.clone();

        let tdb = Self {
            host,
            port,
            user,
            password,
            dbname,
        };

        let server_url = tdb.server_url();
        let url = tdb.url();
        let migration_path = migration_path.into();

        thread::spawn(move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async move {
                // use server url to create database
                let mut conn = PgConnection::connect(&server_url).await.unwrap();
                // 创建一个测试库，`dbname`
                conn.execute(format!(r#"CREATE DATABASE "{}""#, dbname_cloned).as_str())
                    .await
                    .expect("Error while querying the reservation database");

                // now connect to test database for migration
                let mut conn = PgConnection::connect(&url).await.unwrap();
                // 通过 SQL 文件所在的路径创建 Migrator对象
                let m = Migrator::new(Path::new(&migration_path)).await.unwrap();
                // 执行这些 SQL 文件，`command: sqlx migrate run`
                m.run(&mut conn).await.unwrap();
            });
        })
        .join()
        .expect("Error thread create database ");

        tdb
    }

    pub fn server_url(&self) -> String {
        if !self.password.is_empty() {
            format!(
                "postgres://{}:{}@{}:{}",
                self.user, self.password, self.host, self.port
            )
        } else {
            format!("postgres://{}@{}:{}", self.user, self.host, self.port)
        }
    }

    pub fn url(&self) -> String {
        format!("{}/{}", self.server_url(), self.dbname)
    }

    pub async fn get_pool(&self) -> PgPool {
        sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&self.url())
            .await
            .unwrap()
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let server_url = self.server_url().clone();
        let dbname = self.dbname.clone();
        // drop 时删除数据库
        thread::spawn(move || {
            let rt= Runtime::new().unwrap();
            rt.block_on(async move {
                let mut conn = PgConnection::connect(&server_url).await.unwrap();
                // terminate existing connection`中断现有连接
                query(&format!(r#"SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND datname = '{}'"#,dbname))
                .execute(&mut conn)
                .await
                .expect("Terminal all other connections");

                conn.execute(format!(r#"DROP DATABASE "{}""#, dbname).as_str())
                    .await
                    .expect("Error while querying the reservation database");
            });
        })
        .join()
        .expect("Error thread drop database ");
    }
}

#[cfg(test)]
mod tests {
    use crate::TestDb;

    #[tokio::test]
    async fn test_db_should_create_and_drop() {
        let tdb = TestDb::new("127.0.0.1", 5432, "zheng", "zz", "./migrations");
        let pool = tdb.get_pool().await;
        // insert todo
        sqlx::query("INSERT INTO todos (title) VALUES ('test')")
            .execute(&pool)
            .await
            .unwrap();
        // get todo
        let (id, title) = sqlx::query_as::<_, (i32, String)>("SELECT id, title FROM todos")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert_eq!(id, 1);
        assert_eq!(title, "test");
    }
}
