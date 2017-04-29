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
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE (ordering)
);

CREATE TABLE filters (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    content TEXT NOT NULL,
    UNIQUE (id, user_id)
);

CREATE TABLE presence_status (
    user_id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL,
    presence TEXT NOT NULL,
    status_msg TEXT,
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE  presence_list (
    user_id TEXT NOT NULL,
    observed_user_id TEXT NOT NULL,
    PRIMARY KEY (user_id, observed_user_id)
);

CREATE TABLE profiles (
    id TEXT NOT NULL PRIMARY KEY,
    avatar_url TEXT,
    displayname TEXT,
    UNIQUE(id)
);

CREATE TABLE pushers (
    user_id TEXT NOT NULL,
    lang TEXT NOT NULL,
    kind TEXT NOT NULL,
    url TEXT,
    device_display_name TEXT NOT NULL,
    app_id TEXT NOT NULL,
    profile_tag TEXT,
    pushkey TEXT NOT NULL,
    app_display_name TEXT NOT NULL,
    PRIMARY KEY (user_id, app_id)
);

CREATE TABLE room_account_data (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    room_id TEXT NOT NULL,
    data_type TEXT NOT NULL,
    content TEXT NOT NULL,
    UNIQUE (user_id, room_id, data_type)
);

CREATE TABLE room_aliases (
    alias TEXT NOT NULL PRIMARY KEY,
    room_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    servers TEXT[] NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE room_memberships (
    event_id TEXT NOT NULL PRIMARY KEY,
    room_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    sender TEXT NOT NULL,
    membership TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    UNIQUE(room_id, user_id)
);

CREATE TABLE room_tags (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    room_id TEXT NOT NULL,
    tag     TEXT NOT NULL,
    content TEXT NOT NULL,
    UNIQUE (user_id, room_id, tag)
);

CREATE TABLE rooms (
    id TEXT NOT NULL PRIMARY KEY,
    user_id TEXT NOT NULL,
    public BOOLEAN NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE transactions (
    path TEXT NOT NULL,
    access_token TEXT NOT NULL,
    response TEXT NOT NULL,
    PRIMARY KEY (path, access_token)
);

CREATE TABLE users (
    id TEXT NOT NULL PRIMARY KEY,
    password_hash TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    updated_at TIMESTAMP NOT NULL DEFAULT now()
);
