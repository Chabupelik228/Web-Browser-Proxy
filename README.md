# Архитектурная спецификация: Chabupelik Browser (Tauri + Rust + Vue 3)

## 1. Задумка и ключевые цели

**Chabupelik Browser** — это легковесный портативный настольный браузер, разработанный для обхода сетевых ограничений (включая авторизацию через локальные шлюзы колледжа `10.0.16.32`) с нулевым локальным следом и нативной поддержкой защищенных сервисов (Google Accounts, Cloudflare, WebAuthn, AI-платформы).

### Главные требования:
1. **100% совместимость со сложными веб-сервисами**: Google Auth (`accounts.google.com`), Cloudflare Turnstile, Discord, ChatGPT работают без сбоев за счет использования нативного движка Chromium (**Microsoft Edge WebView2**).
2. **Анти-реверс и защита прокси**: невозможность вытащить постоянный прокси-сервер или использовать его вне браузера. Код туннеля компилируется в машинный код на **Rust**.
3. **Zero-Footprint (Нулевой след)**: локальная история, куки и кэш полностью уничтожаются при закрытии программы.
4. **Zero-Knowledge Cloud Sync**: куки и открытые вкладки шифруются на клиенте ключом **AES-256-GCM** и синхронизируются через VPS. Сервер не имеет доступа к содержимому сессий.
5. **Портативность**: один исполняемый файл `.exe` размером ~**10–15 МБ**, запускаемый с флешки без прав администратора.

---

## 2. Общая архитектура системы

```mermaid
graph TB
    subgraph Client["ПК в колледже (Tauri Desktop App)"]
        subgraph UI["Интерфейс (Vue 3 + Vite)"]
            Tabs["Вкладки и адресная строка"]
            AIShortcuts["Быстрый доступ к AI"]
            AuthModal["Вход (Пароль / Telegram OTP)"]
        end

        subgraph Engine["Chromium Engine (WebView2)"]
            NativeTabs["Нативные веб-страницы (Google, ChatGPT, etc.)"]
        end

        subgraph RustCore["Rust Core (Нативный бинарник)"]
            LocalProxy["Локальный Loopback Прокси (127.0.0.1:PORT)<br/>+ Аутентификация сессии"]
            TunnelClient["Wisp / WSS Зашифрованный туннель"]
            CryptoSync["Шифрование кук (AES-256-GCM)"]
            DeviceFP["Device Fingerprint"]
        end
    end

    subgraph Server["VPS (Docker & Fastify)"]
        AuthAPI["Auth API & OTP Service"]
        Postgres[(PostgreSQL: Зашифрованные куки)]
        RedisCache[(Redis: Активные сессии и токены)]
        WispGateway["Wisp / WSS Proxy Gateway"]
    end

    subgraph TG["Telegram"]
        Bot["Telegram Bot (@chabupelik_bot)"]
    end

    AuthModal -->|1. Запрос OTP| Bot
    AuthModal -->|2. Ввод OTP / Пароля| RustCore
    RustCore -->|3. Авторизация + Device ID| AuthAPI
    AuthAPI -->|4. JWT Access Token (5 мин)| RustCore
    AuthAPI <--> RedisCache
    AuthAPI <--> Postgres
    NativeTabs -->|5. HTTP/HTTPS трафик| LocalProxy
    LocalProxy -->|6. Инкапсуляция с JWT| TunnelClient
    TunnelClient ==>|7. WSS туннель (порт 443)| WispGateway
    WispGateway -->|8. Выход в интернет| World((Интернет))
```

---

## 3. Модель безопасности и защита от вскрытия (Threat Model)

| Потенциальный вектор атаки | Риск | Инженерная защита в Tauri + Rust |
| :--- | :--- | :--- |
| **Декомпиляция и анализ исходников** | Злоумышленник распаковывает бинарник и ищет адрес прокси и ключи. | **Rust Native Compilation**: исходный код компилируется в оптимизированный x86_64 ассемблер (`opt-level = 3`, `lto = true`, `strip = true`). Декомпиляция в читаемый код невозможна. |
| **Сниффинг локального порта (Loopback Sniffing)** | Перехват локального порта `127.0.0.1:PORT` через Wireshark / Proxifier для бесплатного серфинга. | **Dynamic Session Auth**: при старте Rust генерирует случайный 256-битный токен авторизации. WebView2 подключается с заголовком `Proxy-Authorization`. Любые сторонние запросы отбрасываются с кодом `407`. |
| **Вытягивание токенов и вечный серфинг** | Попытка скопировать JWT и подключиться к VPS сторонним клиентом (v2ray / curl). | 1. **Короткий TTL токена**: Access Token живет всего 5 минут.<br/>2. **Redis Single-Session Lock**: VPS разрешает строго 1 активный туннель на пользователя. Старый сокет немедленно рвется.<br/>3. **Idle Timeout (60 сек)**: при отсутствии трафика сокет принудительно уничтожается. |
| **Спуфинг Device Fingerprint** | Подделка серийных номеров устройства для мульти-логина. | Хеш оборудования формируется в нативном коде Rust из CPU ID, UUID материнской платы и MAC-адреса через системные вызовы Windows API (WMI/Win32). |
| **Оставление следов на ПК в колледже** | Утечка паролей, истории и кук на публичном компьютере. | При закрытии Rust вызывает полную очистку данных профиля WebView2 (`ClearBrowsingDataAsync`). Временная папка профиля создается во временной директории с удалением при завершении. |

---

## 4. Схема сквозного шифрования (Zero-Knowledge Sync)

```
[Пользователь] Вводит пароль при входе
      │
      ▼
[KDF: Argon2id / PBKDF2] ──► [AES-256-GCM Ключ] (Только в оперативной памяти Rust)
                                    │
    ┌───────────────────────────────┴───────────────────────────────┐
    │ При запуске                                                  │ При закрытии
    ▼                                                               ▼
[Скачивание зашифрованного Blob с VPS]                   [Экспорт кук и открытых вкладок]
    │                                                               │
[Расшифровка в памяти Rust]                              [Шифрование AES-256-GCM (IV + Payload + Tag)]
    │                                                               │
[Нативная инъекция в WebView2]                           [Отправка зашифрованного Blob на VPS]
                                                                    │
                                                         [Полная зачистка локального диска]
```

---

## 5. Структура проекта

```
Scramjet-Browser/
├── src-tauri/                    # Нативный бэкенд на Rust
│   ├── Cargo.toml                # Зависимости Rust (tauri, tokio, tungstenite, aes-gcm, sysinfo)
│   ├── tauri.conf.json           # Конфигурация окна, безопасности и прав WebView2
│   └── src/
│       ├── main.rs               # Точка входа, жизненный цикл приложения
│       ├── tunnel/
│       │   ├── mod.rs
│       │   ├── client.rs         # Wisp / WebSocket клиент с ротацией JWT
│       │   └── loopback.rs       # Локальный HTTP/SOCKS5 прокси с авторизацией
│       ├── security/
│       │   ├── crypto.rs         # AES-256-GCM шифрование и KDF
│       │   ├── device.rs         # Сбор аппаратного отпечатка (Hardware ID)
│       │   └── session.rs        # Управление памятью токенов и очистка следов
│       ├── cookies/
│       │   └── manager.rs        # Нативное чтение и запись кук WebView2
│       └── commands.rs           # Tauri IPC-команды для взаимодействия с UI
│
├── src/                          # Фронтенд интерфейс (Vue 3 + Vite + Tailwind CSS)
│   ├── index.html
│   ├── package.json
│   ├── vite.config.js
│   ├── src/
│   │   ├── main.js
│   │   ├── App.vue               # Главный контейнер (вкладки + переключение Webview)
│   │   ├── assets/               # Стили Tailwind, шрифты, темная тема
│   │   ├── components/
│   │   │   ├── TabBar.vue        # Панель вкладок с фавиконками
│   │   │   ├── AddressBar.vue    # Адресная строка с индикатором шифрования
│   │   │   ├── AIWorkspace.vue   # Быстрый доступ (ChatGPT, Qwen, Claude, AI Studio)
│   │   │   └── AuthModal.vue     # Форма логина / ввода 6-значного OTP
│   │   └── stores/
│   │       ├── auth.store.js     # Состояние сессии и связь с Rust через invoke()
│   │       └── browser.store.js  # Управление вкладками и историей
│
├── backend/                      # Серверная часть на VPS
│   ├── Dockerfile
│   ├── docker-compose.yml
│   ├── database.sql              # Схема PostgreSQL (таблицы users, sessions_sync)
│   └── src/
│       └── index.js              # Fastify API, валидация OTP/JWT, Wisp шлюз, Redis lock
│
└── telegram_bot/                 # Бот для генерации одноразовых паролей
    ├── Dockerfile
    ├── requirements.txt
    └── bot.py                    # Команды /start, /register, /login (генерация 60s OTP)
```

---

## 6. Технологический стек

### Клиентское приложение:
- **Фреймворк приложения**: Tauri v2
- **Язык ядра**: Rust 2021
- **Асинхронный рантайм**: Tokio (`tokio`, `tokio-tungstenite`, `hyper`)
- **Криптография**: `aes-gcm`, `argon2`, `rand`
- **Движок рендеринга**: Microsoft Edge WebView2 (Chromium Evergreen)
- **UI Фреймворк**: Vue 3 (Composition API, `<script setup>`)
- **Стилизация**: Tailwind CSS (Dark Glassmorphism UI)
- **Сборщик UI**: Vite

### Серверная инфраструктура (VPS):
- **Среда выполнения**: Node.js 22 LTS / Fastify
- **База данных**: PostgreSQL 15
- **In-Memory Cache & Session Lock**: Redis 7
- **Проксирование**: `@mercuryworkshop/wisp-js` (WSS Wisp Server)
- **Бот**: Python 3.11 (`aiogram` / `python-telegram-bot`)

---

## 7. Этапы разработки и внедрения

1. **Шаг 1**: Инициализация окружения Tauri + Vue 3 в рабочей директории.
2. **Шаг 2**: Реализация безопасного туннеля на Rust (`loopback.rs` + `client.rs`) с передачей динамического JWT.
3. **Шаг 3**: Реализация модуля нативного управления куками и локальной очистки (`manager.rs` + `session.rs`).
4. **Шаг 4**: Перенос существующего Vue 3 интерфейса в проект Tauri и привязка IPC (`invoke`).
5. **Шаг 5**: Обновление бэкенда Fastify (Redis single-session lock, idle timeout, валидация OTP).
6. **Шаг 6**: Обновление Telegram-бота (генерация 6-значных кодов `/login`).
7. **Шаг 7**: Сборка и релиз Windows Portable `.exe` (~12 МБ).
