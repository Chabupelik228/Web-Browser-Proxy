<!-- frontend/src/App.vue -->
<template>
  <div class="h-screen w-screen bg-gray-900 text-white overflow-hidden font-sans">
    <LoginView v-if="!authStore.accessToken" />
    <BrowserView v-else />
  </div>
</template>

<script setup>
import { onMounted } from 'vue';
import { useAuthStore } from './stores/auth.store';
import LoginView from './views/LoginView.vue';
import BrowserView from './views/BrowserView.vue';

const authStore = useAuthStore();

onMounted(async () => {
  if ('serviceWorker' in navigator) {
    // 1. Инициализируем ScramjetController
    if (window.$scramjetLoadController) {
      try {
        const { ScramjetController } = window.$scramjetLoadController();
        const scramjetClient = new ScramjetController({
          prefix: "/service/",
          files: {
            wasm: "/scram/scramjet.wasm.wasm",
            all: "/scram/scramjet.all.js",
            sync: "/scram/scramjet.sync.js",
          }
        });
        await scramjetClient.init();
        
        // СОХРАНЯЕМ КОНТРОЛЛЕР В WINDOW ДЛЯ КОДИРОВАНИЯ ССЫЛОК
        window.scramjet = scramjetClient;
        console.log('[Scramjet] Клиентский контроллер успешно инициализирован!');
      } catch (e) {
        console.error('[Scramjet] Ошибка инициализации ScramjetController:', e);
      }
    }

    // 2. Регистрируем Service Worker
    await navigator.serviceWorker.register('/sw.js', { scope: '/' });
    console.log('Scramjet SW Registered (Classic Mode)');

    // 3. Инициализация BareMux
    const checkBareMux = setInterval(async () => {
      if (window.BareMux) {
        clearInterval(checkBareMux);
        
        try {
          const ConnectionClass = window.BareMuxConnection ||
                                 window.BareMux?.BareMuxConnection || 
                                 window.BareMux?.default?.BareMuxConnection ||
                                 window.BareMux?.default || 
                                 window.BareMux;

          if (typeof ConnectionClass !== 'function') {
              throw new TypeError('Не удалось найти конструктор подключения в модуле BareMux.');
          }

          const mux = new ConnectionClass('/baremux/worker.js');
          const wispProtocol = location.protocol === 'https:' ? 'wss://' : 'ws://';
          const wispUrl = `${wispProtocol}${location.host}/wisp/`;

          await mux.setTransport('/libcurl/index.mjs', [{ wisp: wispUrl }]);
          console.log('[BareMux] Сетевой транспорт УСПЕШНО установлен!');
        } catch (err) {
          console.error('[BareMux] Ошибка инициализации транспорта:', err);
        }
      }
    }, 100);
  }
});
</script>