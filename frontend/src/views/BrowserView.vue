<template>
  <div class="flex flex-col h-full bg-[#0f0f11] text-zinc-200 select-none font-sans">
    
    <!-- 1. ПАНЕЛЬ ВКЛАДОК -->
    <div class="flex items-center bg-[#141417] px-2 pt-2 overflow-x-auto border-b border-zinc-800/80">
      <div 
        v-for="tab in browserStore.tabs" 
        :key="tab.id"
        @click="browserStore.setActiveTab(tab.id)"
        class="flex items-center justify-between px-3 py-1.5 w-44 text-xs cursor-pointer rounded-t-md border-t border-x transition-all mr-1 group relative"
        :class="tab.id === browserStore.activeTabId ? 'bg-[#1c1c20] text-zinc-100 border-zinc-700/80 shadow-sm font-medium' : 'bg-[#141417] text-zinc-500 border-transparent hover:bg-[#18181c] hover:text-zinc-300'"
      >
        <div class="flex items-center space-x-2 truncate pr-2">
          <!-- Спиннер загрузки -->
          <div v-if="tab.isLoading" class="w-3 h-3 border-2 border-zinc-400 border-t-transparent rounded-full animate-spin flex-shrink-0"></div>
          
          <!-- Аватарка сайта (Проксируется через Scramjet для обхода CORS/COEP) -->
          <img 
            v-else-if="getTabFavicon(tab)" 
            :src="getTabFavicon(tab)" 
            class="w-3.5 h-3.5 rounded-sm flex-shrink-0 object-contain"
            @error="(e) => onFaviconError(e, tab)"
          />
          
          <!-- Иконка по умолчанию -->
          <svg v-else class="w-3.5 h-3.5 text-zinc-500 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" />
          </svg>
          
          <span class="truncate">{{ getTabTitle(tab) }}</span>
        </div>

        <button 
          @click.stop="browserStore.closeTab(tab.id)" 
          class="text-zinc-500 hover:text-zinc-200 font-bold rounded w-4 h-4 flex items-center justify-center hover:bg-zinc-700/50 transition-colors"
        >
          ×
        </button>
      </div>
      
      <!-- Новая вкладка -->
      <button 
        @click="addNewTab" 
        class="ml-1 text-zinc-500 hover:text-zinc-200 text-base w-6 h-6 flex items-center justify-center rounded hover:bg-zinc-800 transition-colors"
        title="Новая вкладка"
      >
        +
      </button>
      
      <!-- Кнопка Выход -->
      <button 
        @click="handleLogout" 
        class="ml-auto bg-zinc-800 hover:bg-zinc-700 text-zinc-300 font-medium text-[11px] px-3 py-1 rounded transition-all border border-zinc-700/60"
      >
        Выход
      </button>
    </div>

    <!-- 2. АДРЕСНАЯ СТРОКА С НАВИГАЦИЕЙ -->
    <div class="flex items-center space-x-1.5 bg-[#1c1c20] p-2 border-b border-zinc-800/80">
      
      <button @click="goBack" class="p-1.5 text-zinc-400 hover:text-zinc-100 rounded-md hover:bg-zinc-800 transition-colors" title="Назад">
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 12H5"/><path d="M12 19l-7-7 7-7"/></svg>
      </button>

      <button @click="goForward" class="p-1.5 text-zinc-400 hover:text-zinc-100 rounded-md hover:bg-zinc-800 transition-colors" title="Вперед">
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5l7 7-7 7"/></svg>
      </button>

      <button @click="reloadCurrentTab" class="p-1.5 text-zinc-400 hover:text-zinc-100 rounded-md hover:bg-zinc-800 transition-colors" title="Обновить">
        <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/></svg>
      </button>
      
      <form @submit.prevent="navigate" class="flex-1 flex pl-1">
        <div class="relative w-full flex items-center">
          <span class="absolute left-3.5 text-zinc-500">
            <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
            </svg>
          </span>
          <input 
            v-model="inputUrl" 
            type="text" 
            placeholder="Введите URL или поисковый запрос..."
            class="w-full bg-[#141417] text-zinc-200 border border-zinc-800 rounded-md pl-9 pr-4 py-1.5 focus:outline-none focus:border-zinc-600 text-xs transition-all placeholder-zinc-600"
          >
        </div>
      </form>
    </div>

    <!-- 3. ОСНОВНОЙ КОНТЕНТ -->
    <div class="flex-1 bg-[#0f0f11] relative overflow-hidden">
      
      <template v-for="tab in browserStore.tabs" :key="tab.id">
        
        <!-- А) СТАРТОВАЯ СТРАНИЦА -->
        <div v-if="isStartPage(tab)" v-show="tab.id === browserStore.activeTabId" class="w-full h-full flex flex-col items-center justify-center bg-[#0f0f11] px-4">
          <div class="flex flex-col items-center mb-10">
            <div class="flex items-center space-x-2">
              <span class="text-3xl font-semibold tracking-tight text-zinc-100">Chabupelik</span>
              <span class="text-3xl font-light text-zinc-500">Search</span>
            </div>
            <p class="text-[11px] text-zinc-600 mt-1 font-mono uppercase tracking-widest">
              Private Autonomous Engine
            </p>
          </div>

          <div class="w-full max-w-lg">
            <form @submit.prevent="handleStartPageSearch(tab.id)" class="flex items-center bg-[#161619] border border-zinc-800 rounded-lg p-1.5 shadow-xl hover:border-zinc-700 transition-all focus-within:border-zinc-600">
              <span class="pl-3 pr-2 text-zinc-500">
                <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                </svg>
              </span>
              <input v-model="startPageInput" type="text" placeholder="Поиск в сети или введите адрес..." class="w-full bg-transparent text-zinc-200 placeholder-zinc-600 text-xs focus:outline-none px-2 py-1" autofocus />
              <button type="submit" class="bg-zinc-800 hover:bg-zinc-700 text-zinc-200 font-medium text-xs px-3.5 py-1.5 rounded transition-colors ml-1 border border-zinc-700/50">
                Поиск
              </button>
            </form>

            <div class="grid grid-cols-6 gap-3 mt-8">
              <button v-for="bookmark in quickBookmarks" :key="bookmark.name" @click="openUrlInTab(tab.id, bookmark.url)" class="flex flex-col items-center justify-center p-3 rounded-lg bg-[#141417] hover:bg-[#1a1a1e] border border-zinc-800/80 hover:border-zinc-700 transition-all group">
                <div class="w-7 h-7 flex items-center justify-center mb-2 text-zinc-400 group-hover:text-zinc-100 transition-colors">
                  <svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
                    <path :d="bookmark.iconPath" />
                  </svg>
                </div>
                <span class="text-[11px] text-zinc-500 group-hover:text-zinc-300 transition-colors font-medium">{{ bookmark.name }}</span>
              </button>
            </div>
          </div>
        </div>

        <!-- Б) IFRAME ПРОКСИ -->
        <template v-else>
          <div
            :id="`frame-container-${tab.id}`"
            v-show="tab.id === browserStore.activeTabId"
            class="w-full h-full relative overflow-hidden bg-white"
          ></div>
        </template>

      </template>

    </div>
  </div>
</template>

<script setup>
import { ref, watch, nextTick } from 'vue';
import { useAuthStore } from '../stores/auth.store';
import { useBrowserStore } from '../stores/browser.store';

const authStore = useAuthStore();
const browserStore = useBrowserStore();

const inputUrl = ref('');
const startPageInput = ref('');
const failedFavicons = ref(new Set());

watch(() => browserStore.activeTabId, (newId) => {
  const activeTab = browserStore.tabs.find(t => t.id === newId);
  if (activeTab) {
    inputUrl.value = activeTab.url || '';
    startPageInput.value = '';
  }
}, { immediate: true });

const isStartPage = (tab) => !tab.url || tab.url === '' || tab.url === 'about:blank';

const quickBookmarks = [
  { name: 'YouTube', url: 'https://youtube.com', iconPath: 'M23.498 6.186a3.016 3.016 0 0 0-2.122-2.136C19.505 3.545 12 3.545 12 3.545s-7.505 0-9.377.505A3.017 3.017 0 0 0 .502 6.186C0 8.07 0 12 0 12s0 3.93.502 5.814a3.016 3.016 0 0 0 2.122 2.136c1.871.505 9.376.505 9.376.505s7.505 0 9.377-.505a3.015 3.015 0 0 0 2.122-2.136C24 15.93 24 12 24 12s0-3.93-.502-5.814zM9.545 15.568V8.432L15.818 12l-6.273 3.568z' },
  { name: 'Discord', url: 'https://discord.com', iconPath: 'M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994.021-.041.001-.09-.041-.106a13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.061 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.893.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.028zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z' },
  { name: 'Telegram', url: 'https://web.telegram.org', iconPath: 'M12 0C5.37 0 0 5.37 0 12s5.37 12 12 12 12-5.37 12-12S18.63 0 12 0zm5.562 8.161c-.18.717-.962 4.084-1.362 5.411-.168.56-.437.747-.697.766-.566.042-1-.383-1.55-.744-.86-.564-1.346-.913-2.181-1.463-.965-.635-.34-.984.211-1.555.144-.15.266-.279.378-.395 2.47-2.24 2.583-2.348 2.624-2.493.007-.024.014-.112-.039-.158-.052-.047-.129-.031-.186-.018-.08.018-1.353.86-3.819 2.525-.361.247-.688.368-.981.361-.323-.007-.945-.182-1.408-.333-.568-.186-1.02-.284-.98-.6.02-.165.248-.334.685-.508 2.682-1.168 4.472-1.94 5.37-2.317 2.553-1.066 3.085-1.252 3.431-1.258.076 0 .247.018.357.108.093.076.119.179.131.252.013.073.027.24.015.373z' },
  { name: 'Reddit', url: 'https://reddit.com', iconPath: 'M12 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0zm5.01 4.744c.688 0 1.25.561 1.25 1.249a1.25 1.25 0 0 1-2.498.056l-2.597-.547-.8 3.747c1.824.07 3.48.632 4.674 1.488.308-.309.73-.491 1.207-.491.968 0 1.754.786 1.754 1.754 0 .716-.435 1.333-1.01 1.614a3.111 3.111 0 0 1 .042.52c0 2.694-3.13 4.87-7.004 4.87-3.874 0-7.004-2.176-7.004-4.87 0-.183.015-.366.043-.534A1.748 1.748 0 0 1 4.028 12c0-.968.786-1.754 1.754-1.754.463 0 .898.196 1.207.49 1.207-.883 2.878-1.43 4.744-1.487l.885-4.182a.342.342 0 0 1 .14-.197.35.35 0 0 1 .238-.042l2.906.617a1.214 1.214 0 0 1 1.108-.701zM9.25 12C8.561 12 8 12.562 8 13.25c0 .687.561 1.248 1.25 1.248.687 0 1.248-.561 1.248-1.249 0-.688-.561-1.249-1.249-1.249zm5.5 0c-.687 0-1.248.562-1.248 1.25 0 .687.561 1.248 1.249 1.248.688 0 1.249-.561 1.249-1.249 0-.688-.562-1.249-1.25-1.249zm-4.566 3.75a.384.384 0 0 0-.27.653c.801.801 2.115.801 2.916 0a.383.383 0 0 0-.542-.542c-.501.501-1.331.501-1.832 0a.38.38 0 0 0-.272-.111z' },
  { name: 'VK', url: 'https://vk.com', iconPath: 'M15.684 0H8.316C1.592 0 0 1.592 0 8.316v7.368C0 22.408 1.592 24 8.316 24h7.368C22.408 24 24 22.408 24 15.684V8.316C24 1.592 22.408 0 15.684 0zm3.692 17.123h-1.644c-.624 0-.816-.495-1.933-1.614-1.119-1.118-1.615-1.265-1.896-1.265-.392 0-.505.112-.505.648v1.614c0 .408-.135.617-1.22.617-2.012 0-4.243-1.22-5.814-3.486-2.365-3.328-3.007-5.823-3.007-6.331 0-.248.096-.48.56-.48h1.644c.416 0 .568.192.728.648.792 2.304 2.128 4.32 2.68 4.32.208 0 .304-.096.304-.624V9.663c-.064-1.128-.656-1.224-.656-1.632 0-.192.16-.384.416-.384h2.584c.352 0 .48.184.48.592v3.192c0 .344.152.48.248.48.208 0 .384-.136.768-.52 1.184-1.336 2.032-3.376 2.032-3.376.112-.248.304-.48.72-.48h1.644c.496 0 .608.256.496.6-.208.976-2.256 3.864-2.256 3.864-.184.288-.256.416 0 .752.184.248.792.776 1.192 1.24 1.128 1.288 1.488 1.84 1.6 2.08.112.248-.032.616-.544.616z' },
  { name: 'GitHub', url: 'https://github.com', iconPath: 'M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z' }
];

const navigate = () => {
  processNavigation(browserStore.activeTabId, inputUrl.value);
};

const handleStartPageSearch = (tabId) => {
  processNavigation(tabId, startPageInput.value);
};

const openUrlInTab = (tabId, url) => {
  processNavigation(tabId, url);
};

// вместо buildProxyUrl / decodeProxyUrl / setupNewWindowInterceptor / syncTabLocation —
// один ScramjetFrame на вкладку

const scramjetFrames = new Map(); // tabId -> ScramjetFrame

function pushHistory(tab, url) {
  if (!tab.history) tab.history = [];
  if (tab.historyIndex === undefined) tab.historyIndex = -1;

  // не дублируем, если это тот же URL, что уже стоит текущим
  if (tab.history[tab.historyIndex] === url) return;

  tab.history = tab.history.slice(0, tab.historyIndex + 1);
  tab.history.push(url);
  tab.historyIndex = tab.history.length - 1;
}

function ensureFrame(tab, container) {
  if (scramjetFrames.has(tab.id)) return scramjetFrames.get(tab.id);
  if (!window.scramjet || typeof window.scramjet.createFrame !== 'function') {
    console.error('Scramjet ещё не инициализирован');
    return null;
  }

  const frame = window.scramjet.createFrame();
  frame.frame.className = 'w-full h-full border-none absolute inset-0 bg-white';
  container.appendChild(frame.frame);

  frame.addEventListener('urlchange', (e) => {
    if (!e.url) return;
    const url = e.url.toString();
    tab.url = url;
    tab.title = extractHostname(url);
    tab.isLoading = false;
    if (tab.id === browserStore.activeTabId) inputUrl.value = url;

    pushHistory(tab, url); // фиксируем в СВОЮ историю, не полагаемся на iframe history
  });

  scramjetFrames.set(tab.id, frame);
  return frame;
}

const buildProxyUrl = (url) => {
  if (!url || url === 'about:blank' || url.trim() === '') return '';
  if (window.scramjet && typeof window.scramjet.encodeUrl === 'function') {
    return window.scramjet.encodeUrl(url);
  }
  return '';
};

const processNavigation = async (tabId, targetUrl) => {
  let url = targetUrl.trim();
  if (!url) return;
  if (!/^https?:\/\//i.test(url) && url.includes('.')) {
    url = 'https://' + url;
  } else if (!url.includes('.')) {
    url = `https://duckduckgo.com/?q=${encodeURIComponent(url)}`;
  }

  url = normalizeUrl(url);

  const tab = browserStore.tabs.find(t => t.id === tabId);
  if (!tab) return;

  tab.isLoading = true;
  tab.url = url;
  tab.title = extractHostname(url);
  if (tabId === browserStore.activeTabId) inputUrl.value = url;

  await nextTick();
  const container = document.getElementById(`frame-container-${tabId}`);
  if (!container) return;

  const frame = ensureFrame(tab, container);
  if (!frame) return;

  pushHistory(tab, url); // добавили сюда явный переход тоже
  frame.go(url);
};

const goBack = async () => {
  const tab = browserStore.tabs.find(t => t.id === browserStore.activeTabId);
  if (!tab || !tab.history || tab.historyIndex <= 0) return;
  tab.historyIndex--;
  const url = tab.history[tab.historyIndex];

  const frame = scramjetFrames.get(tab.id);
  if (frame && container) {
    tab.url = url;
    inputUrl.value = url;
    frame.go(url); // просто грузим нужный адрес в ЭТОТ конкретный фрейм
  }
};

const goForward = async () => {
  const tab = browserStore.tabs.find(t => t.id === browserStore.activeTabId);
  if (!tab || !tab.history || tab.historyIndex >= tab.history.length - 1) return;
  tab.historyIndex++;
  const url = tab.history[tab.historyIndex];

  const frame = scramjetFrames.get(tab.id);
  if (frame) {
    tab.url = url;
    inputUrl.value = url;
    frame.go(url);
  }
};

const reloadCurrentTab = () => {
  scramjetFrames.get(browserStore.activeTabId)?.reload();
};

const addNewTab = () => {
  browserStore.addTab('');
};

watch(() => browserStore.tabs.map(t => t.id), (newIds) => {
  for (const id of scramjetFrames.keys()) {
    if (!newIds.includes(id)) scramjetFrames.delete(id);
  }
});

// Забор иконок (Проксируется через Scramjet для обхода CORS)
const getTabFavicon = (tab) => {
  if (isStartPage(tab) || failedFavicons.value.has(tab.id)) return null;
  try {
    const host = extractHostname(tab.url);
    if (!host || host.length < 3) return null;
    return buildProxyUrl(`https://www.google.com/s2/favicons?domain=${host}&sz=32`);
  } catch (e) {
    return null;
  }
};

const normalizeUrl = (urlStr) => {
  try {
    if (/(^|\.)youtube\.com$/.test(new URL(urlStr).hostname) || urlStr.includes('youtu.be')) {
      return urlStr
        .replace(/(www\.)?youtube\.com/, 'm.youtube.com')
        .replace('youtu.be/', 'm.youtube.com/watch?v=');
    }
  } catch (e) {}
  return urlStr;
};

const onFaviconError = (event, tab) => {
  if (tab && tab.id) failedFavicons.value.add(tab.id);
};

const getTabTitle = (tab) => {
  if (isStartPage(tab)) return 'Новая вкладка';
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