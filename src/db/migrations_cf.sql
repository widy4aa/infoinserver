-- cloudflare_config
CREATE TABLE IF NOT EXISTS cloudflare_config (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- Only 1 row allowed
    account_id TEXT NOT NULL,
    tunnel_id TEXT NOT NULL,
    api_token TEXT NOT NULL
);
