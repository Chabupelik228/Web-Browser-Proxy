# ==========================================
# ЭТАП 1: СБОРКА ВЕБ-ПРИЛОЖЕНИЯ VUE 3 (BUILD)
# ==========================================
FROM node:18-alpine AS builder

WORKDIR /frontend

# Гарантированно устанавливаем Tailwind & Vite для сборки
COPY frontend/package*.json ./
RUN npm install --include=dev

COPY frontend/ ./
RUN npm run build

# ==========================================
# ЭТАП 2: ЗАПУСК СЕРВЕРА FASTIFY (RUNNER)
# ==========================================
FROM node:18-alpine AS runner

ENV NODE_ENV=production

WORKDIR /app

RUN apk add --upgrade --no-cache python3 make g++ 
RUN npm install -g pnpm

COPY backend/package*.json ./
RUN pnpm install --prod

COPY backend/ ./
COPY --from=builder /frontend/dist/ ./public/

ENTRYPOINT [ "node" ]
CMD ["src/index.js"]