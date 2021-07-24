use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tera::Tera;
use tide::listener::Listener;
use tide::{Error, Server};
use tide_tera::prelude::*;
use uuid::Uuid;

mod controllers;
mod handlers;

use controllers::animal;
use controllers::views;

#[derive(Clone, Debug)]
pub struct State {
    db_pool: PgPool,
    tera: Tera,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Animal {
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

pub async fn make_db_pool(db_url: &str) -> PgPool {
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

    let db_url = std::env::var("DATABASE_URL").unwrap();
    let db_pool = make_db_pool(&db_url).await;
    let app = server(db_pool).await;

    let mut listener = app
        .bind("127.0.0.1:8080")
        .await
        .expect("can't bind the port");

    for info in listener.info().iter() {
        println!("Server listening on {}", info);
    }
    listener.accept().await.unwrap();
}

async fn server(db_pool: PgPool) -> Server<State> {
    let mut tera = Tera::new("templates/**/*").expect("Error parsing templates directory");
    tera.autoescape_on(vec!["html"]);

    let state = State { db_pool, tera };

    let mut app = tide::with_state(state);

    // views
    app.at("/").get(views::index);
    app.at("/animals/new").get(views::new);
    app.at("/animals/:id/edit").get(views::edit);

    // api
    app.at("/animals").get(animal::list).post(animal::create);

    app.at("animals/:id")
        .get(animal::get)
        .put(animal::update)
        .delete(animal::delete);

    // serve static files
    app.at("/public")
        .serve_dir("./public")
        .expect("Invalid static file directory");

    app
}

#[cfg(test)]
mod tests {
    use super::*;
    use lazy_static::lazy_static;
    use sqlx::query;

    lazy_static! {
        static ref DB_URL: String =
            std::env::var("DATABASE_URL").expect("missing env var DATABASE_URL");
    }

    async fn clear_animals() -> Result<(), Box<dyn std::error::Error>> {
        let db_pool = make_db_pool(&DB_URL).await;

        sqlx::query("DELETE FROM animals").execute(&db_pool).await?;
        Ok(())
    }

    #[test]
    fn clear() {
        dotenv::dotenv().ok();
        async_std::task::block_on(async {
            clear_animals().await.unwrap();
            ()
        })
    }

    #[async_std::test]
    async fn list_animals() -> tide::Result<()> {
        dotenv::dotenv().ok();
        // clear_animals()
        //     .await
        //     .expect("Failed to clear the animals table");

        let db_pool = make_db_pool(&DB_URL).await;
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

        use assert_json_diff::assert_json_eq;

        let animal = Animal {
            id: Uuid::new_v4(),
            name: String::from("test_create"),
            weight: 500,
            diet: String::from("carnivorous"),
        };

        let db_pool = make_db_pool(&DB_URL).await;
        let app = server(db_pool).await;

        let mut res = surf::Client::with_http_client(app)
            .post("https://example.com/animals")
            .body(serde_json::to_string(&animal)?)
            .await?;

        assert_eq!(201, res.status());

        let a: Animal = res.body_json().await?;
        assert_json_eq!(animal.name, a.name);

        Ok(())
    }

    #[async_std::test]
    async fn create_animal_with_existing_id() -> tide::Result<()> {
        dotenv::dotenv().ok();

        let animal = Animal {
            id: Uuid::new_v4(),
            name: String::from("test_existing_id"),
            weight: 500,
            diet: String::from("carnivorous"),
        };

        let db_pool = make_db_pool(&DB_URL).await;

        // create the animal
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

        let res = surf::Client::with_http_client(app.clone())
            .post("https://example.com/animals")
            .body(serde_json::to_string(&animal)?)
            .await?;

        // let res1 = surf::Client::with_http_client(app)
        //     .post("https://example.com/animals")
        //     .body(serde_json::to_string(&animal)?)
        //     .await?;

        assert_eq!(409, res.status());

        Ok(())
    }

    #[async_std::test]
    async fn get_animal() -> tide::Result<()> {
        dotenv::dotenv().ok();

        use assert_json_diff::assert_json_eq;

        let animal = Animal {
            id: Uuid::new_v4(),
            name: String::from("test_get"),
            weight: 500,
            diet: String::from("carnivorous"),
        };

        let db_pool = make_db_pool(&DB_URL).await;

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

        let mut res = surf::Client::with_http_client(app)
            .get(format!("https://example.com/animals/{}", &animal.id))
            .await?;

        assert_eq!(200, res.status());

        let a: Animal = res.body_json().await?;
        assert_json_eq!(animal, a);
        Ok(())
    }

    #[async_std::test]
    async fn get_animal_non_existing_id() -> tide::Result<()> {
        dotenv::dotenv().ok();

        // start the server
        let db_pool = make_db_pool(&DB_URL).await;
        let app = server(db_pool).await;

        let res = surf::Client::with_http_client(app)
            .get(format!("https://example.com/animals/{}", &Uuid::new_v4()))
            .await?;

        assert_eq!(404, res.status());

        Ok(())
    }

    #[async_std::test]
    async fn update_animal() -> tide::Result<()> {
        dotenv::dotenv().ok();

        use assert_json_diff::assert_json_eq;

        let mut animal = Animal {
            id: Uuid::new_v4(),
            name: String::from("test_get"),
            weight: 500,
            diet: String::from("carnivorous"),
        };

        let db_pool = make_db_pool(&DB_URL).await;

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
        animal.name = String::from("updated from test");

        // start the server
        let app = server(db_pool).await;

        let mut res = surf::Client::with_http_client(app)
            .put(format!("https://example.com/animals/{}", &animal.id))
            .body(serde_json::to_string(&animal)?)
            .await?;

        assert_eq!(200, res.status());

        let a: Animal = res.body_json().await?;
        assert_json_eq!(animal, a);

        Ok(())
    }

    #[async_std::test]
    async fn updatet_animal_non_existing_id() -> tide::Result<()> {
        dotenv::dotenv().ok();

        let animal = Animal {
            id: Uuid::new_v4(),
            name: String::from("test_update"),
            weight: 500,
            diet: String::from("carnivorous"),
        };

        // start the server
        let db_pool = make_db_pool(&DB_URL).await;
        let app = server(db_pool).await;

        let res = surf::Client::with_http_client(app)
            .put(format!("https://example.com/animals/{}", &animal.id))
            .body(serde_json::to_string(&animal)?)
            .await?;

        assert_eq!(404, res.status());

        Ok(())
    }

    #[async_std::test]
    async fn delete_animal() -> tide::Result<()> {
        dotenv::dotenv().ok();

        let animal = Animal {
            id: Uuid::new_v4(),
            name: String::from("test_get"),
            weight: 500,
            diet: String::from("carnivorous"),
        };

        let db_pool = make_db_pool(&DB_URL).await;

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
            .delete(format!("https://example.com/animals/{}", &animal.id))
            .await?;

        assert_eq!(204, res.status());
        Ok(())
    }

    #[async_std::test]
    async fn delete_animal_non_existing_id() -> tide::Result<()> {
        dotenv::dotenv().ok();

        // start the server
        let db_pool = make_db_pool(&DB_URL).await;
        let app = server(db_pool).await;

        let res = surf::Client::with_http_client(app)
            .delete(format!("https://example.com/animals/{}", &Uuid::new_v4()))
            .await?;

        assert_eq!(404, res.status());

        Ok(())
    }
}
