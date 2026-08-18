<template>
  <div class="w-full h-full bg-[#0c0c11]/95 backdrop-blur-2xl flex flex-col font-sans select-none overflow-hidden rounded-2xl border border-zinc-700/70 shadow-[0_20px_50px_rgba(0,0,0,0.6)] text-zinc-200">
    <!-- Header -->
    <div class="flex items-center justify-between px-3.5 py-3 border-b border-zinc-800/80 bg-[#121218]/90" data-tauri-drag-region>
      <div class="flex items-center space-x-2 pointer-events-none">
        <div class="w-6 h-6 rounded-lg bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400">
          <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
          </svg>
        </div>
        <span class="font-semibold text-xs text-zinc-100">Загрузки сессии</span>
        <span 
          v-if="downloads.length > 0"
          class="text-[10px] font-bold px-1.5 py-0.5 rounded-full bg-zinc-800 border border-zinc-700 text-zinc-400"
        >
          {{ downloads.length }}
        </span>
      </div>

      <div class="flex items-center space-x-1.5">
        <button 
          v-if="downloads.length > 0"
          @click="clearDownloads" 
          class="flex items-center space-x-1 text-[11px] text-zinc-400 hover:text-red-400 hover:bg-red-500/10 px-2 py-1 rounded-lg transition-all"
          title="Очистить список"
        >
          <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
          <span>Очистить</span>
        </button>
        <button 
          @click="closeWidget" 
          class="w-6 h-6 flex items-center justify-center text-zinc-400 hover:text-zinc-100 hover:bg-zinc-800/80 rounded-lg transition-colors"
          title="Закрыть"
        >
          <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>

    <!-- Список файлов -->
    <div class="flex-1 overflow-y-auto space-y-2 p-2.5 bg-[#0e0e14]/50 custom-scrollbar">
      <!-- Empty State -->
      <div v-if="downloads.length === 0" class="h-full min-h-[260px] flex flex-col items-center justify-center text-center p-6 space-y-3">
        <div class="w-12 h-12 rounded-2xl bg-zinc-900 border border-zinc-800/80 flex items-center justify-center text-zinc-500 shadow-inner">
          <svg class="w-6 h-6 stroke-[1.5]" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M9 19l3 3m0 0l3-3m-3 3V10" />
          </svg>
        </div>
        <div>
          <p class="text-xs font-medium text-zinc-300">Нет активных загрузок</p>
          <p class="text-[11px] text-zinc-500 mt-0.5 max-w-[200px]">Скачанные в текущей сессии файлы появятся в этом списке</p>
        </div>
      </div>

      <!-- File item card -->
      <div 
        v-for="item in downloads" 
        :key="item.id"
        class="p-2.5 bg-[#14141d]/90 hover:bg-[#1a1a26] border border-zinc-800/80 hover:border-zinc-700/80 rounded-xl transition-all duration-150 flex items-center justify-between group shadow-sm"
      >
        <div class="flex items-center space-x-2.5 min-w-0 pr-2">
          <!-- File Category Icon -->
          <div 
            class="w-8 h-8 rounded-xl flex items-center justify-center shrink-0 border"
            :class="getFileTheme(item.filename)"
          >
            <!-- Executable / App -->
            <svg v-if="getFileType(item.filename) === 'app'" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
            </svg>
            <!-- Archive -->
            <svg v-else-if="getFileType(item.filename) === 'archive'" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4" />
            </svg>
            <!-- Media / Video / Audio -->
            <svg v-else-if="getFileType(item.filename) === 'media'" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <!-- Image -->
            <svg v-else-if="getFileType(item.filename) === 'image'" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" />
            </svg>
            <!-- Document / PDF -->
            <svg v-else-if="getFileType(item.filename) === 'doc'" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
            <!-- Generic File -->
            <svg v-else class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
            </svg>
          </div>

          <div class="min-w-0">
            <p class="font-medium text-zinc-200 text-xs truncate max-w-[170px]" :title="item.filename">
              {{ item.filename }}
            </p>
            <div class="flex items-center space-x-1.5 text-[10px] text-zinc-500 mt-0.5">
              <span>{{ item.time }}</span>
              <span>•</span>
              <span class="text-emerald-400 font-medium">{{ item.size || 'Завершено' }}</span>
            </div>
          </div>
        </div>

        <!-- Action buttons -->
        <div class="flex items-center space-x-1 shrink-0">
          <button 
            @click="openDownloadedFile(item.path)" 
            class="flex items-center space-x-1 px-2.5 py-1 bg-zinc-800 hover:bg-emerald-500 hover:text-zinc-950 text-zinc-300 rounded-lg text-[11px] font-medium transition-all duration-150 shadow-sm"
            title="Открыть файл"
          >
            <svg class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664z" />
            </svg>
            <span>Открыть</span>
          </button>
          <button 
            @click="showDownloadedFileInFolder(item.path)" 
            class="p-1.5 bg-zinc-800/60 hover:bg-zinc-700 text-zinc-400 hover:text-zinc-100 rounded-lg transition-colors"
            title="Показать в папке"
          >
            <svg class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

const downloads = ref([]);
const appWindow = getCurrentWindow();
let channel = null;
let unlistenBlur = null;

onMounted(async () => {
  channel = new BroadcastChannel('downloads-channel');
  channel.onmessage = (e) => {
    if (e.data.type === 'sync') {
      downloads.value = e.data.data || [];
    } else if (e.data.type === 'close-widget') {
      closeWidget();
    }
  };

  // Запрашиваем актуальные данные при старте
  channel.postMessage({ type: 'request-sync' });
  
  // Закрытие при потере фокуса (клик вне виджета)
  window.addEventListener('blur', handleBlur);
  window.addEventListener('keydown', handleKeydown);

  try {
    unlistenBlur = await appWindow.listen('tauri://blur', () => {
      closeWidget();
    });
  } catch (err) {
    console.error('Ошибка подписки на blur:', err);
  }
});

onUnmounted(() => {
  if (channel) {
    channel.close();
  }
  if (unlistenBlur) {
    unlistenBlur();
  }
  window.removeEventListener('blur', handleBlur);
  window.removeEventListener('keydown', handleKeydown);
});

const handleBlur = () => {
  closeWidget();
};

const handleKeydown = (e) => {
  if (e.key === 'Escape') {
    closeWidget();
  }
};

const closeWidget = () => {
  appWindow.hide();
  if (channel) {
    channel.postMessage({ type: 'widget-closed' });
  }
};

const clearDownloads = () => {
  downloads.value = [];
  if (channel) {
    channel.postMessage({ type: 'clear' });
  }
};

const getFileType = (filename) => {
  const ext = filename?.split('.').pop()?.toLowerCase();
  if (['exe', 'msi', 'bat', 'cmd'].includes(ext)) return 'app';
  if (['zip', 'rar', '7z', 'tar', 'gz'].includes(ext)) return 'archive';
  if (['mp4', 'mkv', 'avi', 'mov', 'webm', 'mp3', 'wav', 'flac'].includes(ext)) return 'media';
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'ico'].includes(ext)) return 'image';
  if (['pdf', 'doc', 'docx', 'txt', 'md', 'json', 'csv', 'xlsx'].includes(ext)) return 'doc';
  return 'file';
};

const getFileTheme = (filename) => {
  const type = getFileType(filename);
  switch (type) {
    case 'app':
      return 'bg-emerald-500/15 border-emerald-500/30 text-emerald-400';
    case 'archive':
      return 'bg-amber-500/15 border-amber-500/30 text-amber-400';
    case 'media':
      return 'bg-purple-500/15 border-purple-500/30 text-purple-400';
    case 'image':
      return 'bg-blue-500/15 border-blue-500/30 text-blue-400';
    case 'doc':
      return 'bg-cyan-500/15 border-cyan-500/30 text-cyan-400';
    default:
      return 'bg-zinc-800 border-zinc-700 text-zinc-400';
  }
};

const openDownloadedFile = async (path) => {
  try {
    await invoke('native_open_path', { path });
  } catch (e) {
    console.error('Ошибка открытия файла:', e);
  }
};

const showDownloadedFileInFolder = async (path) => {
  try {
    await invoke('native_show_in_folder', { path });
  } catch (e) {
    console.error('Ошибка показа файла в папке:', e);
  }
};
</script>

<style scoped>
.custom-scrollbar::-webkit-scrollbar {
  width: 5px;
}
.custom-scrollbar::-webkit-scrollbar-track {
  background: transparent;
}
.custom-scrollbar::-webkit-scrollbar-thumb {
  background: #27272a;
  border-radius: 9999px;
}
.custom-scrollbar::-webkit-scrollbar-thumb:hover {
  background: #3f3f46;
}
</style>
