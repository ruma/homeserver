CREATE TABLE room_memberships(
    event_id TEXT NOT NULL PRIMARY KEY,
    room_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    sender TEXT NOT NULL,
    membership TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);

