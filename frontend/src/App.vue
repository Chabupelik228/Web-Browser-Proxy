<!-- frontend/src/App.vue -->
<template>
  <div class="h-screen w-screen bg-[#08080a] text-white overflow-hidden font-sans select-none" :class="{'bg-transparent': isWidget}">
    <DownloadsWidget v-if="widgetName === 'downloads'" />

    <template v-else>
      <LoginView v-if="!authStore.accessToken" />

      <div
        v-else-if="!authStore.isProxyReady"
        class="h-full w-full flex flex-col items-center justify-center bg-[#08080a] text-zinc-400 text-xs gap-3"
      >
        <div class="w-6 h-6 border-2 border-emerald-500 border-t-transparent rounded-full animate-spin"></div>
        <span class="font-mono text-zinc-300">Устанавливаем защищенный Wisp-туннель к VPS...</span>
      </div>

      <BrowserView v-else />
    </template>
  </div>
</template>

<script setup>
import { onMounted, onUnmounted } from 'vue';
import { useAuthStore } from './stores/auth.store';
import LoginView from './views/LoginView.vue';
import BrowserView from './views/BrowserView.vue';
import DownloadsWidget from './views/DownloadsWidget.vue';

const authStore = useAuthStore();

const urlParams = new URLSearchParams(window.location.search);
const widgetName = urlParams.get('widget');
const isWidget = !!widgetName;

let autosaveInterval = null;

onMounted(() => {
  // Периодическое автосохранение сессии каждые 2 минуты
  autosaveInterval = setInterval(() => {
    if (authStore.accessToken && authStore.isProxyReady) {
      authStore.backupSession();
    }
  }, 120000);
});

onUnmounted(() => {
  if (autosaveInterval) {
    clearInterval(autosaveInterval);
  }
});
</script>