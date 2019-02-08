# Actix Diesel Actor
> Packages diesel queries into actix messages

Provides message types to package diesel queries into actix messages.
Provides actors to act on these messeages, and execute queries.

The SQuery Message can be used for anything which is a complete diesel query and implements diesel SelectQuery.
This is useful for when your app does read operations to a read only replica of your database.
The WQuery accepts write operations like INSERT,UPDATE,DELETE in addition to SELECT.
This can be used for generic purpose or when your db cluster has many replicas and one writable master DB.

The db connections are configurable using environment variables, so you can pass the db connections to the app from your swarm configuration tool, like Kubernetes.

At the moment it only supports PostgreSQL.

## Usage
Helpers to create an actix-web app:
```rust
    pub use actix_diesel_actor as db;
    let sys = actix::System::new("my_actix_sys");

    let raddr = db::db_setup(db::ConnectionType::Read);
    let waddr = db::db_setup(db::ConnectionType::Write);

    // routes need to be defined in a most specific to least specific order
    server::new(move || {
        vec![
        App::with_state(db::AppState{rdb: raddr.clone(), wdb: waddr.clone()})
        ...
        ]
    })
    .bind(bind_url)
    .unwrap()
    .run();
    let _ = sys.run();
```
Use it in a handler:
```rust
pub fn index(req: &HttpRequest<AppState>) -> Result<HttpResponse, crate::db::DbExecutorError> {
    let query = users.filter(user_id.is_not_null());
    let select = SQuery{ select: query, phantom: PhantomData::<User> };
    let usr_list = req.state().rdb.send(select)
        .wait()??;
    println!("{:?}",usr_list);
    Ok(HttpResponse::Found().finish())
}
```

## License

Licensed under either of these:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   https://opensource.org/licenses/MIT)

### Contributing

Unless you explicitly state otherwise, any contribution you intentionally submit
for inclusion in the work, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.

