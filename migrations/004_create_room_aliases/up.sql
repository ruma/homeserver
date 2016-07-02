CREATE TABLE room_aliases (
  alias TEXT NOT NULL PRIMARY KEY,
  room_id TEXT NOT NULL,
  servers TEXT[] NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT now(),
  updated_at TIMESTAMP NOT NULL DEFAULT now()
);
