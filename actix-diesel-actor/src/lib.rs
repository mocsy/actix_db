use ::actix::prelude::*;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::{QueryFragment, QueryId, SelectQuery};
use diesel::query_dsl::{LoadQuery, RunQueryDsl};
use diesel::r2d2;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::result::Error;
use dotenv::dotenv;
use std::env;
use std::marker::PhantomData;

use log::debug;

pub struct DbExecutor<T: 'static>
where
    T: Connection,
{
    pub pool: Pool<ConnectionManager<T>>,
}
impl<T: Connection> Actor for DbExecutor<T> {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("I am alive!");
    }
}

/// let query = users.filter(group_id.eq(group.id));
/// let query = query.order((lname.asc(), fname.asc()));
/// let select = SQuery {
///     select: query,
///     phantom: PhantomData::<Users>,
/// };
/// let res = req.state().rdb.send(select).wait().ok().unwrap();
pub struct SQuery<S, I> {
    pub select: S,
    pub phantom: PhantomData<I>,
}
impl<S, I: 'static> Message for SQuery<S, I> {
    type Result = Result<Vec<I>, Error>;
}
impl<S: LoadQuery<PgConnection, I> + SelectQuery + QueryFragment<Pg>, I: 'static>
    Handler<SQuery<S, I>> for DbExecutor<PgConnection>
{
    type Result = Result<Vec<I>, Error>;

    fn handle(&mut self, msg: SQuery<S, I>, _: &mut Self::Context) -> Self::Result {
        let dbg = diesel::debug_query(&msg.select);
        debug!("{:?}", dbg);
        let pool = &self.pool;
        if let Ok(conn) = pool.get() {
            let res = msg.select.load::<I>(&conn);
            return res;
        }
        Ok(Vec::new())
    }
}

// let target = users.filter(id.eq(uid));
// let query = diesel::update(target).set((
//     fname.eq(form.fname),
//     lname.eq(form.lname),
//     email.eq(form.email),
//     phone.eq(form.phone),
//     comment.eq(form.comment),
// ));
// let upd = WQuery {
//     query,
//     phantom: PhantomData::<Users>,
// };
// let res = req.state().wdb.send(upd).wait().ok().unwrap();
pub struct WQuery<W, I> {
    pub query: W,
    pub phantom: PhantomData<I>,
}
impl<W, I: 'static> Message for WQuery<W, I> {
    type Result = Result<Vec<I>, Error>;
}
impl<W: LoadQuery<PgConnection, I> + QueryFragment<Pg>, I: 'static> Handler<WQuery<W, I>>
    for DbExecutor<PgConnection>
{
    type Result = Result<Vec<I>, Error>;

    fn handle(&mut self, msg: WQuery<W, I>, _: &mut Self::Context) -> Self::Result {
        let dbg = diesel::debug_query(&msg.query);
        debug!("{:?}", dbg);
        let pool = &self.pool;
        if let Ok(conn) = pool.get() {
            let res = msg.query.get_results::<I>(&conn);
            return res;
        }
        Ok(Vec::new())
    }
}

pub struct AppState {
    pub rdb: Addr<DbExecutor<diesel::PgConnection>>,
    pub wdb: Addr<DbExecutor<diesel::PgConnection>>,
}

/// Use the Read setting with a connection String to access a read-only db replica
/// Use the Write setting to udpate with a connection String to a writeable DB
/// There is a distinct r2d2 pool for each thread the DbExecutor actor runs on.
/// Therefore the pool is configured with max_size(3) and min_idle(Some(0)):
/// It creates a maximum of 3 connection per pool, starting with 0.
pub enum ConnectionType {
    Read,
    Write,
}
pub fn db_setup(conn_type: ConnectionType) -> actix::Addr<DbExecutor<diesel::PgConnection>> {
    dotenv().ok();

    let var = match conn_type {
        ConnectionType::Read => "DB_READ_URL",
        ConnectionType::Write => "DB_WRITE_URL",
    };
    let database_url = env::var(var).unwrap_or_else(|_| panic!("{} must be set", var));
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .max_size(3)
        .min_idle(Some(0))
        .build(manager)
        .expect("Failed to create pool.");
    SyncArbiter::start(3, move || DbExecutor { pool: pool.clone() })
}

#[derive(Debug)]
pub enum DbExecutorError {
    DatabaseError(diesel::result::Error),
    MailBoxError(actix::MailboxError),
    Unknown,
}
impl From<diesel::result::Error> for DbExecutorError {
    fn from(error: diesel::result::Error) -> Self {
        DbExecutorError::DatabaseError(error)
    }
}
impl From<actix::MailboxError> for DbExecutorError {
    fn from(error: actix::MailboxError) -> Self {
        DbExecutorError::MailBoxError(error)
    }
}

/// This message can be used to ask for an r2d2 pg connection.
/// Generally, try to avoid it.
/// let conn = req.state().wdb.send(Conn{}).wait().ok().unwrap().unwrap();
/// let res= diesel::delete(users.filter(id.eq(uid))).execute(&conn);
pub struct Conn;
impl Message for Conn {
    type Result = Result<r2d2::PooledConnection<ConnectionManager<PgConnection>>, Error>;
}
impl Handler<Conn> for DbExecutor<PgConnection> {
    type Result = Result<r2d2::PooledConnection<ConnectionManager<PgConnection>>, Error>;

    fn handle(&mut self, _msg: Conn, _: &mut Self::Context) -> Self::Result {
        // fn handle(&mut self, msg: Conn, _: &mut Context<Self>) -> Self::Result {
        let pool = &self.pool;
        if let Ok(conn) = pool.get() {
            return Ok(conn);
        }
        Err(Error::NotFound)
    }
}

/// Devised to be used for deleting things, but allows other RunQueryDsl queries as well.
/// let query =  diesel::delete(users.filter(id.eq(uid)));
/// let del = DQuery {
///     query
/// };
/// let res = req.state().wdb.send(del).wait().ok().unwrap();
pub struct DQuery<D> {
    pub query: D,
}
impl<D> Message for DQuery<D> {
    type Result = Result<usize, Error>;
}
impl<D: RunQueryDsl<PgConnection> + QueryId + QueryFragment<Pg>> Handler<DQuery<D>>
    for DbExecutor<PgConnection>
{
    type Result = Result<usize, Error>;

    fn handle(&mut self, msg: DQuery<D>, _: &mut Self::Context) -> Self::Result {
        let dbg = diesel::debug_query(&msg.query);
        debug!("{:?}", dbg);
        let pool = &self.pool;
        if let Ok(conn) = pool.get() {
            let res = msg.query.execute(&conn);
            return res;
        }
        Err(Error::NotFound)
    }
}
