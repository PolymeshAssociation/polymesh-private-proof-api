use actix_web::web;

use crate::repo::MercatRepository;

mod users;
mod assets;
mod accounts;
mod account_balances;

pub fn service<R: MercatRepository>(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1")
            .configure(users::service::<R>)
            .configure(assets::service::<R>)
            .configure(accounts::service::<R>)
    );
}
