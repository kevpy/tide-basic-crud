BEGIN ;
--
-- Add pgcypto extension
--

CREATE EXTENSION IF NOT EXISTS pgcrypto;

--
-- Name: animals; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE animals (
    id uuid DEFAULT gen_random_uuid(),
    name text NOT NULL,
    weight integer NOT NULL,
    diet text NOT NULL
);


ALTER TABLE animals OWNER TO postgres;

--
-- Name: animals animals_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY animals
    ADD CONSTRAINT animals_pkey PRIMARY KEY (id);


--
-- PostgreSQL database dump complete
--

COMMIT ;
