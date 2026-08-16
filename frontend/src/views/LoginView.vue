<template>
  <div class="flex flex-col items-center justify-center min-h-screen bg-[#08080a] text-zinc-100 font-sans select-none p-4 relative overflow-hidden">
    
    <!-- Фоновый паттерн -->
    <div class="absolute inset-0 bg-[radial-gradient(#1c1c24_1px,transparent_1px)] [background-size:24px_24px] opacity-20 pointer-events-none"></div>

    <div class="relative z-10 w-full max-w-3xl bg-[#101014] border border-zinc-800/90 rounded-3xl shadow-[0_25px_60px_rgba(0,0,0,0.75)] overflow-hidden flex flex-col md:flex-row">
      
      <!-- Левая колонка: Форма входа -->
      <div class="w-full md:w-1/2 p-8 flex flex-col justify-center border-b md:border-b-0 md:border-r border-zinc-800/80">
        <div class="flex items-center space-x-3 mb-6">
          <div class="w-10 h-10 bg-[#171720] border border-zinc-700/60 rounded-2xl flex items-center justify-center text-zinc-100 shadow-md">
            <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.75" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
            </svg>
          </div>
          <div>
            <h1 class="text-lg font-bold text-zinc-100 tracking-tight leading-none">Chabupelik Browser</h1>
            <p class="text-xs text-zinc-500 mt-1">Нативный приватный браузер</p>
          </div>
        </div>

        <!-- Переключатель режима входа (OTP vs Пароль) -->
        <div class="flex items-center bg-[#15151d] p-1 rounded-xl border border-zinc-800/90 mb-5">
          <button 
            type="button"
            @click="authMode = 'otp'"
            :class="authMode === 'otp' ? 'bg-[#22222d] text-zinc-100 shadow' : 'text-zinc-500 hover:text-zinc-300'"
            class="flex-1 py-1.5 text-[11px] font-medium rounded-lg transition-all"
          >
            ⚡ Код из бота
          </button>
          <button 
            type="button"
            @click="authMode = 'password'"
            :class="authMode === 'password' ? 'bg-[#22222d] text-zinc-100 shadow' : 'text-zinc-500 hover:text-zinc-300'"
            class="flex-1 py-1.5 text-[11px] font-medium rounded-lg transition-all"
          >
            🔑 Логин и пароль
          </button>
        </div>

        <!-- 1. Вход по 6-значному OTP коду -->
        <form v-if="authMode === 'otp'" @submit.prevent="handleOtpLogin" class="space-y-4">
          <div>
            <label class="block text-[11px] font-medium text-zinc-400 mb-1.5 ml-0.5">
              6-значный одноразовый код
            </label>
            <input 
              v-model="otpCode" 
              type="text" 
              maxlength="6"
              placeholder="Например: 749201"
              class="w-full bg-[#16161d] border border-zinc-800/90 rounded-xl px-4 py-3 text-center tracking-[0.25em] font-mono text-base text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-zinc-500 transition-colors shadow-inner" 
              required
              autofocus
            >
            <p class="text-[10px] text-zinc-500 mt-1.5 ml-0.5">
              Отправьте команду <span class="text-zinc-300 font-mono">/login</span> боту для получения кода.
            </p>
          </div>
          
          <button 
            :disabled="isLoading || otpCode.length !== 6" 
            type="submit" 
            class="w-full bg-zinc-100 hover:bg-white text-zinc-950 font-semibold text-xs py-3 px-4 rounded-xl transition-all disabled:opacity-40 mt-2 shadow-lg active:scale-[0.99]"
          >
            {{ isLoading ? 'Подключение к туннелю...' : 'Войти в браузер' }}
          </button>
        </form>

        <!-- 2. Вход по логину и паролю -->
        <form v-else @submit.prevent="handlePasswordLogin" class="space-y-4">
          <div>
            <label class="block text-[11px] font-medium text-zinc-400 mb-1.5 ml-0.5">Имя пользователя</label>
            <input 
              v-model="username" 
              type="text" 
              placeholder="Username от бота"
              class="w-full bg-[#16161d] border border-zinc-800/90 rounded-xl px-4 py-3 text-xs text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-zinc-500 transition-colors shadow-inner" 
              required
            >
          </div>
          
          <div>
            <label class="block text-[11px] font-medium text-zinc-400 mb-1.5 ml-0.5">Пароль</label>
            <input 
              v-model="password" 
              type="password" 
              placeholder="••••••••"
              class="w-full bg-[#16161d] border border-zinc-800/90 rounded-xl px-4 py-3 text-xs text-zinc-100 placeholder-zinc-600 focus:outline-none focus:border-zinc-500 transition-colors shadow-inner" 
              required
            >
          </div>
          
          <button 
            :disabled="isLoading" 
            type="submit" 
            class="w-full bg-zinc-100 hover:bg-white text-zinc-950 font-semibold text-xs py-3 px-4 rounded-xl transition-all disabled:opacity-50 mt-2 shadow-lg active:scale-[0.99]"
          >
            {{ isLoading ? 'Подключение к туннелю...' : 'Войти в браузер' }}
          </button>
        </form>
      </div>

      <!-- Правая колонка: Презентация возможностей -->
      <div class="w-full md:w-1/2 p-8 bg-[#0c0c0f] flex flex-col justify-between">
        <div>
          <h3 class="text-xs font-semibold text-zinc-300 uppercase font-mono tracking-wider mb-5 flex items-center">
            <span class="w-1.5 h-1.5 rounded-full bg-emerald-400 mr-2"></span>
            Архитектура безопасности
          </h3>

          <div class="space-y-4">
            <div class="flex items-start space-x-3">
              <div class="w-7 h-7 rounded-xl bg-zinc-800/80 border border-zinc-700/50 flex items-center justify-center shrink-0 text-xs mt-0.5">
                🌐
              </div>
              <div>
                <h4 class="text-xs font-semibold text-zinc-200">100% нативный Chromium</h4>
                <p class="text-[11px] text-zinc-500 leading-relaxed mt-0.5">Google Auth, Cloudflare, WebAuthn и AI-сервисы работают без блокировок и проверок.</p>
              </div>
            </div>

            <div class="flex items-start space-x-3">
              <div class="w-7 h-7 rounded-xl bg-zinc-800/80 border border-zinc-700/50 flex items-center justify-center shrink-0 text-xs mt-0.5">
                🔒
              </div>
              <div>
                <h4 class="text-xs font-semibold text-zinc-200">Wisp Loopback туннель</h4>
                <p class="text-[11px] text-zinc-500 leading-relaxed mt-0.5">Локальный прокси на Rust со случайным паролем сессии и ротацией JWT-токенов.</p>
              </div>
            </div>

            <div class="flex items-start space-x-3">
              <div class="w-7 h-7 rounded-lg bg-zinc-800/80 border border-zinc-700/50 flex items-center justify-center shrink-0 text-xs mt-0.5">
                🛡️
              </div>
              <div>
                <h4 class="text-xs font-semibold text-zinc-200">Remote DNS на VPS</h4>
                <p class="text-[11px] text-zinc-500 leading-relaxed mt-0.5">Шлюз колледжа не видит и не может заблокировать DNS-запросы к доменам.</p>
              </div>
            </div>

            <div class="flex items-start space-x-3">
              <div class="w-7 h-7 rounded-lg bg-zinc-800/80 border border-zinc-700/50 flex items-center justify-center shrink-0 text-xs mt-0.5">
                🧹
              </div>
              <div>
                <h4 class="text-xs font-semibold text-zinc-200">Zero-Footprint</h4>
                <p class="text-[11px] text-zinc-500 leading-relaxed mt-0.5">Полное удаление временных файлов и кэша с компьютера при закрытии окна.</p>
              </div>
            </div>
          </div>
        </div>

        <div class="mt-6 pt-4 border-t border-zinc-800/80 text-[10px] text-zinc-500 font-mono flex items-center justify-between">
          <span>TAURI RUST EDITION</span>
          <span>TELEGRAM OTP</span>
        </div>
      </div>

    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue';
import { useAuthStore } from '../stores/auth.store';

const authMode = ref('otp');
const otpCode = ref('');
const username = ref('');
const password = ref('');
const isLoading = ref(false);
const authStore = useAuthStore();

const handleOtpLogin = async () => {
  const cleanCode = otpCode.value.replace(/\D/g, '');
  if (cleanCode.length !== 6) {
    alert('Пожалуйста, введите 6-значный цифровой код.');
    return;
  }
  isLoading.value = true;
  const success = await authStore.loginWithOtp(cleanCode);
  if (!success) {
    alert('Неверный или истекший код. Запросите новый код в Telegram-боте через /login.');
  }
  isLoading.value = false;
};

const handlePasswordLogin = async () => {
  isLoading.value = true;
  const success = await authStore.login(username.value.trim(), password.value);
  if (!success) {
    alert('Ошибка авторизации. Проверьте логин и пароль от Telegram-бота.');
  }
  isLoading.value = false;
};
</script>