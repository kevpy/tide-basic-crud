use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

use async_std::sync::RwLock;
use serde::{Deserialize, Serialize};
use tide::{Body, Request, Response};

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    let state = State {
        dinos: Default::default(),
    };

    tide::log::start();
    let mut app = tide::with_state(state);

    // default route
    app.at("/").get(|_| async { Ok("Hello, world!") });

    // post a dino
    app.at("/dinos").post(|mut req: Request<State>| async move {
        let dino: Dino = req.body_json().await?;

        // let get a mut ref of our store ( hashMap )
        let mut dinos = req.state().dinos.write().await;
        dinos.insert(String::from(&dino.name), dino.clone());

        let mut res = Response::new(201);
        res.set_body(Body::from_json(&dino)?);
        Ok(res)
    });

    // get all dinos
    app.at("/dinos").get(|req: Request<State>| async move {
        let dinos = req.state().dinos.read().await;

        // get all the dinos as a vector
        let dinos_vec: Vec<Dino> = dinos.values().cloned().collect();
        let mut res = Response::new(200);
        res.set_body(Body::from_json(&dinos_vec)?);
        Ok(res)
    });

    app.at("/dinos/:name")
        // get a single dino
        .get(|req: Request<State>| async move {
            let mut dinos = req.state().dinos.write().await;
            let key: String = req.param("name")?.to_string();
            let res = match dinos.entry(key) {
                Entry::Vacant(_entry) => Response::new(404),
                Entry::Occupied(entry) => {
                    let mut res = Response::new(200);
                    res.set_body(Body::from_json(&entry.get())?);
                    res
                }
            };
            Ok(res)
        })
        // update a dino
        .put(|mut req: Request<State>| async move {
            let dino_update: Dino = req.body_json().await?;
            let mut dinos = req.state().dinos.write().await;
            let key: String = req.param("name")?.to_string();

            let res = match dinos.entry(key) {
                Entry::Vacant(_entry) => Response::new(404),
                Entry::Occupied(mut entry) => {
                    *entry.get_mut() = dino_update;
                    let mut res = Response::new(200);
                    res.set_body(Body::from_json(&entry.get())?);
                    res
                }
            };
            Ok(res)
        })
        // delete a dino
        .delete(|req: Request<State>| async move {
            let mut dinos = req.state().dinos.write().await;
            let key: String = req.param("name")?.to_string();
            let deleted = dinos.remove(&key);
            let res = match deleted {
                None => Response::new(404),
                Some(_) => Response::new(204),
            };
            Ok(res)
        });

    // update a dino

    app.listen("127.0.0.1:8080").await?;
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Dino {
    name: String,
    weight: u16,
    diet: String,
}

#[derive(Clone, Debug)]
struct State {
    dinos: Arc<RwLock<HashMap<String, Dino>>>,
}
