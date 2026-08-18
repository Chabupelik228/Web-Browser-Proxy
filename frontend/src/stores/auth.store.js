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
            accessToken.value = token;
            axios.defaults.headers.common['Authorization'] = `Bearer ${token}`;

            // Запускаем нативный Wisp-туннель в Rust
            await startProxyTunnel(token);

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
            accessToken.value = token;
            axios.defaults.headers.common['Authorization'] = `Bearer ${token}`;

            localStorage.setItem('chabupelik_session', JSON.stringify({
                accessToken: token,
                user: data.user,
            }));

            await startProxyTunnel(token);

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



    // Выход из сессии с зачисткой следов
    const logout = async () => {
        try {
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
        deviceId,
        proxyInfo,
        isProxyReady,
        login,
        loginWithOtp,
        logout,
    };
});