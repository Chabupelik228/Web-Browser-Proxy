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
    if (window.$scramjetLoadController && window.__scramjet$config) {
      try {
        const { ScramjetController } = window.$scramjetLoadController();
        
        // Передаем полную структуру files (все 7 строк)
        const scramjetClient = new ScramjetController({
          prefix: window.__scramjet$config.prefix,
          codec: window.__scramjet$config.codec,
          files: window.__scramjet$config.files // В нем 7 строк, toString() сработает идеально
        });
        await scramjetClient.init();
        
        window.scramjet = scramjetClient;
        console.log('[Scramjet] Клиентский контроллер успешно инициализирован со встроенным XOR!');
      } catch (e) {
        console.error('[Scramjet] Ошибка инициализации Controller:', e);
      }
    }

    // 2. Регистрируем Service Worker
    await navigator.serviceWorker.register('/sw.js', { scope: '/' });
    console.log('Scramjet SW Registered');

    // 3. Инициализация BareMux
    const checkBareMux = setInterval(async () => {
      if (window.BareMux) {
        clearInterval(checkBareMux);
        try {
          const ConnectionClass = window.BareMuxConnection || window.BareMux?.BareMuxConnection || window.BareMux;
          const mux = new ConnectionClass('/baremux/worker.js');
          const wispUrl = `${location.protocol === 'https:' ? 'wss://' : 'ws://'}${location.host}/wisp/`;

          await mux.setTransport('/libcurl/index.mjs', [{ wisp: wispUrl }]);
          console.log('[BareMux] Транспорт УСПЕШНО установлен!');
        } catch (err) {
          console.error('[BareMux] Ошибка инициализации транспорта:', err);
        }
      }
    }, 100);
  }
});
</script>