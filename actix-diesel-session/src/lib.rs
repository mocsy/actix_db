mod schema;
mod data;

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate serde_derive;

use diesel::prelude::*;
// use diesel::result::Error as DieselError;

use std::collections::HashMap;
use std::marker::PhantomData;
// use std::env;
use log::debug;

use futures::future::{err as FutErr, ok as FutOk, FutureResult};
use futures::Future;

use ::actix::prelude::*;
use actix_web::{ HttpResponse, HttpRequest, Error};
use actix_web::middleware::identity::RequestIdentity;
use actix_web::middleware::session::{SessionImpl, SessionBackend};
use actix_web::middleware::Response;
use actix_diesel_actor::{ WQuery, DbExecutor };

use crate::schema::user_sessions::dsl::*;
use crate::data::UserSession;

pub struct DbSession {
    wdb: Addr<DbExecutor<diesel::PgConnection>>,
    identity: String,
    changed: bool,
    state: HashMap<String, String>,
}

impl SessionImpl for DbSession {
    fn get(&self, key: &str) -> Option<&str> {   
        if let Some(s) = self.state.get(key) {
            Some(s)
        } else {
            None
        }
    }

    fn set(&mut self, key: &str, value: String) {
        self.changed = true;
        self.state.insert(key.to_owned(), value);
    }

    fn remove(&mut self, key: &str) {
        self.changed = true;
        self.state.remove(key);
    }

    fn clear(&mut self) {
        self.changed = true;
        self.state.clear()
    }

    // 'variable does not need to be mutable' is a lie, the trait requires it
    #[allow(unused_mut)]
    fn write(&self, mut resp: HttpResponse) -> Result<Response, Error> {
        if self.changed {
            let state_json = serde_json::to_string(&self.state)?;
            let state_value: serde_json::Value = serde_json::from_str(&state_json)?;
            let wdb = &self.wdb;

            let sessions = DbSessionBackend::load(wdb.clone(), self.identity.clone());
            if sessions.is_empty() {
                let query = diesel::insert_into(user_sessions)
                    .values((skey.eq(self.identity.clone()), sdata.eq(state_value)));
                let ins = WQuery {
                    query,
                    phantom: PhantomData::<UserSession>,
                };
                let usrs = wdb.send(ins)
                    .wait()
                    .unwrap().unwrap();
                debug!("{:?}", usrs);
            } else {
                let target = user_sessions.filter(skey.eq(self.identity.clone()));
                let query = diesel::update(target)
                    .set(sdata.eq(state_value));
                let upd = WQuery {
                    query,
                    phantom: PhantomData::<UserSession>,
                };
                let usrs = wdb.send(upd)
                    .wait()
                    .unwrap().unwrap();
                debug!("{:?}", usrs);
            }
        }
        Ok(Response::Done(resp))
    }
}

pub struct DbSessionBackend(Addr<DbExecutor<diesel::PgConnection>>);
impl DbSessionBackend {
    pub fn load(wdb: Addr<DbExecutor<diesel::PgConnection>>, identity: String) -> Vec<UserSession> {
        let query = user_sessions.filter(skey.eq(identity));
        let sel = WQuery {
            query,
            phantom: PhantomData::<UserSession>,
        };
        wdb.send(sel)
            .wait()
            .unwrap().unwrap()
    }
}
impl<S> SessionBackend<S> for DbSessionBackend {
    type Session = DbSession;
    type ReadFuture = FutureResult<DbSession, Error>;

    fn from_request(&self, req: &mut HttpRequest<S>) -> Self::ReadFuture {
        if let Some(identity) = req.identity() {
            let sessions = DbSessionBackend::load(self.0.clone(), identity.clone());
            if sessions.len() > 1 {
                return FutErr(actix_web::error::ErrorUnauthorized("Session load inconclusive."));
            }
            if let Some(usr) = sessions.first() {
                let res = serde_json::to_string(&usr.sdata);
                if let Ok(jsn) = res {
                    // self.json = Some(jsn);
                    let state: HashMap<String, String> = serde_json::from_str(&jsn).unwrap_or_default();
                    return FutOk(DbSession{wdb: self.0.clone(), identity, state, changed: false});
                }
            }
        }
        FutErr(actix_web::error::ErrorUnauthorized("Identiy not set."))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
