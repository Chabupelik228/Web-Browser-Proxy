<!-- frontend/src/App.vue -->
<template>
  <div class="h-screen w-screen bg-[#08080a] text-white overflow-hidden font-sans select-none morph-container" :class="{'bg-transparent': isWidget}">
    <DownloadsWidget v-if="widgetName === 'downloads'" />
    <MenuWidget v-else-if="widgetName === 'menu'" />

    <template v-else>
      <FakeChromeView 
        v-if="!authStore.accessToken || authStore.transitionPhase !== 'done'" 
        class="morph-layer"
        :class="{ 'morph-fade-out': authStore.transitionPhase === 'morphing' }"
      />

      <BrowserView 
        v-if="authStore.isProxyReady" 
        class="morph-layer"
        :class="{ 'morph-fade-in': authStore.transitionPhase === 'morphing' }"
      />
    </template>
  </div>
</template>

<script setup>
import { onMounted, onUnmounted } from 'vue';
import { useAuthStore } from './stores/auth.store';
import FakeChromeView from './views/FakeChromeView.vue';
import BrowserView from './views/BrowserView.vue';
import DownloadsWidget from './views/DownloadsWidget.vue';
import MenuWidget from './views/MenuWidget.vue';

const authStore = useAuthStore();

const urlParams = new URLSearchParams(window.location.search);
const widgetName = urlParams.get('widget');
const isWidget = !!widgetName;

</script>