// frontend/src/stores/auth.store.js
import { defineStore } from 'pinia';
import { ref } from 'vue';
import axios from 'axios';
import { useCrypto } from '../composables/useCrypto';
import { useBrowserStore } from './browser.store'; // <--- КРИТИЧЕСКИ ВАЖНЫЙ ИМПОРТ

export const useAuthStore = defineStore('auth', () => {
    const user = ref(null);
    const accessToken = ref('');
    const aesKey = ref(null); // Ключ хранится ТОЛЬКО в ОЗУ

    const { deriveKey, encryptData } = useCrypto();

    const login = async (username, password) => {
        try {
            // 1. Авторизация на бэкенде (получение JWT)
            const { data } = await axios.post('/api/auth/login', { username, password });
            accessToken.value = data.accessToken;
            user.value = data.user;

            // 2. Генерация ключа шифрования на основе пароля локально
            aesKey.value = await deriveKey(password);

            // Настраиваем axios для передачи токена
            axios.defaults.headers.common['Authorization'] = `Bearer ${data.accessToken}`;

            return true;
        } catch (error) {
            console.error('Ошибка входа:', error);
            return false;
        }
    };

    const logout = async () => {
        try {
            // Отправляем финальный зашифрованный бэкап перед выходом (Panic Button)
            await backupSession();
            await axios.post('/api/auth/logout');
        } catch (e) {
            console.error('Ошибка сети при выходе:', e);
        } finally {

            // 1. Удаляем куки и сессии сайтов Scramjet из IndexedDB
            try {
                if (window.indexedDB && window.indexedDB.deleteDatabase) {
                    window.indexedDB.deleteDatabase('$scramjet');
                }
            } catch (e) {
                console.error('Ошибка удаления IndexedDB Scramjet:', e);
            }

            // 2. Сбрасываем стор вкладок браузера
            try {
                const browserStore = useBrowserStore();
                if (browserStore && typeof browserStore.clearUserSession === 'function') {
                    await browserStore.clearUserSession();
                }
            } catch (e) {
                console.error('Ошибка очистки стора браузера:', e);
            }

            // Тотальная зачистка ОЗУ и хранилищ
            user.value = null;
            accessToken.value = '';
            aesKey.value = null;
            localStorage.clear();
            sessionStorage.clear();
            delete axios.defaults.headers.common['Authorization'];

            // Перезагрузка страницы
            window.location.reload();
        }
    };

    // Зашифрованный бэкап реальных сессий и вкладок перед выходом
    const backupSession = async () => {
        if (!aesKey.value) return;

        try {
            const browserStore = useBrowserStore();
            
            // Собираем НАСТОЯЩИЕ открытые вкладки пользователя
            const realOpenTabs = browserStore.tabs.map(tab => ({
                url: tab.url,
                title: tab.title
            }));

            const sessionData = {
                cookies_backup: "encrypted_session_data",
                open_tabs: realOpenTabs
            };

            const encrypted = await encryptData(sessionData, aesKey.value);
            await axios.post('/api/sync', {
                encrypted_cookies: encrypted.payload,
                open_tabs: realOpenTabs
            });
        } catch (e) {
            console.error('Ошибка создания бэкапа сессии:', e);
        }
    };

    return { user, accessToken, aesKey, login, logout, backupSession };
});