-- Системный пользователь «Администратор» для десктопа (quick 261117-w2q).
-- Только схема: строка создаётся идемпотентно при старте (AuthService::ensure_system_admin),
-- чтобы не ломать БД/тесты, ожидающие пустую users до первичной настройки.
ALTER TABLE users ADD COLUMN is_system INTEGER NOT NULL DEFAULT 0;

CREATE UNIQUE INDEX uq_users_single_system ON users(is_system) WHERE is_system = 1;

PRAGMA user_version = 53;
