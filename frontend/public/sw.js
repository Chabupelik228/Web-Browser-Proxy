

const originalOpen = self.indexedDB.open;
self.indexedDB.open = function(name, version) {
    const request = originalOpen.apply(this, arguments);
    request.addEventListener('upgradeneeded', (event) => {
        const db = event.target.result;
        if (name === '$scramjet') {
            if (!db.objectStoreNames.contains('keyval')) db.createObjectStore('keyval');
            if (!db.objectStoreNames.contains('config')) db.createObjectStore('config');
            if (!db.objectStoreNames.contains('cookies')) db.createObjectStore('cookies');
        }
    });
    return request;
};

// 2. Загружаем конфигурацию и ядро Scramjet
// frontend/public/sw.js

importScripts('/scram/scramjet.all.js');

const { ScramjetServiceWorker } = $scramjetLoadWorker();
const scramjet = new ScramjetServiceWorker();

self.addEventListener('install', () => {
    void self.skipWaiting();
});

self.addEventListener('activate', (event) => {
    event.waitUntil(self.clients.claim());
});

self.addEventListener('fetch', (event) => {
    const url = event.request.url;

    // АНТИ-КРАШ YOUTUBE НА УРОВНЕ СЕТИ:
    // Перехватываем десктопный Ютуб до загрузки heavy base.js и перенаправляем на легкий плеер embed
    if (url.includes('youtube.com/watch?v=')) {
        const videoId = url.split('v=')[1]?.split('&')[0];
        if (videoId) {
            event.respondWith(Response.redirect(`https://www.youtube-nocookie.com/embed/${videoId}?autoplay=1`, 302));
            return;
        }
    }
    if (url.includes('youtu.be/')) {
        const videoId = url.split('youtu.be/')[1]?.split('?')[0];
        if (videoId) {
            event.respondWith(Response.redirect(`https://www.youtube-nocookie.com/embed/${videoId}?autoplay=1`, 302));
            return;
        }
    }

    // Стандартная обработка Scramjet
    event.respondWith(
        (async () => {
            try {
                await scramjet.loadConfig();
                if (scramjet.route(event)) {
                    return await scramjet.fetch(event);
                }
            } catch (err) {
                console.error('[Scramjet SW Error]', err);
            }
            return fetch(event.request);
        })()
    );
});