INSERT INTO description_element_type VALUES(7,'Binary');

CREATE TABLE description_binary (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  mode BYTEA
);

CREATE TABLE top_n_binary (
  id SERIAL PRIMARY KEY,
  description_id INTEGER NOT NULL REFERENCES column_description (id),
  ranking BIGINT,
  value BYTEA,
  count BIGINT,
  UNIQUE(description_id, ranking)
);
