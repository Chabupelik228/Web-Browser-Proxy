// frontend/src/stores/browser.store.js
import { defineStore } from 'pinia';
import { ref, watch } from 'vue';
import { useAuthStore } from './auth.store';

export const useBrowserStore = defineStore('browser', () => {
  const authStore = useAuthStore();

  // Динамический ключ в зависимости от логина пользователя
  const getUserKey = (key) => {
    const username = authStore.user?.username || authStore.username || 'guest';
    return `chabupelik_${username}_${key}`;
  };

  let tabIdCounter = 0;
  const generateTabId = () => `${Date.now()}_${tabIdCounter++}`;

  const loadSavedTabs = () => {
    const saved = localStorage.getItem(getUserKey('tabs'));
    return saved ? JSON.parse(saved) : [{ id: generateTabId(), url: '', title: 'Новая вкладка' }];
  };

  const loadSavedActiveId = () => {
    const saved = localStorage.getItem(getUserKey('active_tab'));
    return saved ? Number(saved) : null;
  };

  const tabs = ref(loadSavedTabs());
  const activeTabId = ref(loadSavedActiveId() || tabs.value[0]?.id);

  // Автосохранение вкладок именно под текущим пользователем
  watch(tabs, (newTabs) => {
    localStorage.setItem(getUserKey('tabs'), JSON.stringify(newTabs));
  }, { deep: true });

  watch(activeTabId, (newId) => {
    if (newId) localStorage.setItem(getUserKey('active_tab'), String(newId));
  });

  const addTab = (url = '') => {
    const newTab = { id: generateTabId(), url, title: 'Новая вкладка', isLoading: false };
    tabs.value.push(newTab);
    activeTabId.value = newTab.id;
  };

  const closeTab = (id) => {
    const index = tabs.value.findIndex(t => t.id === id);
    if (index === -1) return;

    tabs.value.splice(index, 1);

    if (tabs.value.length === 0) {
      addTab('');
    } else if (activeTabId.value === id) {
      activeTabId.value = tabs.value[Math.max(0, index - 1)].id;
    }
  };

  const setActiveTab = (id) => {
    activeTabId.value = id;
  };

  // Метод полной зачистки при выходе
  const clearUserSession = async () => {
    // 1. Очищаем сохраненные вкладки в localStorage
    localStorage.removeItem(getUserKey('tabs'));
    localStorage.removeItem(getUserKey('active_tab'));

    // 2. Сбрасываем вкладки в памяти
    tabs.value = [{ id: generateTabId(), url: '', title: 'Новая вкладка' }];
    activeTabId.value = tabs.value[0].id;

    // 3. Полностью удаляем куки и сессии сайтов из IndexedDB
    try {
      if (window.indexedDB && window.indexedDB.deleteDatabase) {
        window.indexedDB.deleteDatabase('$scramjet');
      }
    } catch (e) {
      console.error('Ошибка очистки куки:', e);
    }
  };

  const restoreTabs = (savedTabs) => {
    if (!Array.isArray(savedTabs) || savedTabs.length === 0) return;
    tabs.value = savedTabs.map(t => ({
      id: generateTabId(),
      url: t.url || '',
      title: t.title || 'Новая вкладка',
      isLoading: false,
    }));
    activeTabId.value = tabs.value[0].id;
  };

  return { tabs, activeTabId, addTab, closeTab, setActiveTab, clearUserSession, restoreTabs };
});