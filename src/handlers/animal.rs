use super::*;

use crate::Animal;

use sqlx::{query, query_as, PgPool};

pub async fn create(animal: Animal, db_pool: &PgPool) -> tide::Result<Animal> {
    let row: Animal = query_as!(
        Animal,
        r#"
        INSERT INTO animals (id, name, weight, diet) VALUES
        ($1, $2, $3, $4) returning id as "id!", name, weight, diet
        "#,
        animal.id,
        animal.name,
        animal.weight,
        animal.diet
    )
    .fetch_one(db_pool)
    .await
    .map_err(|e| Error::new(409, e))?;

    Ok(row)
}
pub async fn list(db_pool: &PgPool) -> tide::Result<Vec<Animal>> {
    let rows = query_as!(
        Animal,
        r#"
        SELECT id, name, weight, diet from animals
        "#
    )
    .fetch_all(db_pool)
    .await
    .map_err(|e| Error::new(409, e))?;

    Ok(rows)
}

pub async fn get(id: Uuid, db_pool: &PgPool) -> tide::Result<Option<Animal>> {
    let row = query_as!(
        Animal,
        r#"
        SELECT  id, name, weight, diet from animals
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| Error::new(409, e))?;

    Ok(row)
}
pub async fn delete(id: Uuid, db_pool: &PgPool) -> tide::Result<Option<()>> {
    let row = query!(
        r#"
        delete from animals
        WHERE id = $1
        returning id
        "#,
        id
    )
    .fetch_optional(db_pool)
    .await
    .map_err(|e| Error::new(409, e))?;

    let r = match row {
        None => None,
        Some(_) => Some(()),
    };

    Ok(r)
}

pub async fn update(id: Uuid, animal: Animal, db_pool: &PgPool) -> tide::Result<Option<Animal>> {
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
    .fetch_optional(db_pool)
    .await
    .map_err(|e| Error::new(409, e))?;

    Ok(row)
}
