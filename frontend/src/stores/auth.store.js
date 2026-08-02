// frontend/src/stores/auth.store.js
import { defineStore } from 'pinia';
import { ref } from 'vue';
import axios from 'axios';
import { useCrypto } from '../composables/useCrypto';
import { exportScramjetCookies, importScramjetCookies } from '../composables/useScramjetStorage';
import { useBrowserStore } from './browser.store';

export const useAuthStore = defineStore('auth', () => {
    const user = ref(null);
    const accessToken = ref('');
    const aesKey = ref(null);

    const { deriveKey, encryptData, decryptData } = useCrypto(); // <-- decryptData нужен реально

    const login = async (username, password) => {
        try {
            const { data } = await axios.post('/api/auth/login', { username, password });
            accessToken.value = data.accessToken;
            user.value = data.user;
            aesKey.value = await deriveKey(password);
            axios.defaults.headers.common['Authorization'] = `Bearer ${data.accessToken}`;

            await restoreSession(); // <-- новое: подтягиваем бэкап сразу после входа

            return true;
        } catch (error) {
            console.error('Ошибка входа:', error);
            return false;
        }
    };

    // Восстановление куки + вкладок из зашифрованного бэкапа на сервере
    const restoreSession = async () => {
        try {
            const { data } = await axios.get('/api/sync');
            const payload = data?.data;
            if (!payload || !payload.encrypted_cookies) return; // бэкапа ещё нет — первый вход

            const decryptedCookies = await decryptData(payload.encrypted_cookies, aesKey.value);
            if (decryptedCookies) {
                await importScramjetCookies(decryptedCookies);
            }

            const browserStore = useBrowserStore();
            if (Array.isArray(payload.open_tabs) && payload.open_tabs.length > 0) {
                browserStore.restoreTabs(payload.open_tabs);
            }
        } catch (e) {
            console.error('Не удалось восстановить сессию:', e);
            // не критично — просто продолжаем с чистого состояния
        }
    };

    const logout = async () => {
        try {
            await backupSession();
            await axios.post('/api/auth/logout');
        } catch (e) {
            console.error('Ошибка сети при выходе:', e);
        } finally {
            try {
                if (window.indexedDB && window.indexedDB.deleteDatabase) {
                    window.indexedDB.deleteDatabase('$scramjet');
                }
            } catch (e) {
                console.error('Ошибка удаления IndexedDB Scramjet:', e);
            }

            try {
                const browserStore = useBrowserStore();
                if (browserStore && typeof browserStore.clearUserSession === 'function') {
                    await browserStore.clearUserSession();
                }
            } catch (e) {
                console.error('Ошибка очистки стора браузера:', e);
            }

            user.value = null;
            accessToken.value = '';
            aesKey.value = null;
            localStorage.clear();
            sessionStorage.clear();
            delete axios.defaults.headers.common['Authorization'];

            window.location.reload();
        }
    };

    // Зашифрованный бэкап РЕАЛЬНЫХ куки + вкладок перед выходом
    const backupSession = async () => {
        if (!aesKey.value) return;

        try {
            const browserStore = useBrowserStore();
            const realOpenTabs = browserStore.tabs.map(tab => ({
                url: tab.url,
                title: tab.title
            }));

            const realCookies = await exportScramjetCookies(); // <-- настоящие куки, не заглушка
            const encrypted = await encryptData(realCookies, aesKey.value);

            await axios.post('/api/sync', {
                encrypted_cookies: encrypted.payload,
                open_tabs: realOpenTabs,
            });
        } catch (e) {
            console.error('Ошибка создания бэкапа сессии:', e);
        }
    };

    return { user, accessToken, aesKey, login, logout, backupSession };
});