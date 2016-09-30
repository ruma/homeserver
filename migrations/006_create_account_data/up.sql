CREATE TABLE account_data (
    id BIGSERIAL PRIMARY KEY,
    user_id TEXT NOT NULL,
    data_type TEXT NOT NULL,
    content TEXT NOT NULL,
    UNIQUE (user_id, data_type)
);
