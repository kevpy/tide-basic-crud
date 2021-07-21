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
struct Animal {
    id: Option<Uuid>,
    name: Option<String>,
    weight: Option<i32>,
    diet: Option<String>,
}

// struct Animal {
//     id: Uuid,
//     name: String,
//     weight: i32,
//     diet: String,
// }
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
async fn list_animals() -> tide::Result<()> {
    dotenv::dotenv().ok();

    // let animal = Animal {
    //     id: Some(Uuid::new_v4()),
    //     name: Some(String::from("test_list")),
    //     weight: Some(500),
    //     diet: Some(String::from("carnivorous")),
    // };

    let db_pool = make_db_pool().await;
    let app = server(db_pool).await;

    let res = surf::Client::with_http_client(app)
        .get("https://example.com/animals")
        .await?;

    assert_eq!(200, res.status());
    Ok(())
}

#[async_std::test]
async fn create_animal() -> tide::Result<()> {
    dotenv::dotenv().ok();

    let animal = Animal {
        id: Some(Uuid::new_v4()),
        name: Some(String::from("test_create")),
        weight: Some(500),
        diet: Some(String::from("carnivorous")),
    };

    let db_pool = make_db_pool().await;
    let app = server(db_pool).await;

    let res = surf::Client::with_http_client(app)
        .post("https://example.com/animals")
        .body(serde_json::to_string(&animal)?)
        .await?;

    assert_eq!(201, res.status());

    Ok(())
}

#[async_std::test]
async fn get_animal() -> tide::Result<()> {
    dotenv::dotenv().ok();

    let animal = Animal {
        id: Some(Uuid::new_v4()),
        name: Some(String::from("test_get")),
        weight: Some(500),
        diet: Some(String::from("carnivorous")),
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

    let res = surf::Client::with_http_client(app)
        .get(format!(
            "https://example.com/animals/{}",
            &animal.id.unwrap()
        ))
        .await?;

    assert_eq!(200, res.status());
    Ok(())
}

#[async_std::test]
async fn update_animal() -> tide::Result<()> {
    dotenv::dotenv().ok();

    let mut animal = Animal {
        id: Some(Uuid::new_v4()),
        name: Some(String::from("test_update")),
        weight: Some(500),
        diet: Some(String::from("carnivorous")),
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

    // change the animal
    animal.name = Some(String::from("updated from test"));

    // start the server
    let app = server(db_pool).await;

    let res = surf::Client::with_http_client(app)
        .put(format!(
            "https://example.com/animals/{}",
            &animal.id.unwrap()
        ))
        .body(serde_json::to_string(&animal)?)
        .await?;

    assert_eq!(200, res.status());
    Ok(())
}

#[async_std::test]
async fn delete_animal() -> tide::Result<()> {
    dotenv::dotenv().ok();

    let animal = Animal {
        id: Some(Uuid::new_v4()),
        name: Some(String::from("test_delete")),
        weight: Some(500),
        diet: Some(String::from("carnivorous")),
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

    let res = surf::Client::with_http_client(app)
        .delete(format!(
            "https://example.com/animals/{}",
            &animal.id.unwrap()
        ))
        .await?;

    assert_eq!(204, res.status());
    Ok(())
}
