// backend/src/index.js
import { join } from "node:path";
import { hostname } from "node:os";
import { createServer } from "node:http";
import { fileURLToPath } from "url";
import { server as wisp, logging } from "@mercuryworkshop/wisp-js/server";
import Fastify from "fastify";
import fastifyCors from "@fastify/cors";
import fastifyStatic from "@fastify/static";
import pg from "pg";
import { createClient } from "redis";
import jwt from "jsonwebtoken";
import argon2 from "argon2";
import crypto from "node:crypto";

const publicPath = fileURLToPath(new URL("../public/", import.meta.url));

// Хранилище активных Wisp сокетов: userId -> Set(socket)
const activeWispSockets = new Map();

// Инициализация БД и Redis
const { Pool } = pg;
const db = new Pool({ connectionString: process.env.DATABASE_URL });
const redis = createClient({ url: process.env.REDIS_URL });

redis.on("error", (err) => console.error("[REDIS ERROR]", err));
await redis.connect().catch((err) => {
	console.error("[REDIS CONNECT ERROR]", err);
});

// Настройки Wisp-проксирования
logging.set_level(logging.NONE);
Object.assign(wisp.options, {
	allow_udp_streams: false,
	hostname_blacklist: [/example\.com/],
	// Remote DNS: строго внешние DNS, чтобы шлюз колледжа (10.0.16.32) не видел запросов
	dns_servers: ["1.1.1.1", "8.8.8.8", "94.140.14.14", "94.140.15.15"],
});

const fastify = Fastify({
	serverFactory: (handler) => {
		return createServer()
			.on("request", (req, res) => {
				const url = req.url || "";
				const isApiOrStatic = (
					url.startsWith("/api/") ||
					url.startsWith("/scram/") ||
					url.startsWith("/libcurl/") ||
					url.startsWith("/baremux/") ||
					url.startsWith("/wisp/")
				);

				if (!isApiOrStatic) {
					res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
					res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
				}

				handler(req, res);
			})
			.on("upgrade", async (req, socket, head) => {
				if (socket.__wispHandled) {
					console.warn("[WISP GATEWAY] Повторный upgrade на сокет — игнорируем");
					return;
				}

				if (req.url.startsWith("/wisp/")) {
					const segments = req.url.split("/").filter(Boolean);
					const token = segments[1] ? decodeURIComponent(segments[1]) : null;

					if (!token) {
						socket.write("HTTP/1.1 401 Unauthorized\r\n\r\n");
						socket.destroy();
						return;
					}

					// Защита от сторонних WISP-клиентов и Replay-атак
					const clientTimestamp = req.headers["x-wisp-timestamp"];
					const clientSignature = req.headers["x-wisp-signature"];

					if (!clientTimestamp || !clientSignature) {
						console.warn("[WISP AUTH] Отсутствует подпись или timestamp");
						socket.write("HTTP/1.1 403 Forbidden\r\n\r\n");
						socket.destroy();
						return;
					}

					const ts = Number(clientTimestamp);
					if (!Number.isFinite(ts) || isNaN(ts) || Math.abs(Date.now() / 1000 - ts) > 30) {
						console.warn("[WISP AUTH] Replay attack или истекший timestamp");
						socket.write("HTTP/1.1 403 Forbidden\r\n\r\n");
						socket.destroy();
						return;
					}

					const expectedSignature = crypto.createHash('sha256')
						.update(token + clientTimestamp + process.env.WISP_SALT)
						.digest('hex');

					if (clientSignature.length !== expectedSignature.length) {
						socket.write("HTTP/1.1 403 Forbidden\r\n\r\n");
						socket.destroy();
						return;
					}

					const isValid = crypto.timingSafeEqual(
						Buffer.from(clientSignature, 'hex'),
						Buffer.from(expectedSignature, 'hex')
					);

					if (!isValid) {
						console.warn("[WISP AUTH] Невалидная HMAC подпись");
						socket.write("HTTP/1.1 403 Forbidden\r\n\r\n");
						socket.destroy();
						return;
					}

					let decoded;
					try {
						decoded = jwt.verify(token, process.env.JWT_SECRET);
					} catch (e) {
						socket.write("HTTP/1.1 403 Forbidden\r\n\r\n");
						socket.destroy();
						return;
					}

					const userId = decoded.id;
					const deviceHash = decoded.deviceHash || "unknown";

					// 1. Проверка активности сессии в Redis (Single-Session Enforcement)
					try {
						const activeSession = await redis.get(`session:${userId}`);
						if (activeSession) {
							const sessionData = JSON.parse(activeSession);
							// Если пришел запрос с другим хэшем
							if (sessionData.deviceHash && sessionData.deviceHash !== deviceHash) {
								console.warn(`[WISP AUTH] Отклонен сокет для user ${userId}: несовпадение deviceHash`);
								socket.write("HTTP/1.1 403 Forbidden\r\n\r\n");
								socket.destroy();
								return;
							}
						}
					} catch (redisErr) {
						console.error("[WISP REDIS ERROR]", redisErr);
					}

					// 2. Закрываем предыдущие активные сокеты этого пользователя (Single-Connection Lock)
					const existingSockets = activeWispSockets.get(userId);
					if (existingSockets && existingSockets.size > 0) {
						for (const oldSocket of existingSockets) {
							try {
								oldSocket.destroy();
							} catch (_) { }
						}
						activeWispSockets.delete(userId);
					}

					// Регистрация сокета
					socket.__wispHandled = true;
					const userSocketSet = activeWispSockets.get(userId) || new Set();
					userSocketSet.add(socket);
					activeWispSockets.set(userId, userSocketSet);

					socket.on("close", () => {
						const set = activeWispSockets.get(userId);
						if (set) {
							set.delete(socket);
							if (set.size === 0) activeWispSockets.delete(userId);
						}
					});

					try {
						console.log(`[WISP GATEWAY] Wisp-туннель активен для user_id: ${userId} (${deviceId})`);
						req.url = "/";
						wisp.routeRequest(req, socket, head);
					} catch (e) {
						console.error("[WISP ROUTE ERROR]", e);
						try { socket.destroy(); } catch (_) { }
					}
					return;
				}
				socket.end();
			});
	},
});

fastify.register(fastifyCors, {
	origin: true,
	methods: ["GET", "POST", "PUT", "DELETE", "OPTIONS"],
	allowedHeaders: ["Content-Type", "Authorization", "x-bot-token"],
	credentials: true,
});

// Middleware проверки Bearer JWT токена
const checkAuth = async (req, reply) => {
	const authHeader = req.headers["authorization"];
	const token = authHeader && authHeader.split(" ")[1];
	if (!token) {
		reply.code(401).send({ status: "error", message: "Токен отсутствует" });
		return;
	}
	try {
		req.user = jwt.verify(token, process.env.JWT_SECRET);
	} catch (err) {
		reply.code(403).send({ status: "error", message: "Невалидный токен" });
	}
};

process.on("uncaughtException", (err) => {
	console.error("[BACKEND CRITICAL] Uncaught exception:", err);
});

process.on("unhandledRejection", (reason) => {
	console.error("[BACKEND CRITICAL] Unhandled rejection:", reason);
});

// ----------------------------------------------------
// 1. РЕГИСТРАЦИЯ ПОЛЬЗОВАТЕЛЯ (через Telegram-бота)
// ----------------------------------------------------
fastify.post("/api/auth/register", async (req, reply) => {
	const botToken = req.headers["x-bot-token"];
	if (botToken !== process.env.BOT_TOKEN) {
		return reply.code(403).send({ error: "Доступ запрещен" });
	}
	const { username, password } = req.body || {};
	if (!username || !password) {
		return reply.code(400).send({ error: "Логин и пароль обязательны" });
	}

	try {
		const passwordHash = await argon2.hash(password);
		const result = await db.query(
			"INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING id, username",
			[username, passwordHash]
		);
		return { status: "ok", user: result.rows[0] };
	} catch (err) {
		return reply.code(500).send({ error: "Пользователь уже существует" });
	}
});

// ----------------------------------------------------
// 2. ГЕНЕРАЦИЯ ОДНОРАЗОВОГО 6-ЗНАЧНОГО OTP (через Telegram-бота)
// ----------------------------------------------------
fastify.post("/api/auth/otp/generate", async (req, reply) => {
	const botToken = req.headers["x-bot-token"];
	if (botToken !== process.env.BOT_TOKEN) {
		return reply.code(403).send({ error: "Доступ запрещен" });
	}

	const { username } = req.body || {};
	if (!username) {
		return reply.code(400).send({ error: "Username обязателен" });
	}

	try {
		const result = await db.query("SELECT id, username, is_active FROM users WHERE username = $1", [username]);
		const user = result.rows[0];
		if (!user || !user.is_active) {
			return reply.code(404).send({ error: "Пользователь не найден или заблокирован" });
		}

		// Генерация 6-значного числового кода (100000 - 999999)
		const otpCode = crypto.randomInt(100000, 999999).toString();

		// Сохраняем в Redis с TTL 120 секунд
		await redis.set(`otp:${otpCode}`, JSON.stringify({
			userId: user.id,
			username: user.username,
		}), { EX: 120 });

		console.log(`[AUTH OTP] Сгенерирован OTP для @${username} (действителен 120с)`);
		return { status: "ok", code: otpCode, expiresIn: 120 };
	} catch (err) {
		console.error("[OTP GENERATE ERROR]", err);
		return reply.code(500).send({ error: "Ошибка генерации OTP" });
	}
});

// ----------------------------------------------------
// 3. ВХОД ПО ОДНОРАЗОВОМУ OTP (из браузера)
// ----------------------------------------------------
fastify.post("/api/auth/otp/verify", async (req, reply) => {
	const { code, device_id } = req.body || {};
	if (!code) {
		return reply.code(400).send({ status: "error", message: "Код обязателен" });
	}

	try {
		const raw = await redis.get(`otp:${code}`);
		if (!raw) {
			return reply.code(400).send({ status: "error", message: "Неверный или истекший код" });
		}

		const data = JSON.parse(raw);
		// Одноразовый код: удаляем сразу после успешной валидации
		await redis.del(`otp:${code}`);

		const userId = data.userId;
		const username = data.username;
		const deviceId = device_id || "desktop-unknown";

		const deviceHash = crypto
			.createHmac('sha256', process.env.JWT_SECRET)
			.update(deviceId)
			.digest('hex');

		// Фиксация активной сессии в Redis (Single-Session Lock)
		await redis.set(`session:${userId}`, JSON.stringify({
			deviceHash,
			updatedAt: Date.now(),
		}), { EX: 7 * 24 * 60 * 60 });

		// Короткоживущий AccessToken (15 минут) с привязкой к хэшу Device ID
		const accessToken = jwt.sign(
			{ id: userId, username, deviceHash },
			process.env.JWT_SECRET,
			{ expiresIn: "15m" }
		);

		const refreshToken = jwt.sign(
			{ id: userId, deviceHash },
			process.env.JWT_SECRET,
			{ expiresIn: "30d" }
		);

		return {
			status: "ok",
			accessToken,
			refreshToken,
			user: { id: userId, username },
		};
	} catch (err) {
		console.error("[OTP VERIFY ERROR]", err);
		return reply.code(500).send({ status: "error", message: "Ошибка сервера" });
	}
});

// ----------------------------------------------------
// 4. СТАНДАРТНЫЙ ВХОД ПО ЛОГИНУ И ПАРОЛЮ
// ----------------------------------------------------
fastify.post("/api/auth/login", async (req, reply) => {
	const { username, password, device_id } = req.body || {};
	if (!username || !password) {
		return reply.code(400).send({ status: "error", message: "Неверные данные" });
	}

	try {
		const result = await db.query("SELECT * FROM users WHERE username = $1", [username]);
		const user = result.rows[0];
		if (!user || !user.is_active || !(await argon2.verify(user.password_hash, password))) {
			return reply.code(400).send({ status: "error", message: "Неверный логин или пароль" });
		}

		const deviceId = device_id || "desktop-unknown";

		const deviceHash = crypto
			.createHmac('sha256', process.env.JWT_SECRET)
			.update(deviceId)
			.digest('hex');

		// Фиксация активной сессии в Redis
		await redis.set(`session:${user.id}`, JSON.stringify({
			deviceHash,
			updatedAt: Date.now(),
		}), { EX: 30 * 24 * 60 * 60 });

		const accessToken = jwt.sign(
			{ id: user.id, username: user.username, deviceHash },
			process.env.JWT_SECRET,
			{ expiresIn: "15m" }
		);

		const refreshToken = jwt.sign(
			{ id: user.id, deviceHash },
			process.env.JWT_SECRET,
			{ expiresIn: "30d" }
		);

		return {
			status: "ok",
			accessToken,
			refreshToken,
			user: { id: user.id, username: user.username },
		};
	} catch (err) {
		console.error("[AUTH LOGIN ERROR]", err);
		return reply.code(500).send({ status: "error", message: "Ошибка сервера" });
	}
});

// ----------------------------------------------------
// 5. РОТАЦИЯ ТОКЕНОВ (REFRESH)
// ----------------------------------------------------
fastify.post("/api/auth/refresh", async (req, reply) => {
	const { refresh_token, device_id } = req.body || {};
	const cookieToken = req.headers.cookie?.match(/refreshToken=([^;]+)/)?.[1];
	const tokenToVerify = refresh_token || cookieToken;

	if (!tokenToVerify) {
		return reply.code(401).send({ status: "error", message: "Refresh token отсутствует" });
	}

	try {
		const decoded = jwt.verify(tokenToVerify, process.env.JWT_SECRET);
		const result = await db.query("SELECT id, username, is_active FROM users WHERE id = $1", [decoded.id]);
		const user = result.rows[0];

		if (!user || !user.is_active) {
			return reply.code(403).send({ status: "error", message: "Сессия недействительна" });
		}

		if (!device_id || !decoded.deviceHash) {
			console.warn(`[AUTH REFRESH] Попытка рефреша: отсутствует device_id или deviceHash`);
			return reply.code(403).send({ status: "error", message: "Несовпадение аппаратного идентификатора" });
		}

		const expectedHash = crypto
			.createHmac('sha256', process.env.JWT_SECRET)
			.update(device_id)
			.digest('hex');

		const isValid = expectedHash.length === decoded.deviceHash.length && crypto.timingSafeEqual(Buffer.from(expectedHash, 'hex'), Buffer.from(decoded.deviceHash, 'hex'));

		if (!isValid) {
			console.warn(`[AUTH REFRESH] Попытка рефреша с несовпадающим device_id.`);
			return reply.code(403).send({ status: "error", message: "Несовпадение аппаратного идентификатора" });
		}
		const deviceHash = decoded.deviceHash;

		// Проверка Single-Session Lock
		const sessionRaw = await redis.get(`session:${user.id}`);
		if (sessionRaw) {
			const sessionData = JSON.parse(sessionRaw);
			if (sessionData.deviceHash && sessionData.deviceHash !== deviceHash) {
				return reply.code(403).send({ status: "error", message: "Сессия открыта на другом устройстве" });
			}
		}

		// Обновляем метку времени сессии
		await redis.set(`session:${user.id}`, JSON.stringify({
			deviceHash,
			updatedAt: Date.now(),
		}), { EX: 30 * 24 * 60 * 60 });

		const newAccessToken = jwt.sign(
			{ id: user.id, username: user.username, deviceHash },
			process.env.JWT_SECRET,
			{ expiresIn: "15m" }
		);

		const newRefreshToken = jwt.sign(
			{ id: user.id, deviceHash },
			process.env.JWT_SECRET,
			{ expiresIn: "30d" }
		);

		return {
			status: "ok",
			accessToken: newAccessToken,
			refreshToken: newRefreshToken,
			user: { id: user.id, username: user.username },
		};
	} catch (err) {
		return reply.code(403).send({ status: "error", message: "Невалидный refresh token" });
	}
});

// ----------------------------------------------------
// 6. ВЫХОД И ПРИНУДИТЕЛЬНЫЙ СБРОС ТУННЕЛЕЙ
// ----------------------------------------------------
fastify.post("/api/auth/logout", async (req, reply) => {
	await checkAuth(req, reply);
	if (reply.sent) return;

	const userId = req.user.id;

	// Удаляем сессию из Redis
	await redis.del(`session:${userId}`).catch(() => { });

	// Принудительно рвем все Wisp сокеты пользователя
	const sockets = activeWispSockets.get(userId);
	if (sockets) {
		for (const s of sockets) {
			try { s.destroy(); } catch (_) { }
		}
		activeWispSockets.delete(userId);
	}

	console.log(`[AUTH LOGOUT] Сессия и сокеты пользователя ${userId} сброшены`);
	return { status: "ok" };
});



// ----------------------------------------------------
// СТАТИЧЕСКИЕ МАРШРУТЫ ДЛЯ СОВМЕСТИМОСТИ
// ----------------------------------------------------
fastify.register(fastifyStatic, {
	root: publicPath,
	decorateReply: true,
});

fastify.setNotFoundHandler((req, reply) => {
	return reply.code(404).type("text/html").sendFile("404.html");
});

fastify.server.on("listening", () => {
	const address = fastify.server.address();
	console.log(`[BACKEND] Listening on http://0.0.0.0:${address.port}`);
});

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);

function shutdown() {
	console.log("[BACKEND] Получен сигнал завершения. Закрываем соединения...");
	fastify.close();
	db.end();
	redis.quit();
	process.exit(0);
}

let port = parseInt(process.env.PORT || "");
if (isNaN(port)) port = 8080;

fastify.listen({
	port: port,
	host: "0.0.0.0",
});