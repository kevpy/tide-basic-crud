use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{query, query_as, FromRow, PgPool, Row};
use tide::{Body, Request, Response, Server};
use uuid::Uuid;

#[derive(Clone, Debug)]
struct State {
    db_pool: PgPool,
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
// struct Animal {
//     id: Option<Uuid>,
//     name: Option<String>,
//     weight: Option<i32>,
//     diet: Option<String>,
// }

struct Animal {
    id: Uuid,
    name: String,
    weight: i32,
    diet: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AnimalRequest {
    name: String,
    weight: i32,
    diet: String,
}

struct RestEntity {
    base_path: String,
}

impl RestEntity {
    async fn create(mut req: Request<State>) -> tide::Result {
        let dino: AnimalRequest = req.body_json().await?;

        let db_pool = req.state().db_pool.clone();

        let row = query(
            r#"
            INSERT INTO animals (name, weight, diet) 
                VALUES
                ($1, $2, $3) 
            returning id, name, weight, diet
            "#,
        )
        // .bind(&dino.id)
        .bind(&dino.name)
        .bind(&dino.weight)
        .bind(&dino.diet)
        .map(|row: PgRow| Animal {
            id: row.get(0),
            name: row.get(1),
            weight: row.get(2),
            diet: row.get(3),
        })
        .fetch_one(&db_pool)
        .await?;

        let mut res = Response::new(201);
        res.set_body(Body::from_json(&row)?);
        Ok(res)
    }

    async fn list(req: tide::Request<State>) -> tide::Result {
        let mut animals = vec![];

        let db_pool = req.state().db_pool.clone();

        let rows = query(
            r#"
                SELECT id, name, weight, diet
                    FROM animals
                ORDER BY weight
            "#,
        )
        .fetch_all(&db_pool)
        .await?;

        for row in rows {
            animals.push(Animal {
                id: row.get(0),
                name: row.get(1),
                weight: row.get(2),
                diet: row.get(3),
            });
        }

        let mut res = Response::new(200);
        res.set_body(Body::from_json(&animals)?);
        Ok(res)
    }

    async fn get(req: tide::Request<State>) -> tide::Result {
        let db_pool = req.state().db_pool.clone();
        let id: Uuid = Uuid::parse_str(req.param("id")?).unwrap();

        let row = query_as!(
            Animal,
            r#"
            SELECT  id, name, weight, diet from animals
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&db_pool)
        .await?;

        let res = match row {
            None => {
                let mut r = Response::new(404);
                r.set_body(Body::from_string("Animal not found".to_string()));
                r
            }
            Some(row) => {
                let mut r = Response::new(200);
                r.set_body(Body::from_json(&row)?);
                r
            }
        };

        Ok(res)
    }

    async fn update(mut req: tide::Request<State>) -> tide::Result {
        let animal: Animal = req.body_json().await?;
        let db_pool = req.state().db_pool.clone();
        let id: Uuid = Uuid::parse_str(req.param("id")?).unwrap();
        let row = query_as!(
            Animal,
            r#"
            UPDATE animals SET name = $2, weight = $3, diet = $4
                WHERE id = $1
            returning id, name, weight, diet
            "#,
            id,
            animal.name,
            animal.weight,
            animal.diet
        )
        .fetch_optional(&db_pool)
        .await?;

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

    async fn delete(req: Request<State>) -> tide::Result {
        let db_pool = req.state().db_pool.clone();
        let id: Uuid = Uuid::parse_str(req.param("id")?).unwrap();
        let row = query!(
            r#"
            delete from animals
                WHERE id = $1
            returning id
            "#,
            id
        )
        .fetch_optional(&db_pool)
        .await?;

        let res = match row {
            None => Response::new(404),
            Some(_) => Response::new(204),
        };
        Ok(res)
    }
}

fn register_rest_entity(app: &mut Server<State>, entity: RestEntity) {
    app.at(&entity.base_path)
        .get(RestEntity::list)
        .post(RestEntity::create);

    app.at(&format!("{}/:id", entity.base_path))
        .get(RestEntity::get)
        .put(RestEntity::update)
        .delete(RestEntity::delete);
}

pub async fn make_db_pool() -> PgPool {
    let db_url = std::env::var("DATABASE_URL").unwrap();
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .unwrap()
}

#[async_std::main]
async fn main() {
    dotenv::dotenv().ok();

    tide::log::start();
    let db_pool = make_db_pool().await;
    let app = server(db_pool).await;

    app.listen("127.0.0.1:8080").await.unwrap();
}

async fn server(db_pool: PgPool) -> Server<State> {
    let state = State { db_pool };

    let mut app = tide::with_state(state);

    // default route
    app.at("/").get(|_| async { Ok("ok") });

    let dinos_endpoint = RestEntity {
        base_path: String::from("/animals"),
    };

    register_rest_entity(&mut app, dinos_endpoint);

    app
}

#[async_std::test]
async fn list_dinos() -> tide::Result<()> {
    dotenv::dotenv().ok();
    use tide::http::{Method, Request, Response, Url};

    let animal = Animal {
        id: Uuid::new_v4(),
        name: String::from("test_list"),
        weight: 500,
        diet: String::from("carnivorous"),
    };

    let db_pool = make_db_pool().await;
    let app = server(db_pool).await;

    let url = Url::parse("http://example.com/animals").unwrap();
    let req = Request::new(Method::Get, url);
    let res: Response = app.respond(req).await?;

    assert_eq!(200, res.status());
    Ok(())
}

#[async_std::test]
async fn create_dino() -> tide::Result<()> {
    dotenv::dotenv().ok();
    use tide::http::{Method, Request, Response, Url};

    let animal = Animal {
        id: Uuid::new_v4(),
        name: String::from("test_create"),
        weight: 500,
        diet: String::from("carnivorous"),
    };

    let db_pool = make_db_pool().await;
    let app = server(db_pool).await;

    let url = Url::parse("https://example.com/animals").unwrap();
    let mut req = Request::new(Method::Post, url);
    req.set_body(serde_json::to_string(&animal)?);
    let res: Response = app.respond(req).await?;

    assert_eq!(201, res.status());

    Ok(())
}

#[async_std::test]
async fn get_dino() -> tide::Result<()> {
    dotenv::dotenv().ok();
    use tide::http::{Method, Request, Response, Url};

    let animal = Animal {
        id: Uuid::new_v4(),
        name: String::from("test_get"),
        weight: 500,
        diet: String::from("carnivorous"),
    };

    let db_pool = make_db_pool().await;

    // create the dino for get
    query!(
        r#"
        INSERT INTO animals (id, name, weight, diet) VALUES
        ($1, $2, $3, $4) returning id, name, weight, diet
        "#,
        animal.id,
        animal.name,
        animal.weight,
        animal.diet
    )
    .fetch_one(&db_pool)
    .await?;

    // start the server
    let app = server(db_pool).await;

    let url = Url::parse(format!("https://example.com/animals/{}", &animal.id).as_str()).unwrap();
    let req = Request::new(Method::Get, url);

    let res: Response = app.respond(req).await?;
    assert_eq!(200, res.status());
    Ok(())
}

#[async_std::test]
async fn update_dino() -> tide::Result<()> {
    dotenv::dotenv().ok();
    use tide::http::{Method, Request, Response, Url};

    let mut animal = Animal {
        id: Uuid::new_v4(),
        name: String::from("test_update"),
        weight: 500,
        diet: String::from("carnivorous"),
    };

    let db_pool = make_db_pool().await;

    // create the dino for update
    query!(
        r#"
        INSERT INTO animals (id, name, weight, diet) VALUES
        ($1, $2, $3, $4) returning id, name, weight, diet
        "#,
        animal.id,
        animal.name,
        animal.weight,
        animal.diet
    )
    .fetch_one(&db_pool)
    .await?;

    // change the dino
    animal.name = String::from("updated from test");

    // start the server
    let app = server(db_pool).await;

    let url = Url::parse(format!("https://example.com/animals/{}", &animal.id).as_str()).unwrap();
    let mut req = Request::new(Method::Put, url);
    let dinos_as_json_string = serde_json::to_string(&animal)?;
    req.set_body(dinos_as_json_string);
    let res: Response = app.respond(req).await?;
    assert_eq!(200, res.status());
    Ok(())
}

#[async_std::test]
async fn delete_dino() -> tide::Result<()> {
    dotenv::dotenv().ok();
    use tide::http::{Method, Request, Response, Url};

    let animal = Animal {
        id: Uuid::new_v4(),
        name: String::from("test_delete"),
        weight: 500,
        diet: String::from("carnivorous"),
    };

    let db_pool = make_db_pool().await;

    // create the dino for delete
    query!(
        r#"
        INSERT INTO animals (id, name, weight, diet) VALUES
        ($1, $2, $3, $4) returning id, name, weight, diet
        "#,
        animal.id,
        animal.name,
        animal.weight,
        animal.diet
    )
    .fetch_one(&db_pool)
    .await?;

    // start the server
    let app = server(db_pool).await;

    let url = Url::parse(format!("https://example.com/animals/{}", &animal.id).as_str()).unwrap();
    let req = Request::new(Method::Delete, url);
    let res: Response = app.respond(req).await?;
    assert_eq!(204, res.status());
    Ok(())
}
