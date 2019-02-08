CREATE TABLE user_sessions (
  id SERIAL PRIMARY KEY,
  skey TEXT UNIQUE NOT NULL,
  sdata JSONB
);

CREATE UNIQUE INDEX idx_user_sessions_key ON user_sessions (skey);
CREATE INDEX idx_user_sessions_data ON user_sessions USING gin(sdata);
