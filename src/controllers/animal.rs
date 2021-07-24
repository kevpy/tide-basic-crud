use super::*;

use tide::{Body, Request, Response};

use crate::handlers;

pub async fn create(mut req: Request<State>) -> tide::Result {
    let animal: Animal = req.body_json().await?;
    let db_pool = req.state().db_pool.clone();

    let row = handlers::animal::create(animal, &db_pool).await?;

    let mut res = Response::new(201);
    res.set_body(Body::from_json(&row)?);
    Ok(res)
}

pub async fn list(req: tide::Request<State>) -> tide::Result {
    let db_pool = req.state().db_pool.clone();
    let rows = handlers::animal::list(&db_pool).await?;

    let mut res = Response::new(200);
    res.set_body(Body::from_json(&rows)?);
    Ok(res)
}

pub async fn get(req: tide::Request<State>) -> tide::Result {
    let db_pool = req.state().db_pool.clone();
    let id: Uuid = Uuid::parse_str(req.param("id")?).unwrap();
    let row = handlers::animal::get(id, &db_pool).await?;

    let res = match row {
        None => Response::new(404),
        Some(row) => {
            let mut r = Response::new(200);
            r.set_body(Body::from_json(&row)?);
            r
        }
    };
    Ok(res)
}

pub async fn update(mut req: tide::Request<State>) -> tide::Result {
    let animal: Animal = req.body_json().await?;
    let db_pool = req.state().db_pool.clone();
    let id: Uuid = Uuid::parse_str(req.param("id")?).unwrap();
    let row = handlers::animal::update(id, animal, &db_pool).await?;

    let res = match row {
        None => Response::new(404),
        Some(row) => {
            let mut r = Response::new(200);
            r.set_body(Body::from_json(&row)?);
            r
        }
    };

    Ok(res)
}

pub async fn delete(req: tide::Request<State>) -> tide::Result {
    let db_pool = req.state().db_pool.clone();
    let id: Uuid = Uuid::parse_str(req.param("id")?).unwrap();
    let row = handlers::animal::delete(id, &db_pool).await?;

    let res = match row {
        None => Response::new(404),
        Some(_) => Response::new(204),
    };

    Ok(res)
}
