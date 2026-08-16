<template>
  <div class="flex flex-col h-screen w-screen bg-[#08080a] text-zinc-200 select-none font-sans overflow-hidden">
    
    <!-- 1. ПАНЕЛЬ ВКЛАДОК -->
    <div class="flex items-center bg-[#0d0d10] px-2 pt-1.5 overflow-x-auto border-b border-zinc-800/80 relative z-20 shrink-0">
      <div 
        v-for="tab in browserStore.tabs" 
        :key="tab.id"
        @click="setActiveTab(tab.id)"
        class="flex items-center justify-between px-3 py-1.5 w-48 text-xs cursor-pointer rounded-t-xl transition-all mr-1.5 group relative border-t border-x"
        :class="tab.id === browserStore.activeTabId 
          ? 'bg-[#15151b] text-zinc-100 border-zinc-700/80 shadow-sm font-medium' 
          : 'bg-[#0d0d10] text-zinc-500 border-transparent hover:bg-[#121217] hover:text-zinc-300'"
      >
        <div class="flex items-center space-x-2 truncate pr-2">
          <div v-if="tab.isLoading" class="w-3.5 h-3.5 border-2 border-zinc-400 border-t-transparent rounded-full animate-spin flex-shrink-0"></div>
          
          <img 
            v-else-if="getTabFavicon(tab)" 
            :src="getTabFavicon(tab)" 
            class="w-3.5 h-3.5 rounded-sm flex-shrink-0 object-contain"
            @error="(e) => onFaviconError(tab)"
          />
          
          <svg v-else class="w-3.5 h-3.5 text-zinc-500 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
          </svg>
          
          <span class="truncate">{{ getTabTitle(tab) }}</span>
        </div>

        <button 
          @click.stop="closeTab(tab.id)" 
          class="text-zinc-500 hover:text-zinc-200 font-medium rounded-md w-4 h-4 flex items-center justify-center hover:bg-zinc-700/50 transition-colors"
        >
          ✕
        </button>
      </div>
      
      <!-- Новая вкладка -->
      <button 
        @click="addNewTab" 
        class="ml-0.5 text-zinc-500 hover:text-zinc-200 text-sm w-6 h-6 flex items-center justify-center rounded-lg hover:bg-zinc-800 transition-colors"
        title="Новая вкладка"
      >
        +
      </button>
      
      <!-- Правый статус-блок -->
      <div class="ml-auto flex items-center space-x-3">
        <div class="hidden md:flex items-center space-x-1.5 text-[11px] text-emerald-400 font-mono bg-[#13131a] px-2.5 py-1 rounded-lg border border-zinc-800">
          <span class="w-1.5 h-1.5 bg-emerald-400 rounded-full animate-pulse"></span>
          <span>Туннель активен</span>
        </div>
        <button 
          @click="handleLogout" 
          class="bg-zinc-800 hover:bg-zinc-700 text-zinc-300 font-medium text-[11px] px-3 py-1 rounded-md transition-all border border-zinc-700/60"
        >
          Завершить сессию
        </button>
      </div>
    </div>

    <!-- 2. АДРЕСНАЯ СТРОКА -->
    <div class="flex items-center space-x-2 bg-[#15151b] p-2 border-b border-zinc-800/80 shrink-0">
      
      <div class="flex items-center space-x-1">
        <button @click="goBack" class="p-1.5 text-zinc-400 hover:text-zinc-100 rounded-lg hover:bg-zinc-800 transition-colors" title="Назад">
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 12H5"/><path d="M12 19l-7-7 7-7"/></svg>
        </button>

        <button @click="goForward" class="p-1.5 text-zinc-400 hover:text-zinc-100 rounded-lg hover:bg-zinc-800 transition-colors" title="Вперед">
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5l7 7-7 7"/></svg>
        </button>

        <button @click="reloadCurrentTab" class="p-1.5 text-zinc-400 hover:text-zinc-100 rounded-lg hover:bg-zinc-800 transition-colors" title="Обновить">
          <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/></svg>
        </button>
      </div>
      
      <form @submit.prevent="navigate" class="flex-1 flex">
        <div class="relative w-full flex items-center">
          <span class="absolute left-3 text-emerald-400">
            <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
            </svg>
          </span>
          <input 
            v-model="inputUrl" 
            type="text" 
            placeholder="Введите URL-адрес или поисковый запрос..."
            class="w-full bg-[#0d0d11] text-zinc-200 border border-zinc-800 rounded-xl pl-9 pr-24 py-1.5 focus:outline-none focus:border-zinc-600 text-xs transition-all placeholder-zinc-600 shadow-inner"
          >
          <span class="absolute right-3 text-[10px] font-mono text-emerald-500/80 select-none hidden sm:inline">
            CHROMIUM ENGINE
          </span>
        </div>
      </form>
    </div>

    <!-- 3. СТАРТОВАЯ СТРАНИЦА (Отображается когда вкладка пустая) -->
    <div v-show="isCurrentStartPage" class="flex-1 bg-[#08080a] relative overflow-hidden flex flex-col items-center justify-center px-4">
      
      <div class="w-full max-w-2xl flex flex-col items-center py-6">
        
        <div class="flex flex-col items-center mb-7 text-center">
          <div class="w-12 h-12 bg-[#111115] border border-zinc-700/80 rounded-2xl flex items-center justify-center text-zinc-100 mb-3 shadow-xl">
            <svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
            </svg>
          </div>
          <h1 class="text-2xl font-bold text-zinc-100 tracking-tight">Chabupelik Browser</h1>
          <p class="text-xs text-zinc-500 font-medium mt-1">
            Настоящий браузер с защищенным туннелем к VPS
          </p>
        </div>

        <!-- Поисковая строка -->
        <form @submit.prevent="handleStartPageSearch" class="w-full flex items-center bg-[#111116] border border-zinc-700/80 rounded-2xl p-1.5 shadow-2xl focus-within:border-zinc-500 transition-all mb-8">
          <span class="pl-3.5 pr-2 text-zinc-500">
            <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
          </span>
          <input v-model="startPageInput" type="text" placeholder="Поиск в интернете или ввод URL..." class="w-full bg-transparent text-zinc-100 placeholder-zinc-500 text-xs focus:outline-none px-2 py-2" autofocus />
          
          <button type="submit" class="bg-zinc-100 hover:bg-white text-zinc-950 p-2.5 rounded-xl transition-all shadow-md active:scale-95 shrink-0 flex items-center justify-center ml-1" title="Искать">
            <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
          </button>
        </form>

        <!-- БЛОК БЫСТРОГО ДОСТУПА К НЕЙРОСЕТЯМ -->
        <div class="w-full mb-8">
          <div class="flex items-center justify-between mb-3 px-1">
            <span class="text-[11px] font-mono text-zinc-400 uppercase tracking-wider">Быстрый доступ к сервисам</span>
            <span class="text-[10px] text-emerald-400 font-mono">100% Native Chrome</span>
          </div>

          <div class="grid grid-cols-2 sm:grid-cols-3 gap-2.5">
            
            <button @click="openUrlInActiveTab('https://chatgpt.com')" class="flex items-center space-x-3 p-3 bg-[#101014] hover:bg-[#16161c] border border-zinc-800/90 hover:border-zinc-700 rounded-xl transition-all group text-left shadow-sm">
              <div class="w-8 h-8 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center shrink-0 text-emerald-400 font-bold text-sm group-hover:scale-105 transition-transform">
                🤖
              </div>
              <div class="truncate">
                <div class="text-xs font-semibold text-zinc-200 group-hover:text-white transition-colors">ChatGPT</div>
                <div class="text-[10px] text-zinc-500 truncate">chatgpt.com</div>
              </div>
            </button>

            <button @click="openUrlInActiveTab('https://chat.qwen.ai')" class="flex items-center space-x-3 p-3 bg-[#101014] hover:bg-[#16161c] border border-zinc-800/90 hover:border-zinc-700 rounded-xl transition-all group text-left shadow-sm">
              <div class="w-8 h-8 rounded-lg bg-indigo-500/10 border border-indigo-500/20 flex items-center justify-center shrink-0 text-indigo-400 font-bold text-sm group-hover:scale-105 transition-transform">
                ⚡
              </div>
              <div class="truncate">
                <div class="text-xs font-semibold text-zinc-200 group-hover:text-white transition-colors">Qwen AI</div>
                <div class="text-[10px] text-zinc-500 truncate">chat.qwen.ai</div>
              </div>
            </button>

            <button @click="openUrlInActiveTab('https://aistudio.google.com')" class="flex items-center space-x-3 p-3 bg-[#101014] hover:bg-[#16161c] border border-zinc-800/90 hover:border-zinc-700 rounded-xl transition-all group text-left shadow-sm">
              <div class="w-8 h-8 rounded-lg bg-blue-500/10 border border-blue-500/20 flex items-center justify-center shrink-0 text-blue-400 font-bold text-sm group-hover:scale-105 transition-transform">
                ✨
              </div>
              <div class="truncate">
                <div class="text-xs font-semibold text-zinc-200 group-hover:text-white transition-colors">AI Studio</div>
                <div class="text-[10px] text-zinc-500 truncate">aistudio.google.com</div>
              </div>
            </button>

            <button @click="openUrlInActiveTab('https://claude.ai')" class="flex items-center space-x-3 p-3 bg-[#101014] hover:bg-[#16161c] border border-zinc-800/90 hover:border-zinc-700 rounded-xl transition-all group text-left shadow-sm">
              <div class="w-8 h-8 rounded-lg bg-amber-500/10 border border-amber-500/20 flex items-center justify-center shrink-0 text-amber-400 font-bold text-sm group-hover:scale-105 transition-transform">
                🧠
              </div>
              <div class="truncate">
                <div class="text-xs font-semibold text-zinc-200 group-hover:text-white transition-colors">Claude AI</div>
                <div class="text-[10px] text-zinc-500 truncate">claude.ai</div>
              </div>
            </button>

            <button @click="openUrlInActiveTab('https://perplexity.ai')" class="flex items-center space-x-3 p-3 bg-[#101014] hover:bg-[#16161c] border border-zinc-800/90 hover:border-zinc-700 rounded-xl transition-all group text-left shadow-sm">
              <div class="w-8 h-8 rounded-lg bg-cyan-500/10 border border-cyan-500/20 flex items-center justify-center shrink-0 text-cyan-400 font-bold text-xs group-hover:scale-105 transition-transform">
                🔍
              </div>
              <div class="truncate">
                <div class="text-xs font-semibold text-zinc-200 group-hover:text-white transition-colors">Perplexity</div>
                <div class="text-[10px] text-zinc-500 truncate">perplexity.ai</div>
              </div>
            </button>

            <button @click="openUrlInActiveTab('https://accounts.google.com')" class="flex items-center space-x-3 p-3 bg-[#101014] hover:bg-[#16161c] border border-zinc-800/90 hover:border-zinc-700 rounded-xl transition-all group text-left shadow-sm">
              <div class="w-8 h-8 rounded-lg bg-red-500/10 border border-red-500/20 flex items-center justify-center shrink-0 text-red-400 font-bold text-sm group-hover:scale-105 transition-transform">
                G
              </div>
              <div class="truncate">
                <div class="text-xs font-semibold text-zinc-200 group-hover:text-white transition-colors">Google Аккаунт</div>
                <div class="text-[10px] text-zinc-500 truncate">accounts.google.com</div>
              </div>
            </button>

          </div>
        </div>

        <!-- СТАТУС-БАР -->
        <div class="grid grid-cols-2 sm:grid-cols-4 gap-2 w-full pt-4 border-t border-zinc-800/60 text-[11px] text-zinc-400">
          <div class="flex items-center space-x-1.5 p-2 bg-[#101014] rounded-xl border border-zinc-800/60">
            <span class="text-xs">🌐</span>
            <span class="truncate">Chromium Core</span>
          </div>
          <div class="flex items-center space-x-1.5 p-2 bg-[#101014] rounded-xl border border-zinc-800/60">
            <span class="text-xs">🛡️</span>
            <span class="truncate">Remote DNS</span>
          </div>
          <div class="flex items-center space-x-1.5 p-2 bg-[#101014] rounded-xl border border-zinc-800/60">
            <span class="text-xs">☁️</span>
            <span class="truncate">Zero-Knowledge</span>
          </div>
          <div class="flex items-center space-x-1.5 p-2 bg-[#101014] rounded-xl border border-zinc-800/60">
            <span class="text-xs">🧹</span>
            <span class="truncate">Zero-Footprint</span>
          </div>
        </div>

      </div>
    </div>

  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '../stores/auth.store';
import { useBrowserStore } from '../stores/browser.store';

const authStore = useAuthStore();
const browserStore = useBrowserStore();

const inputUrl = ref('');
const startPageInput = ref('');
const failedFavicons = ref(new Set());

const isCurrentStartPage = computed(() => {
  const activeTab = browserStore.tabs.find(t => t.id === browserStore.activeTabId);
  return !activeTab || !activeTab.url || activeTab.url === '' || activeTab.url === 'about:blank';
});

let backupTimeout = null;
const scheduleBackup = () => {
  if (backupTimeout) clearTimeout(backupTimeout);
  backupTimeout = setTimeout(() => {
    if (authStore.accessToken && authStore.isProxyReady) {
      authStore.backupSession();
    }
  }, 1000);
};

watch(() => browserStore.tabs, () => {
  scheduleBackup();
}, { deep: true });

watch(() => browserStore.activeTabId, (newId) => {
  const activeTab = browserStore.tabs.find(t => t.id === newId);
  if (activeTab) {
    inputUrl.value = activeTab.url || '';
    startPageInput.value = '';
    if (activeTab.url && activeTab.url !== 'about:blank') {
      invoke('native_navigate_tab', { tabId: newId, url: activeTab.url }).catch(console.error);
    } else {
      invoke('native_switch_tab', { tabId: newId }).catch(console.error);
    }
  }
}, { immediate: true });

onMounted(() => {
  const activeTab = browserStore.tabs.find(t => t.id === browserStore.activeTabId);
  if (activeTab && activeTab.url && activeTab.url !== 'about:blank') {
    invoke('native_navigate_tab', { tabId: activeTab.id, url: activeTab.url }).catch(console.error);
  }
});

const navigate = () => {
  processNavigation(browserStore.activeTabId, inputUrl.value);
};

const handleStartPageSearch = () => {
  processNavigation(browserStore.activeTabId, startPageInput.value);
};

const openUrlInActiveTab = (url) => {
  processNavigation(browserStore.activeTabId, url);
};

const processNavigation = async (tabId, targetUrl) => {
  let url = targetUrl.trim();
  if (!url) return;
  if (!/^https?:\/\//i.test(url) && url.includes('.')) {
    url = 'https://' + url;
  } else if (!url.includes('.')) {
    url = `https://duckduckgo.com/?q=${encodeURIComponent(url)}`;
  }

  const tab = browserStore.tabs.find(t => t.id === tabId);
  if (!tab) return;

  tab.url = url;
  tab.title = extractHostname(url);
  if (tabId === browserStore.activeTabId) inputUrl.value = url;

  // Передаем управление в нативный движок Chromium (WebView2)
  try {
    await invoke('native_navigate_tab', { tabId, url });
  } catch (err) {
    console.error('Ошибка навигации нативного Webview:', err);
  }
};

const setActiveTab = async (id) => {
  browserStore.setActiveTab(id);
  try {
    await invoke('native_switch_tab', { tabId: id });
  } catch (_) {}
};

const closeTab = async (id) => {
  try {
    await invoke('native_close_tab', { tabId: id });
  } catch (_) {}
  browserStore.closeTab(id);
  const active = browserStore.tabs.find(t => t.id === browserStore.activeTabId);
  if (active) {
    if (!active.url || active.url === 'about:blank') {
      try { await invoke('native_navigate_tab', { tabId: active.id, url: '' }); } catch (_) {}
    } else {
      try { await invoke('native_switch_tab', { tabId: active.id }); } catch (_) {}
    }
  }
};

const goBack = () => {
  invoke('native_go_back', { tabId: browserStore.activeTabId }).catch(console.error);
};

const goForward = () => {
  invoke('native_go_forward', { tabId: browserStore.activeTabId }).catch(console.error);
};

const reloadCurrentTab = () => {
  invoke('native_reload', { tabId: browserStore.activeTabId }).catch(console.error);
};

const addNewTab = () => {
  browserStore.addTab('');
};

const getTabFavicon = (_tab) => {
  return null;
};

const onFaviconError = (tab) => {
  if (tab && tab.id) failedFavicons.value.add(tab.id);
};

const getTabTitle = (tab) => {
  if (!tab.url || tab.url === 'about:blank') return 'Новая вкладка';
  return tab.title || extractHostname(tab.url);
};

const extractHostname = (urlStr) => {
  try {
    const u = new URL(urlStr);
    return u.hostname.replace('www.', '');
  } catch (e) {
    return urlStr;
  }
};

const handleLogout = async () => {
  await authStore.logout();
};
</script>