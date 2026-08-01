<!-- frontend/src/views/LoginView.vue -->
<template>
  <div class="flex items-center justify-center h-full bg-[#0f0f11] text-zinc-200 font-sans select-none">
    <div class="bg-[#141417] p-8 rounded-xl shadow-2xl w-96 border border-zinc-800/80">
      
      <!-- Заголовок -->
      <div class="flex flex-col items-center mb-8">
        <div class="flex items-center space-x-2">
          <span class="text-2xl font-semibold tracking-tight text-zinc-100">Chabupelik</span>
          <span class="text-2xl font-light text-zinc-500">Browser</span>
        </div>
        <p class="text-[11px] text-zinc-600 mt-1 font-mono uppercase tracking-widest">
          Secured Access Gateway
        </p>
      </div>
      
      <form @submit.prevent="handleLogin" class="space-y-4">
        <div>
          <label class="block text-xs font-medium text-zinc-400 mb-1">Имя пользователя (от бота)</label>
          <input 
            v-model="username" 
            type="text" 
            placeholder="Username"
            class="w-full bg-[#1c1c20] border border-zinc-800 rounded-lg p-2.5 text-xs text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600 transition-colors" 
            required
          >
        </div>
        
        <div>
          <label class="block text-xs font-medium text-zinc-400 mb-1">Пароль</label>
          <input 
            v-model="password" 
            type="password" 
            placeholder="••••••••"
            class="w-full bg-[#1c1c20] border border-zinc-800 rounded-lg p-2.5 text-xs text-zinc-200 placeholder-zinc-600 focus:outline-none focus:border-zinc-600 transition-colors" 
            required
          >
        </div>
        
        <button 
          :disabled="isLoading" 
          type="submit" 
          class="w-full bg-zinc-800 hover:bg-zinc-700 text-zinc-100 font-medium text-xs py-2.5 px-4 rounded-lg transition-colors border border-zinc-700/60 disabled:opacity-50 mt-2 shadow-sm"
        >
          {{ isLoading ? 'Дешифровка сессии...' : 'Войти' }}
        </button>
      </form>

    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue';
import { useAuthStore } from '../stores/auth.store';

const username = ref('');
const password = ref('');
const isLoading = ref(false);
const authStore = useAuthStore();

const handleLogin = async () => {
  isLoading.value = true;
  const success = await authStore.login(username.value, password.value);
  if (!success) alert('Ошибка авторизации. Проверьте данные от бота.');
  isLoading.value = false;
};
</script>