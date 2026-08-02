// frontend/public/sw.js

importScripts('/config.js');
importScripts('/scram/scramjet.all.js');

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

const { ScramjetServiceWorker } = $scramjetLoadWorker();
const scramjet = new ScramjetServiceWorker();

self.addEventListener('install', () => {
    void self.skipWaiting();
});

self.addEventListener('activate', (event) => {
    event.waitUntil(self.clients.claim());
});

self.addEventListener('fetch', (event) => {
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