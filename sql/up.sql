CREATE database dinos_db;


CREATE TABLE animals (
    id uuid DEFAULT gen_random_uuid(),
    name text,
    weight integer,
    diet text
);


ALTER TABLE animals OWNER TO postgres;

ALTER TABLE ONLY animals
    ADD CONSTRAINT dinos_pkey PRIMARY KEY (id);