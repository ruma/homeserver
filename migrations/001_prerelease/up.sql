CREATE TABLE access_tokens (
  id BIGSERIAL PRIMARY KEY,
  user_id TEXT NOT NULL,
  value TEXT NOT NULL,
  revoked BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMP NOT NULL DEFAULT now(),
  updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE account_data (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    data_type TEXT NOT NULL,
    content TEXT NOT NULL,
    UNIQUE (user_id, data_type)
);

CREATE TABLE events (
  id TEXT NOT NULL PRIMARY KEY,
  ordering BIGSERIAL NOT NULL,
  room_id TEXT NOT NULL,
  user_id TEXT NOT NULL,
  event_type TEXT NOT NULL,
  state_key TEXT,
  content TEXT NOT NULL,
  extra_content TEXT,
  created_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE room_aliases (
  alias TEXT NOT NULL PRIMARY KEY,
  room_id TEXT NOT NULL,
  user_id TEXT NOT NULL,
  servers TEXT[] NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT now(),
  updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE room_memberships(
    event_id TEXT NOT NULL PRIMARY KEY,
    room_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    sender TEXT NOT NULL,
    membership TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE rooms (
  id TEXT NOT NULL PRIMARY KEY,
  user_id TEXT NOT NULL,
  public BOOLEAN NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE users (
  id TEXT NOT NULL PRIMARY KEY,
  password_hash TEXT NOT NULL,
  active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMP NOT NULL DEFAULT now(),
  updated_at TIMESTAMP NOT NULL DEFAULT now()
);
