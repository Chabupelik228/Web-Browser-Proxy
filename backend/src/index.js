import { join } from "node:path";
import { hostname } from "node:os";
import { createServer } from "node:http";
import { fileURLToPath } from "url";
import { server as wisp, logging } from "@mercuryworkshop/wisp-js/server";
import Fastify from "fastify";
import fastifyStatic from "@fastify/static";
import pg from "pg";
import { createClient } from "redis";
import jwt from "jsonwebtoken";
import argon2 from "argon2";

import { scramjetPath } from "@mercuryworkshop/scramjet/path";
import { libcurlPath } from "@mercuryworkshop/libcurl-transport";
import { baremuxPath } from "@mercuryworkshop/bare-mux/node";

const publicPath = fileURLToPath(new URL("../public/", import.meta.url));

const { Pool } = pg;
const db = new Pool({ connectionString: process.env.DATABASE_URL });
const redis = createClient({ url: process.env.REDIS_URL });
redis.connect().catch(console.error);

logging.set_level(logging.NONE);
Object.assign(wisp.options, {
	allow_udp_streams: false,
	hostname_blacklist: [/example\.com/],
	dns_servers: ["94.140.14.14", "94.140.15.15"],
});

const fastify = Fastify({
	serverFactory: (handler) => {
		return createServer()
			.on("request", (req, res) => {
				res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
				res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
				handler(req, res);
			})
			.on("upgrade", (req, socket, head) => {
				if (req.url.endsWith("/wisp/")) {
					console.log("[BACKEND] Успешное Wisp-подключение (проксирование)");
					wisp.routeRequest(req, socket, head);
					return;
				}
				socket.end();
			});
	},
});

const checkAuth = async (req, reply) => {
	const authHeader = req.headers["authorization"];
	const token = authHeader && authHeader.split(" ")[1];
	if (!token) {
		console.warn("[BACKEND AUTH WARNING] Запрос без токена к:", req.url);
		reply.code(401).send({ status: "error", message: "Токен отсутствует" });
		return;
	}
	try {
		req.user = jwt.verify(token, process.env.JWT_SECRET);
	} catch (err) {
		console.warn("[BACKEND AUTH WARNING] Невалидный токен:", req.url);
		reply.code(403).send({ status: "error", message: "Невалидный токен" });
	}
};

fastify.post("/api/auth/register", async (req, reply) => {
	console.log("[BACKEND API] Регистрация пользователя от бота:", req.body?.username);
	const botToken = req.headers["x-bot-token"];
	if (botToken !== process.env.BOT_TOKEN) {
		console.error("[BACKEND API ERROR] Ошибка x-bot-token");
		return reply.code(403).send({ error: "Доступ запрещен" });
	}
	const { username, password } = req.body;
	try {
		const passwordHash = await argon2.hash(password);
		const result = await db.query(
			"INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING id, username",
			[username, passwordHash]
		);
		console.log("[BACKEND API SUCCESS] Пользователь зарегистрирован в БД:", username);
		return { status: "ok", user: result.rows[0] };
	} catch (err) {
		console.error("[BACKEND API ERROR] Ошибка создания юзера в БД:", err.message);
		return reply.code(500).send({ error: "Пользователь уже существует" });
	}
});

fastify.post("/api/auth/login", async (req, reply) => {
	console.log("[BACKEND API] Запрос входа:", req.body?.username);
	const { username, password } = req.body;
	try {
		const result = await db.query("SELECT * FROM users WHERE username = $1", [username]);
		const user = result.rows[0];
		if (!user || !(await argon2.verify(user.password_hash, password))) {
			console.warn("[BACKEND API LOGIN FAIL] Ошибка логина/пароля:", username);
			return reply.code(400).send({ status: "error", message: "Неверный логин или пароль" });
		}

		const accessToken = jwt.sign({ id: user.id, username: user.username }, process.env.JWT_SECRET, { expiresIn: "15m" });
		const refreshToken = jwt.sign({ id: user.id }, process.env.JWT_SECRET, { expiresIn: "7d" });

		reply.header("Set-Cookie", `refreshToken=${refreshToken}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=${7 * 24 * 60 * 60}`);
		console.log("[BACKEND API LOGIN SUCCESS] Успешный вход юзера:", username);
		return { status: "ok", accessToken, user: { id: user.id, username: user.username } };
	} catch (err) {
		console.error("[BACKEND API ERROR] Ошибка авторизации:", err);
		return reply.code(500).send({ status: "error", message: "Ошибка сервера" });
	}
});

fastify.post("/api/auth/refresh", async (req, reply) => {
	console.log("[BACKEND API] Запрос обновления токенов (Refresh)");
	const cookieHeader = req.headers.cookie || "";
	const match = cookieHeader.match(/refreshToken=([^;]+)/);
	const refreshToken = match ? match[1] : null;

	if (!refreshToken) return reply.code(401).send({ status: "error" });

	try {
		const decoded = jwt.verify(refreshToken, process.env.JWT_SECRET);
		const result = await db.query("SELECT id, username FROM users WHERE id = $1", [decoded.id]);
		const user = result.rows[0];
		if (!user) return reply.code(403).send({ status: "error" });

		const newAccessToken = jwt.sign({ id: user.id, username: user.username }, process.env.JWT_SECRET, { expiresIn: "15m" });
		const newRefreshToken = jwt.sign({ id: user.id }, process.env.JWT_SECRET, { expiresIn: "7d" });

		reply.header("Set-Cookie", `refreshToken=${newRefreshToken}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=${7 * 24 * 60 * 60}`);
		return { status: "ok", accessToken: newAccessToken, user };
	} catch (err) {
		return reply.code(403).send({ status: "error" });
	}
});

fastify.post("/api/auth/logout", async (req, reply) => {
	console.log("[BACKEND API] Выход пользователя");
	reply.header("Set-Cookie", "refreshToken=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0");
	return { status: "ok" };
});

fastify.get("/api/sync", async (req, reply) => {
	await checkAuth(req, reply);
	if (reply.sent) return;

	try {
		console.log("[BACKEND SYNC] Запрос зашифрованных кук для user_id:", req.user.id);
		const result = await db.query("SELECT encrypted_cookies, open_tabs FROM sessions_sync WHERE user_id = $1", [req.user.id]);
		return { status: "ok", data: result.rows[0] || { encrypted_cookies: null, open_tabs: [] } };
	} catch (err) {
		return reply.code(500).send({ status: "error" });
	}
});

fastify.post("/api/sync", async (req, reply) => {
	await checkAuth(req, reply);
	if (reply.sent) return;

	const { encrypted_cookies, open_tabs } = req.body;
	try {
		console.log("[BACKEND SYNC] Сохранение зашифрованных кук от user_id:", req.user.id);
		await db.query(
			"INSERT INTO sessions_sync (user_id, encrypted_cookies, open_tabs, updated_at) VALUES ($1, $2, $3, NOW()) ON CONFLICT (user_id) DO UPDATE SET encrypted_cookies = $2, open_tabs = $3, updated_at = NOW()",
			[req.user.id, encrypted_cookies, JSON.stringify(open_tabs)]
		);
		return { status: "ok" };
	} catch (err) {
		return reply.code(500).send({ status: "error" });
	}
});

fastify.register(fastifyStatic, {
	root: publicPath,
	decorateReply: true,
});

fastify.register(fastifyStatic, {
	root: scramjetPath,
	prefix: "/scram/",
	decorateReply: false,
});

fastify.register(fastifyStatic, {
	root: libcurlPath,
	prefix: "/libcurl/",
	decorateReply: false,
});

fastify.register(fastifyStatic, {
	root: baremuxPath,
	prefix: "/baremux/",
	decorateReply: false,
});

fastify.setNotFoundHandler((req, reply) => {
	console.warn("[BACKEND 404] Запрос не найден:", req.url);
	return reply.code(404).type("text/html").sendFile("404.html");
});

fastify.server.on("listening", () => {
	const address = fastify.server.address();
	console.log("Listening on:");
	console.log(`\thttp://localhost:${address.port}`);
	console.log(`\thttp://${hostname()}:${address.port}`);
});

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);

function shutdown() {
	console.log("SIGTERM signal received: closing HTTP server");
	fastify.close();
	db.end();
	process.exit(0);
}

let port = parseInt(process.env.PORT || "");
if (isNaN(port)) port = 8080;

fastify.listen({
	port: port,
	host: "0.0.0.0",
});