# db-sqlx-tester

这是一个用 postgres 测试 sqlx 的工具。 目前它只支持 tokio runtime。


## How to use it

You should first create a `TestDb` data Structure in your tests. It wil automatically create a database and a connection pool for you.

您应该首先在测试中创建一个 `TestDb` 数据结构。 它会自动为你创建一个数据库和一个连接池。

```rust
#[tokio::test]
fn some_awesom_test(){
  let tdb = TestDb::new("localhost",5432,"postgres","postgres","./migrations");
  let pool = tdb.get_pool().await;
}
```
