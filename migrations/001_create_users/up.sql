CREATE TABLE users (
  id TEXT NOT NULL PRIMARY KEY,
  password_hash TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT now(),
  updated_at TIMESTAMP NOT NULL DEFAULT now()
);
