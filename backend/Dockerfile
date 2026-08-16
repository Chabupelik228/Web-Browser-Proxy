# ==========================================
# ЗАПУСК FASTIFY + WISP СЕРВЕРА (API GATEWAY)
# ==========================================
FROM node:18-alpine

ENV NODE_ENV=production

WORKDIR /app

RUN apk add --upgrade --no-cache python3 make g++ 
RUN npm install -g pnpm

# Копируем package.json и устанавливаем зависимости бэкенда
COPY package*.json ./
RUN pnpm install --prod

# Копируем исходный код бэкенда
COPY . ./

ENTRYPOINT [ "node" ]
CMD ["src/index.js"]