export class LogService {
    constructor(dbPool) {
        this.db = dbPool;
        this.queue = [];
        this.batchSize = 100;
        this.maxQueueSize = 10000;
        this.isFlushing = false;

        // Периодический сброс каждую 1 секунду
        this.flushInterval = setInterval(() => {
            this.flush();
        }, 1000);
        
        // Prevent interval from blocking Node.js event loop shutdown if needed
        if (this.flushInterval.unref) {
            this.flushInterval.unref();
        }

        // Cache для дедупликации (очищается каждые 5 секунд)
        this.seenKeys = new Set();
        this.dedupInterval = setInterval(() => {
            this.seenKeys.clear();
        }, 5000);
        if (this.dedupInterval.unref) {
            this.dedupInterval.unref();
        }
    }

    /**
     * @param {string} telegramId
     * @param {string} targetHost 
     * @param {number} targetPort 
     */
    log(telegramId, targetHost, targetPort) {
        if (!telegramId || !targetHost || !targetPort) return;

        // Защита от OOM при падении БД
        if (this.queue.length >= this.maxQueueSize) {
            console.warn("[LOG_SERVICE] Очередь переполнена, событие отброшено.");
            return;
        }

        // Дедупликация в пределах 1 секунды
        // Отсекаем миллисекунды от ISO-строки, чтобы получить метку времени с точностью до секунды
        const secTimestamp = new Date().toISOString().split('.')[0] + 'Z';
        const dedupKey = `${telegramId}|${targetHost}|${targetPort}|${secTimestamp}`;

        if (this.seenKeys.has(dedupKey)) {
            return; // Пропускаем дубликат
        }
        this.seenKeys.add(dedupKey);

        this.queue.push({
            telegramId,
            targetHost,
            targetPort,
            timestamp: secTimestamp // Сохраняем округленное время
        });

        // Немедленный flush при достижении batchSize
        if (this.queue.length >= this.batchSize) {
            // Асинхронно вызываем flush, не блокируя поток Wisp
            setImmediate(() => this.flush());
        }
    }

    async flush() {
        if (this.isFlushing || this.queue.length === 0) return;
        this.isFlushing = true;

        const batch = this.queue.splice(0, this.queue.length);

        try {
            const values = [];
            const flatData = [];
            let placeholderIndex = 1;

            for (const item of batch) {
                values.push(`($${placeholderIndex++}, $${placeholderIndex++}, $${placeholderIndex++}, $${placeholderIndex++})`);
                flatData.push(item.telegramId, item.targetHost, item.targetPort, item.timestamp);
            }

            const query = `
                INSERT INTO wisp_access_logs (telegram_id, target_host, target_port, created_at)
                VALUES ${values.join(", ")}
            `;

            await this.db.query(query, flatData);
        } catch (err) {
            console.error("[LOG_SERVICE] Ошибка записи логов в PostgreSQL:", err.message);
            // Возвращаем события обратно в очередь, если есть место
            if (this.queue.length + batch.length <= this.maxQueueSize) {
                this.queue = batch.concat(this.queue);
            }
        } finally {
            this.isFlushing = false;
        }
    }

    async shutdown() {
        clearInterval(this.flushInterval);
        clearInterval(this.dedupInterval);
        await this.flush();
    }
}
