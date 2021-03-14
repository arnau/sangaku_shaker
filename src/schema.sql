CREATE TABLE IF NOT EXISTS entry (
  ordinal text NOT NULL PRIMARY KEY,
  parent  text,
  ancestor  NUMBER NOT NULL,
  slug    text NOT NULL,
  title   text NOT NULL,
  difficulty NUMBER,
  content text NOT NULL
);
