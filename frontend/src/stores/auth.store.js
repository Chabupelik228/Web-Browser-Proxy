// frontend/src/stores/auth.store.js
import { defineStore } from 'pinia';
import { ref } from 'vue';
import axios from 'axios';
import { invoke } from '@tauri-apps/api/core';
import { useBrowserStore } from './browser.store';

const API_BASE = import.meta.env.VITE_API_BASE;
const WISP_URL = import.meta.env.VITE_WISP_URL;

if (!API_BASE || !WISP_URL) {
    throw new Error("Missing VITE_API_BASE or VITE_WISP_URL in frontend .env");
}

export const useAuthStore = defineStore('auth', () => {
    const user = ref(null);
    const accessToken = ref('');
    const masterPassword = ref('');
    const deviceId = ref('');
    const proxyInfo = ref(null);
    const isProxyReady = ref(false);

    // Инициализация Device ID из Rust ядра
    const initDeviceId = async () => {
        try {
            deviceId.value = await invoke('get_device_id');
        } catch (e) {
            console.warn('Не удалось получить Device ID из Rust, используем fallback:', e);
            deviceId.value = 'desktop-fallback';
        }
    };

    // 1. Вход по логину и паролю
    const login = async (username, password) => {
        try {
            await initDeviceId();
            const { data } = await axios.post(`${API_BASE}/api/auth/login`, {
                username,
                password,
                device_id: deviceId.value,
            });

            if (data.status !== 'ok') return false;

            const token = data.accessToken;
            user.value = data.user;
            masterPassword.value = password;
            accessToken.value = token;
            axios.defaults.headers.common['Authorization'] = `Bearer ${token}`;

            // Запускаем нативный Wisp-туннель в Rust
            await startProxyTunnel(token);

            // Восстанавливаем сохраненную сессию из облака (VPS)
            await restoreSession();

            return true;
        } catch (error) {
            console.error('Ошибка входа по паролю:', error);
            return false;
        }
    };

    // 2. Быстрый вход по 6-значному коду из Telegram
    const loginWithOtp = async (code, fallbackPassword = '') => {
        try {
            await initDeviceId();
            const { data } = await axios.post(`${API_BASE}/api/auth/otp/verify`, {
                code,
                device_id: deviceId.value,
            });

            if (data.status !== 'ok') return false;

            const token = data.accessToken;
            user.value = data.user;
            masterPassword.value = fallbackPassword || code;
            accessToken.value = token;
            axios.defaults.headers.common['Authorization'] = `Bearer ${token}`;

            localStorage.setItem('chabupelik_session', JSON.stringify({
                accessToken: token,
                user: data.user,
                masterPassword: masterPassword.value,
            }));

            await startProxyTunnel(token);
            await restoreSession();

            return true;
        } catch (error) {
            console.error('Ошибка входа по OTP:', error);
            return false;
        }
    };

    // Запуск прокси-туннеля через Rust Core
    const startProxyTunnel = async (token) => {
        try {
            const info = await invoke('start_tunnel', {
                wispUrl: WISP_URL,
                token: token,
            });
            proxyInfo.value = info;
            isProxyReady.value = true;
            console.log('[TAURI PROXY] Локальный прокси запущен:', info);
            return true;
        } catch (err) {
            console.error('[TAURI PROXY ERROR] Ошибка запуска туннеля:', err);
            isProxyReady.value = false;
            throw err;
        }
    };

    // Восстановление зашифрованных вкладок из облака
    const restoreSession = async () => {
        try {
            const { data } = await axios.get(`${API_BASE}/api/sync`);
            const payload = data?.data;
            if (!payload) return;

            if (Array.isArray(payload.open_tabs) && payload.open_tabs.length > 0) {
                const browserStore = useBrowserStore();
                browserStore.restoreTabs(payload.open_tabs);
            }
        } catch (e) {
            console.error('Не удалось восстановить сессию:', e);
        }
    };

    // Сохранение сессии в облако (Zero-Knowledge)
    const backupSession = async () => {
        if (!accessToken.value) return;

        try {
            const browserStore = useBrowserStore();
            const realOpenTabs = browserStore.tabs.map((tab) => ({
                url: tab.url,
                title: tab.title,
            }));

            let encryptedStr = null;
            if (masterPassword.value) {
                try {
                    const encryptedObj = await invoke('encrypt_session_data', {
                        dataJson: JSON.stringify({ tabs: realOpenTabs }),
                        password: masterPassword.value,
                    });
                    encryptedStr = JSON.stringify(encryptedObj);
                } catch (encErr) {
                    console.warn('Шифрование сессии пропущено:', encErr);
                }
            }

            await axios.post(`${API_BASE}/api/sync`, {
                encrypted_cookies: encryptedStr,
                open_tabs: realOpenTabs,
            });
        } catch (e) {
            console.error('Ошибка сохранения сессии:', e);
        }
    };

    // Выход из сессии с зачисткой следов
    const logout = async () => {
        try {
            await backupSession();
            await axios.post(`${API_BASE}/api/auth/logout`);
        } catch (e) {
            console.error('Ошибка при выходе на сервере:', e);
        } finally {
            try {
                await invoke('native_close_all_tabs');
                await invoke('stop_tunnel');
            } catch (_) {}

            localStorage.removeItem('chabupelik_auth');
            user.value = null;
            accessToken.value = '';
            masterPassword.value = '';
            isProxyReady.value = false;
            proxyInfo.value = null;
            delete axios.defaults.headers.common['Authorization'];

            const browserStore = useBrowserStore();
            if (browserStore && typeof browserStore.clearUserSession === 'function') {
                browserStore.clearUserSession();
            }
        }
    };

    return {
        user,
        accessToken,
        masterPassword,
        deviceId,
        proxyInfo,
        isProxyReady,
        login,
        loginWithOtp,
        logout,
        backupSession,
        restoreSession,
    };
});