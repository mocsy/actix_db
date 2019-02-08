use diesel::prelude::*;
use diesel::r2d2;
use diesel::r2d2::{ ConnectionManager, Pool };
use diesel::result::Error;
use diesel::query_builder::SelectQuery;
use ::actix::prelude::*;
use diesel::query_dsl::LoadQuery;
use std::marker::PhantomData;
use std::env;
use dotenv::dotenv;

use log::debug;

pub struct DbExecutor<T: 'static> where T: Connection {
    pub pool: Pool<ConnectionManager<T>>
}
impl<T: Connection> Actor for DbExecutor<T> {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
       debug!("I am alive!");
    }
}

pub struct SQuery<S,I> {
    pub select : S,
    pub phantom: PhantomData<I>,
}
impl<S,I:'static> Message for SQuery<S,I>{
    type Result = Result<Vec<I>, Error>;
}
impl<S: LoadQuery<PgConnection, I>+SelectQuery,I:'static> Handler<SQuery<S,I>> for DbExecutor<PgConnection> {
    type Result = Result<Vec<I>, Error>;

    fn handle(&mut self, msg: SQuery<S,I>, _: &mut Self::Context) -> Self::Result {
        let pool = &self.pool;
        debug!("handle Queries");
        if let Ok(conn) = pool.get() {
            let res = msg.select.load::<I>(&conn);
            return res;
        }
        Ok(Vec::new())
    }
}

pub struct WQuery<W,I> {
    pub query : W,
    pub phantom: PhantomData<I>,
}
impl<W,I:'static> Message for WQuery<W,I>{
    type Result = Result<Vec<I>, Error>;
}
impl<W: LoadQuery<PgConnection, I>,I:'static> Handler<WQuery<W,I>> for DbExecutor<PgConnection> {
    type Result = Result<Vec<I>, Error>;

    fn handle(&mut self, msg: WQuery<W,I>, _: &mut Self::Context) -> Self::Result {
        let pool = &self.pool;
        debug!("handle Queries");
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
    let database_url = env::var(var)
        .unwrap_or_else(|_| panic!("{} must be set", var));
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create pool.");
    SyncArbiter::start(3, move || DbExecutor{pool:pool.clone()})
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
