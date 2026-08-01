CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Таблица пользователей (менеджеров/студентов)
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(50) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Таблица синхронизации зашифрованных кук (Zero-Knowledge)
CREATE TABLE IF NOT EXISTS sessions_sync (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    encrypted_cookies TEXT, -- Зашифрованный массив кук (AES-GCM-256)
    open_tabs JSONB,        -- Список открытых вкладок для восстановления сессии
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Таблица зашифрованных сообщений чата
CREATE TABLE IF NOT EXISTS chat_messages (
    id SERIAL PRIMARY KEY,
    sender_id UUID REFERENCES users(id) ON DELETE CASCADE,
    recipient_id UUID REFERENCES users(id) ON DELETE SET NULL, -- NULL означает общий чат
    encrypted_text TEXT NOT NULL,                             -- Зашифрованное сообщение
    iv VARCHAR(255) NOT NULL,                                 -- Вектор инициализации AES
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);