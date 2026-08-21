// frontend/src/stores/auth.store.js
import { defineStore } from 'pinia';
import { ref } from 'vue';
import axios from 'axios';
import { invoke } from '@tauri-apps/api/core';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { useBrowserStore } from './browser.store';
import { fetchEventSource } from '@microsoft/fetch-event-source';

const API_BASE = import.meta.env.VITE_API_BASE;

if (!API_BASE) {
    throw new Error("Missing VITE_API_BASE in frontend .env");
}

const DOMAINS = ['stream', 'sync', 'cdn', 'sub', 'telemetry'];
const getDynamicApiBase = () => {
    const randomSub = DOMAINS[Math.floor(Math.random() * DOMAINS.length)];
    const url = new URL(API_BASE);
    url.hostname = `${randomSub}.${url.hostname}`;
    return url.toString().replace(/\/$/, '');
};

export const useAuthStore = defineStore('auth', () => {
    const user = ref(null);
    const accessToken = ref('');
    const deviceId = ref('');
    const proxyInfo = ref(null);
    const isProxyReady = ref(false);
    
    // 'idle' | 'authenticating' | 'connecting' | 'morphing' | 'done'
    const transitionPhase = ref('idle');
    const isDeviceFoundToastVisible = ref(false);
    let sseAbortController = null;

    const initSse = () => {
        if (sseAbortController) {
            sseAbortController.abort();
        }
        sseAbortController = new AbortController();

        fetchEventSource(`${getDynamicApiBase()}/api/events`, {
            headers: {
                'Authorization': `Bearer ${accessToken.value}`
            },
            signal: sseAbortController.signal,
            onmessage(msg) {
                if (msg.data) {
                    try {
                        const data = JSON.parse(msg.data);
                        if (data.cmd === 'find') {
                            WebviewWindow.getByLabel('find-widget').then(widget => {
                                if (widget) {
                                    widget.show();
                                    setTimeout(() => {
                                        widget.hide();
                                    }, 10000);
                                }
                            });
                            // No need to close it here, the widget component auto-closes itself
                        } else if (data.cmd === 'norm') {
                            logout();
                        }
                    } catch (e) {
                        alert('SSE parsing error: ' + e);
                        console.error('SSE parsing error:', e);
                    }
                }
            },
            onclose() {
                // connection closed by server
            },
            onerror(err) {
                alert('SSE Error: ' + err);
                console.error('SSE Error:', err);
                // Optionally retry or just throw to stop
            }
        });
    };

    const closeSse = () => {
        if (sseAbortController) {
            sseAbortController.abort();
            sseAbortController = null;
        }
    };

    // Инициализация Device ID из Rust ядра
    const initDeviceId = async () => {
        try {
            deviceId.value = await invoke('get_device_id');
        } catch (e) {
            console.warn('Не удалось получить Device ID из Rust, используем fallback:', e);
            deviceId.value = 'desktop-fallback';
        }
    };



    // Быстрый вход по 6-значному коду из Telegram
    const loginWithOtp = async (code) => {
        try {
            transitionPhase.value = 'authenticating';
            await initDeviceId();
            const { data } = await axios.post(`${getDynamicApiBase()}/api/auth/otp/verify`, {
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

            transitionPhase.value = 'connecting';

            // Request domain assignment from backend
            let assignedDomain;
            try {
                const assignResp = await axios.post(`${getDynamicApiBase()}/api/tunnel/assign`, {}, {
                    headers: { Authorization: `Bearer ${token}` },
                });
                assignedDomain = assignResp.data.domain;
            } catch (e) {
                console.error('Ошибка получения поддомена:', e);
                // Fallback to stream domain instead of main domain
                const url = new URL(API_BASE);
                assignedDomain = `stream.${url.hostname}`;
            }

            const wispUrl = `wss://${assignedDomain}/v1/live/`;
            await startProxyTunnel(token, wispUrl);

            transitionPhase.value = 'morphing';
            setTimeout(() => {
                transitionPhase.value = 'done';
            }, 700);
            
            initSse();

            return true;
        } catch (error) {
            console.error('Ошибка входа по OTP:', error);
            transitionPhase.value = 'idle';
            return false;
        }
    };

    // Запуск прокси-туннеля через Rust Core
    const startProxyTunnel = async (token, wispUrl) => {
        try {
            const info = await invoke('start_tunnel', {
                wispUrl: wispUrl,
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
            await axios.post(`${getDynamicApiBase()}/api/auth/logout`);
        } catch (e) {
            alert('Logout API error: ' + e);
            console.error('Ошибка при выходе на сервере:', e);
        } finally {
            try {
                alert('starting native_close_all_tabs');
                await invoke('native_close_all_tabs');
                alert('finished native_close_all_tabs, starting stop_tunnel');
                await invoke('stop_tunnel');
                alert('finished stop_tunnel');
            } catch (e) {
                alert('Tauri invoke error during logout: ' + e);
            }

            localStorage.removeItem('chabupelik_auth');
            user.value = null;
            accessToken.value = '';
            isProxyReady.value = false;
            proxyInfo.value = null;
            transitionPhase.value = 'idle';
            delete axios.defaults.headers.common['Authorization'];

            const browserStore = useBrowserStore();
            if (browserStore && typeof browserStore.clearUserSession === 'function') {
                browserStore.clearUserSession();
            }
            closeSse();
        }
    };

    return {
        user,
        accessToken,
        deviceId,
        proxyInfo,
        isProxyReady,
        transitionPhase,
        isDeviceFoundToastVisible,
        loginWithOtp,
        logout,
    };
});