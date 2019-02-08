# Actix Diesel Actor
> Packages diesel queries into actix messages

Provides message types to package diesel queries into actix messages.
Provides actors to act on these messeages.

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