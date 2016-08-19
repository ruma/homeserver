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
