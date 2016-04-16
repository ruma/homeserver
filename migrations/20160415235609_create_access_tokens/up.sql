CREATE TABLE access_tokens (
  id BIGSERIAL PRIMARY KEY,
  user_id TEXT NOT NULL,
  value TEXT NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT now()
);
