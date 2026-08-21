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
import crypto from "node:crypto";
import ipRangeCheck from "ip-range-check";
import { WebSocketServer, WebSocket } from "ws";

const publicPath = fileURLToPath(new URL("../public/", import.meta.url));

// Хранилище активных Wisp сокетов: userId -> Set(socket)
const activeWispSockets = new Map();

// --- Multi-Domain Distribution ---
const rootDomain = process.env.ROOT_DOMAIN || "chabupelik.su";
const WISP_DOMAINS = [
	{ domain: `stream.${rootDomain}`, weight: 3 },  // тяжелый трафик
	{ domain: `cdn.${rootDomain}`,    weight: 3 },  // тяжелый трафик
	{ domain: `sync.${rootDomain}`,   weight: 2 },  // средний
	{ domain: `sub.${rootDomain}`,    weight: 1 },  // легкий
	{ domain: `telemetry.${rootDomain}`, weight: 1 }, // легкий
];

// userId -> { domain, sessionBytes, connectedAt }
const activeUserDomains = new Map();
// In-memory traffic counters: userId -> { session: number, delta: number }
const trafficCounters = new Map();

// userId -> Array of FastifyReply objects
const activeSseClients = new Map();

// SSE Heartbeat (every 20s) to keep proxy connections alive
setInterval(() => {
	for (const [userId, clients] of activeSseClients.entries()) {
		for (const res of clients) {
			try {
				res.raw.write(': ping\n\n');
			} catch (_) {}
		}
	}
}, 20000);

// Flush traffic deltas to Redis every 5 seconds
setInterval(async () => {
	for (const [userId, counter] of trafficCounters.entries()) {
		if (counter.delta > 0) {
			const delta = counter.delta;
			counter.delta = 0;
			try {
				await redis.incrBy(`traffic:${userId}:total`, delta);
			} catch (_) {}
		}
	}
}, 5000);

// Инициализация БД и Redis
const { Pool } = pg;
const db = new Pool({ connectionString: process.env.DATABASE_URL });
const redis = createClient({ url: process.env.REDIS_URL });

redis.on("error", (err) => console.error("[REDIS ERROR]", err));
await redis.connect().catch((err) => {
	console.error("[REDIS CONNECT ERROR]", err);
});

// Инициализация списков из БД
let allowedIps = [];
let allowedTgIds = new Map();

const client = await db.connect();
try {
	await client.query(`
		CREATE TABLE IF NOT EXISTS whitelists (
			id SERIAL PRIMARY KEY,
			type VARCHAR(10) NOT NULL,
			value VARCHAR(64) NOT NULL,
			name VARCHAR(255),
			created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
			UNIQUE(type, value)
		)
	`);
	// Автомиграция для старых баз
	await client.query(`ALTER TABLE whitelists ADD COLUMN IF NOT EXISTS name VARCHAR(255)`);
	await client.query(`ALTER TABLE users ADD COLUMN IF NOT EXISTS telegram_id VARCHAR(64)`);
} finally {
	client.release();
}

async function loadWhitelists() {
	try {
		const result = await db.query("SELECT type, value, name FROM whitelists");
		const ips = [];
		const tgs = new Map();
		for (const row of result.rows) {
			if (row.type === 'ip') ips.push(row.value);
			if (row.type === 'tg_id') tgs.set(row.value, row.name || null);
		}
		allowedIps = ips;
		allowedTgIds = tgs;
		console.log(`[FIREWALL] Loaded ${ips.length} IPs and ${tgs.size} TG IDs.`);
	} catch (e) {
		console.error("[FIREWALL] Error loading whitelists:", e);
	}
}
await loadWhitelists();

// Проверка IP адреса на соответствие белому списку
function isIpAllowed(clientIp) {
	if (allowedIps.includes('all')) return true;
	if (allowedIps.length === 0) return false;
	return ipRangeCheck(clientIp, allowedIps);
}

// Настройка Wisp-сервера
logging.set_level(logging.NONE);
Object.assign(wisp.options, {
	allow_udp_streams: false,
	hostname_blacklist: [/example\.com/],
	dns_servers: ["1.1.1.1", "8.8.8.8", "94.140.14.14", "94.140.15.15"],
});

const internalWispServer = createServer();
internalWispServer.on("upgrade", (req, socket, head) => {
	wisp.routeRequest(req, socket, head);
});
internalWispServer.listen(11339, '127.0.0.1', () => {
	console.log("[INTERNAL] Internal Wisp Server listening on 127.0.0.1:11339");
});

const fastify = Fastify({
	serverFactory: (handler) => {
		return createServer()
			.on("request", (req, res) => {
				const url = req.url || "";
				const isApiOrStatic = (
					url.startsWith("/api/") ||
					url.startsWith("/v1/bundle/") ||
					url.startsWith("/v1/transport/") ||
					url.startsWith("/v1/mux/") ||
					url.startsWith("/v1/live/")
				);

				if (!isApiOrStatic) {
					res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
					res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
				}

				// Проверка IP адреса перед обработкой запроса
				let clientIp = req.headers['x-forwarded-for'] || req.socket.remoteAddress;
				if (clientIp && typeof clientIp === 'string') clientIp = clientIp.split(',')[0].trim();

				const botToken = req.headers["x-bot-token"];
				const isBot = botToken && process.env.BOT_TOKEN && botToken.length === process.env.BOT_TOKEN.length && crypto.timingSafeEqual(Buffer.from(botToken), Buffer.from(process.env.BOT_TOKEN));

				if (!isBot && !isIpAllowed(clientIp)) {
					console.warn(`[FIREWALL HTTP] Blocked request from IP: ${clientIp}`);
					res.statusCode = 403;
					res.end(JSON.stringify({ error: "Access denied by IP whitelist" }));
					return;
				}

				handler(req, res);
			})
			.on("upgrade", async (req, socket, head) => {
				if (socket.__wispHandled) {
					console.warn("[WISP GATEWAY] Повторный upgrade на сокет — игнорируем");
					return;
				}

				let clientIp = req.headers['x-forwarded-for'] || socket.remoteAddress;
				if (clientIp && typeof clientIp === 'string') clientIp = clientIp.split(',')[0].trim();

				if (!isIpAllowed(clientIp)) {
					console.warn(`[FIREWALL WISP] Blocked upgrade from IP: ${clientIp}`);
					socket.write("HTTP/1.1 403 Forbidden\r\n\r\n");
					socket.destroy();
					return;
				}

				const liveMatch = req.url.match(/\/v1\/live\/([^/?]+)/);
				if (liveMatch) {
					const token = decodeURIComponent(liveMatch[1]);

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
						console.log(`[WISP GATEWAY] Wisp-туннель активен для user_id: ${userId} (${deviceHash})`);
						
						const wss = new WebSocketServer({ noServer: true });
						wss.handleUpgrade(req, socket, head, (clientWs) => {
							const aesKey = crypto.createHash('sha256').update(process.env.WISP_SALT).digest();
							
							const internalWs = new WebSocket("ws://127.0.0.1:11339/");
							const messageBuffer = [];

							// Init in-memory traffic counter for this user
							if (!trafficCounters.has(userId)) {
								trafficCounters.set(userId, { session: 0, delta: 0 });
							} else {
								trafficCounters.get(userId).session = 0;
							}
							
							const pingInterval = setInterval(() => {
								if (clientWs.readyState === WebSocket.OPEN) {
									clientWs.ping();
								}
							}, 30000);

							clientWs.on("message", (data) => {
								if (!Buffer.isBuffer(data) || data.length < 28) return;
								try {
									const iv = data.subarray(0, 12);
									const authTag = data.subarray(data.length - 16);
									const ciphertext = data.subarray(12, data.length - 16);
									
									const decipher = crypto.createDecipheriv('aes-256-gcm', aesKey, iv);
									decipher.setAuthTag(authTag);
									let plaintext = decipher.update(ciphertext);
									plaintext = Buffer.concat([plaintext, decipher.final()]);
									
									if (internalWs.readyState === WebSocket.OPEN) {
										internalWs.send(plaintext);
									} else if (internalWs.readyState === WebSocket.CONNECTING) {
										messageBuffer.push(plaintext);
									}
								} catch (err) {
									console.error("[WISP DECRYPT ERROR]", err.message);
								}
								// Count incoming traffic (from client)
								const tc = trafficCounters.get(userId);
								if (tc) { tc.session += data.length; tc.delta += data.length; }
							});

							internalWs.on("open", () => {
								messageBuffer.forEach(msg => internalWs.send(msg));
								messageBuffer.length = 0;
							});
							
							internalWs.on("message", (data) => {
								if (!Buffer.isBuffer(data)) return;
								try {
									const iv = crypto.randomBytes(12);
									const cipher = crypto.createCipheriv('aes-256-gcm', aesKey, iv);
									let ciphertext = cipher.update(data);
									ciphertext = Buffer.concat([ciphertext, cipher.final()]);
									const authTag = cipher.getAuthTag();
									
									const payload = Buffer.concat([iv, ciphertext, authTag]);
									
									if (clientWs.readyState === WebSocket.OPEN) {
										clientWs.send(payload);
									}
								} catch (err) {
									console.error("[WISP ENCRYPT ERROR]", err.message);
								}
								// Count outgoing traffic (to client)
								const tc2 = trafficCounters.get(userId);
								if (tc2) { tc2.session += data.length; tc2.delta += data.length; }
							});
							
							clientWs.on("close", () => {
								clearInterval(pingInterval);
								internalWs.close();
								// Flush remaining traffic delta to Redis
								const tc3 = trafficCounters.get(userId);
								if (tc3 && tc3.delta > 0) {
									redis.incrBy(`traffic:${userId}:total`, tc3.delta).catch(() => {});
									tc3.delta = 0;
								}
								// Clean up activeUserDomains
								activeUserDomains.delete(userId);
							});
							internalWs.on("close", () => clientWs.close());
							clientWs.on("error", () => internalWs.close());
							internalWs.on("error", () => clientWs.close());
						});
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



// Middleware для E2EE шифрования API
fastify.addHook('preHandler', async (req, reply) => {
	if (req.headers['x-encrypted-payload'] === '1' && req.body) {
		try {
			const aesKey = crypto.createHash('sha256').update(process.env.WISP_SALT).digest();
			let data = req.body;
			if (typeof data === 'string') data = Buffer.from(data, 'utf-8');
			
			if (Buffer.isBuffer(data)) {
				const iv = data.subarray(0, 12);
				const authTag = data.subarray(data.length - 16);
				const ciphertext = data.subarray(12, data.length - 16);

				const decipher = crypto.createDecipheriv('aes-256-gcm', aesKey, iv);
				decipher.setAuthTag(authTag);
				let plaintext = decipher.update(ciphertext);
				plaintext = Buffer.concat([plaintext, decipher.final()]);

				req.body = JSON.parse(plaintext.toString('utf-8'));
				req.raw.encryptedResponseRequested = true;
			}
		} catch (err) {
			console.error("[API DECRYPT ERROR]", err);
			return reply.code(400).send({ error: "Failed to decrypt payload" });
		}
	}
});

fastify.addHook('onSend', async (req, reply, payload) => {
	if (req.raw.encryptedResponseRequested && payload) {
		try {
			const aesKey = crypto.createHash('sha256').update(process.env.WISP_SALT).digest();
			const iv = crypto.randomBytes(12);
			const cipher = crypto.createCipheriv('aes-256-gcm', aesKey, iv);
			
			const dataToEncrypt = typeof payload === 'string' ? payload : JSON.stringify(payload);
			let ciphertext = cipher.update(Buffer.from(dataToEncrypt, 'utf-8'));
			ciphertext = Buffer.concat([ciphertext, cipher.final()]);
			const authTag = cipher.getAuthTag();

			const encryptedPayload = Buffer.concat([iv, ciphertext, authTag]);
			reply.header('x-encrypted-payload', '1');
			reply.header('content-type', 'application/octet-stream');
			return encryptedPayload;
		} catch (err) {
			console.error("[API ENCRYPT ERROR]", err);
		}
	}
	return payload;
});

// Добавляем поддержку raw body для application/octet-stream
fastify.addContentTypeParser('application/octet-stream', { parseAs: 'buffer' }, (req, body, done) => {
	done(null, body);
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
// 0. НАЗНАЧЕНИЕ ПОДДОМЕНА ДЛЯ WISP-ТУННЕЛЯ
// ----------------------------------------------------
fastify.post("/api/tunnel/assign", async (req, reply) => {
	await checkAuth(req, reply);
	if (reply.sent) return;

	const userId = req.user.id;

	// Already assigned? Return existing
	const existing = activeUserDomains.get(userId);
	if (existing) {
		return { status: "ok", domain: existing.domain };
	}

	// Count active users per domain
	const domainCounts = {};
	for (const d of WISP_DOMAINS) domainCounts[d.domain] = 0;
	for (const [, info] of activeUserDomains) {
		if (domainCounts[info.domain] !== undefined) domainCounts[info.domain]++;
	}

	// Pick domains with the lowest score, then pick randomly among them
	let bestDomains = [];
	let bestScore = Infinity;
	for (const d of WISP_DOMAINS) {
		const score = domainCounts[d.domain] / d.weight;
		if (score < bestScore) {
			bestScore = score;
			bestDomains = [d.domain];
		} else if (score === bestScore) {
			bestDomains.push(d.domain);
		}
	}
	const bestDomain = bestDomains[Math.floor(Math.random() * bestDomains.length)];

	activeUserDomains.set(userId, {
		domain: bestDomain,
		sessionBytes: 0,
		connectedAt: Date.now(),
	});

	console.log(`[DOMAIN ASSIGN] User ${userId} -> ${bestDomain}`);
	return { status: "ok", domain: bestDomain };
});

// ----------------------------------------------------
// 0.5 SSE COMMAND CHANNEL & ADMIN DISCONNECT
// ----------------------------------------------------
fastify.get("/api/events", async (req, reply) => {
	await checkAuth(req, reply);
	if (reply.sent) return;

	const userId = req.user.id;

	reply.raw.setHeader('Content-Type', 'text/event-stream');
	reply.raw.setHeader('Cache-Control', 'no-cache');
	reply.raw.setHeader('Connection', 'keep-alive');
	reply.raw.setHeader('Access-Control-Allow-Origin', '*');
	
	// Отправляем первое сообщение чтобы соединение сразу установилось
	reply.raw.write('data: {"cmd":"connected"}\n\n');

	if (!activeSseClients.has(userId)) {
		activeSseClients.set(userId, []);
	}
	activeSseClients.get(userId).push(reply);

	req.raw.on('close', () => {
		const clients = activeSseClients.get(userId);
		if (clients) {
			const idx = clients.indexOf(reply);
			if (idx !== -1) clients.splice(idx, 1);
			if (clients.length === 0) activeSseClients.delete(userId);
		}
	});

	// Держим соединение открытым
	await new Promise(() => {}); 
});

fastify.post("/api/admin/command", async (req, reply) => {
	const botToken = req.headers["x-bot-token"];
	if (botToken !== process.env.BOT_TOKEN) {
		return reply.code(403).send({ error: "Доступ запрещен" });
	}

	const { user_id, command } = req.body || {};
	if (!user_id || !command) {
		return reply.code(400).send({ error: "user_id и command обязательны" });
	}

	// 1. Send SSE event to frontend
	const clients = activeSseClients.get(user_id);
	if (clients && clients.length > 0) {
		for (const res of clients) {
			try {
				res.raw.write(`data: ${JSON.stringify({ cmd: command })}\n\n`);
			} catch (_) {}
		}
	}

	// 2. If 'norm', gracefully allow time for SSE to reach frontend before killing WISP
	if (command === 'norm') {
		const sockets = activeWispSockets.get(user_id);
		if (sockets) {
			setTimeout(() => {
				for (const s of sockets) {
					try { s.destroy(); } catch (_) {}
				}
				activeWispSockets.delete(user_id);
			}, 3000);
		}
	}

	return { status: "ok" };
});

// ----------------------------------------------------
// 1. РЕГИСТРАЦИЯ ПОЛЬЗОВАТЕЛЯ (через Telegram-бота)
// ----------------------------------------------------
fastify.post("/api/auth/register", async (req, reply) => {
	const botToken = req.headers["x-bot-token"];
	if (botToken !== process.env.BOT_TOKEN) {
		return reply.code(403).send({ error: "Доступ запрещен" });
	}
	let { username, user_id } = req.body || {};
	
	if (user_id && allowedTgIds.has(String(user_id))) {
		const assignedName = allowedTgIds.get(String(user_id));
		if (assignedName) {
			username = assignedName;
		}
	}

	if (!username) {
		return reply.code(400).send({ error: "Логин обязателен" });
	}

	try {
		if (!user_id) {
			return reply.code(400).send({ error: "user_id (telegram_id) обязателен" });
		}

		const existingByTg = await db.query("SELECT id, username FROM users WHERE telegram_id = $1", [user_id]);
		if (existingByTg.rows.length > 0) {
			return reply.code(409).send({ error: "Пользователь уже существует" });
		}

		const result = await db.query(
			"INSERT INTO users (username, telegram_id) VALUES ($1, $2) RETURNING id, username",
			[username, user_id]
		);
		return { status: "ok", user: result.rows[0] };
	} catch (err) {
		console.error("[AUTH] Error registering user", err);
		return reply.code(500).send({ error: "Ошибка БД при регистрации (возможно имя уже занято)" });
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

	const { user_id } = req.body || {};
	if (!user_id) {
		return reply.code(400).send({ error: "user_id обязателен" });
	}

	try {
		const resTg = await db.query("SELECT id, username, is_active FROM users WHERE telegram_id = $1", [user_id]);
		const user = resTg.rows[0];

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

		console.log(`[AUTH OTP] Сгенерирован OTP для @${user.username} (действителен 120с)`);
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
// 4. (УДАЛЕН) СТАНДАРТНЫЙ ВХОД ПО ЛОГИНУ И ПАРОЛЮ
// ----------------------------------------------------

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
// 7. УПРАВЛЕНИЕ БЕЛЫМИ СПИСКАМИ (ADMIN API)
// ----------------------------------------------------
const verifyBotToken = (req, reply, done) => {
	const botToken = req.headers["x-bot-token"];
	if (!botToken || botToken.length !== process.env.BOT_TOKEN.length) {
		return reply.code(403).send({ error: "Доступ запрещен" });
	}
	if (!crypto.timingSafeEqual(Buffer.from(botToken), Buffer.from(process.env.BOT_TOKEN))) {
		return reply.code(403).send({ error: "Доступ запрещен" });
	}
	done();
};

// Статистика трафика пользователей (для TG бота)
fastify.get("/api/admin/traffic", { preHandler: verifyBotToken }, async (req, reply) => {
	try {
		const result = await db.query("SELECT id, username, is_active, telegram_id FROM users ORDER BY id ASC");
		const users = [];

		for (const u of result.rows) {
			const totalRaw = await redis.get(`traffic:${u.id}:total`).catch(() => null);
			const totalBytes = parseInt(totalRaw || "0", 10);

			const tc = trafficCounters.get(u.id);
			const sessionBytes = tc ? tc.session : 0;

			const domainInfo = activeUserDomains.get(u.id);
			const isConnected = activeWispSockets.has(u.id) && activeWispSockets.get(u.id).size > 0;

			users.push({
				id: u.id,
				username: u.username,
				telegram_id: u.telegram_id,
				is_active: u.is_active,
				connected: isConnected,
				domain: domainInfo ? domainInfo.domain : null,
				sessionBytes,
				totalBytes,
			});
		}

		return { status: "ok", users };
	} catch (e) {
		console.error("[ADMIN TRAFFIC ERROR]", e);
		return reply.code(500).send({ error: "Error fetching traffic" });
	}
});

fastify.get("/api/admin/whitelists", { preHandler: verifyBotToken }, async (req, reply) => {
	const tgArray = Array.from(allowedTgIds.entries()).map(([id, name]) => ({ id, name }));
	return { status: "ok", ips: allowedIps, tg_ids: tgArray };
});

fastify.post("/api/admin/whitelists", { preHandler: verifyBotToken }, async (req, reply) => {
	const { type, value, name } = req.body || {};
	if (!['ip', 'tg_id'].includes(type) || !value) {
		return reply.code(400).send({ error: "Неверные данные" });
	}
	try {
		await db.query(
			"INSERT INTO whitelists (type, value, name) VALUES ($1, $2, $3) ON CONFLICT (type, value) DO UPDATE SET name = EXCLUDED.name",
			[type, value, name || null]
		);
		if (type === 'ip' && !allowedIps.includes(value)) allowedIps.push(value);
		if (type === 'tg_id') allowedTgIds.set(value, name || null);
		return { status: "ok" };
	} catch (err) {
		console.error("[ADMIN API] Error adding to whitelist:", err);
		return reply.code(500).send({ error: "Ошибка БД" });
	}
});

fastify.delete("/api/admin/whitelists", { preHandler: verifyBotToken }, async (req, reply) => {
	const { type, value } = req.body || {};
	if (!['ip', 'tg_id'].includes(type) || !value) {
		return reply.code(400).send({ error: "Неверные данные" });
	}
	try {
		await db.query(
			"DELETE FROM whitelists WHERE type = $1 AND value = $2",
			[type, value]
		);
		if (type === 'ip') allowedIps = allowedIps.filter(v => v !== value);
		if (type === 'tg_id') allowedTgIds.delete(value);
		return { status: "ok" };
	} catch (err) {
		console.error("[ADMIN API] Error deleting from whitelist:", err);
		return reply.code(500).send({ error: "Ошибка БД" });
	}
});

fastify.get("/api/admin/users", { preHandler: verifyBotToken }, async (req, reply) => {
	try {
		const result = await db.query("SELECT id, username, is_active, created_at, telegram_id FROM users ORDER BY id ASC");
		return { status: "ok", users: result.rows };
	} catch (e) {
		console.error("[ADMIN] Error fetching users", e);
		return reply.code(500).send({ error: "DB error" });
	}
});

fastify.put("/api/admin/users/:id", { preHandler: verifyBotToken }, async (req, reply) => {
	const { id } = req.params;
	const { username } = req.body || {};
	if (!username) return reply.code(400).send({ error: "Имя не может быть пустым" });
	try {
		await db.query("UPDATE users SET username = $1 WHERE id = $2", [username, id]);
		return { status: "ok" };
	} catch (e) {
		console.error("[ADMIN] Error updating user", e);
		return reply.code(500).send({ error: "DB error" });
	}
});

fastify.delete("/api/admin/users/:id", { preHandler: verifyBotToken }, async (req, reply) => {
	const { id } = req.params;
	console.log(`[DEBUG DELETE] Attempting to delete user ${id}`);
	try {
		const delRes = await db.query("DELETE FROM users WHERE id = $1", [id]);
		console.log(`[DEBUG DELETE] DB delete result: ${delRes.rowCount}`);
		
		// Invalidate active wisp session
		for (const [key, sockets] of activeWispSockets.entries()) {
			if (String(key) === String(id)) {
				console.log(`[DEBUG DELETE] Found wisp sockets for ${id}`);
				for (const socket of sockets) {
					try { socket.destroy(); } catch (e) { console.error(`[DEBUG DELETE] Socket destroy error:`, e); }
				}
				activeWispSockets.delete(key);
				console.log(`[ADMIN] Wisp sockets for deleted user ${id} closed.`);
			}
		}
		
		console.log(`[DEBUG DELETE] Success`);
		return { status: "ok" };
	} catch (e) {
		console.error("[ADMIN] Error deleting user", e);
		return reply.code(500).send({ error: "DB error" });
	}
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