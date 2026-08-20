<template>
  <div class="flex flex-col h-screen w-screen bg-[#202124] text-white select-none font-sans overflow-hidden">
    
    <!-- Chrome Header -->
    <div ref="headerWrapper" class="flex flex-col w-full shrink-0 z-30">
      
      <!-- 1. Window Controls & Tabs Bar -->
      <div class="flex items-end bg-[#1f2020] h-[40px] pl-0 pr-2 relative" @mousedown="startDragging">
        <!-- Tabs container -->
        <div class="flex items-end h-full pl-3 overflow-x-auto scrollbar-hide flex-1" style="scrollbar-width: none; -ms-overflow-style: none;">
          <!-- Tabs loop -->
          <div v-for="(tab, index) in tabs" :key="tab.id" class="flex items-end h-full shrink">
            <div 
              class="relative flex items-center justify-between pl-3 pr-1.5 py-1 shrink min-w-[64px] w-[232px] max-w-[232px] h-[34px] text-[12px] rounded-t-[10px] group cursor-default" 
              :class="activeTabId === tab.id ? 'bg-[#3c3c3c] text-zinc-200' : 'hover:bg-[#282a2d] text-zinc-400'"
              @mousedown.stop="setActiveTab(tab.id)"
            >
              <!-- Inverted curves (съезды) at the bottom for active tab -->
              <div v-if="activeTabId === tab.id" class="absolute -left-3 bottom-0 w-3 h-3 bg-transparent rounded-br-xl shadow-[6px_6px_0_4px_#3c3c3c] z-20 pointer-events-none"></div>
              <div v-if="activeTabId === tab.id" class="absolute -right-3 bottom-0 w-3 h-3 bg-transparent rounded-bl-xl shadow-[-6px_6px_0_4px_#3c3c3c] z-20 pointer-events-none"></div>
              
              <div class="flex items-center space-x-2 truncate pl-0 pr-1.5 -translate-x-[4px] -translate-y-[3px] z-30 relative">
                <div v-if="tab.isLoading" class="w-4 h-4 border-2 border-zinc-400 border-t-transparent rounded-full animate-spin shrink-0"></div>
                
                <img v-else-if="tab.favicon" :src="tab.favicon" class="w-4 h-4 shrink-0 object-contain" />
                <svg v-else class="w-4 h-4 text-zinc-400 shrink-0" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"/>
                </svg>

                <span class="truncate">{{ tab.title || 'Новая вкладка' }}</span>
              </div>
              <button 
                @mousedown.stop="closeTab(tab.id)"
                class="text-[#e8eaed] hover:bg-[#ffffff]/20 rounded-full w-[20px] h-[20px] flex items-center justify-center transition-colors z-30 relative -translate-y-[3px] shrink-0"
              >
                <svg class="w-[14px] h-[14px]" fill="none" stroke="currentColor" stroke-width="2.35" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12"></path></svg>
              </button>
            </div>
            <!-- Separator between inactive tabs -->
            <div v-if="index < tabs.length - 1 && activeTabId !== tab.id && activeTabId !== tabs[index+1].id" class="h-[20px] w-px bg-[#4a4d51] mb-1.5 mx-0.5 shrink-0"></div>
          </div>
          
          <!-- New tab button -->
          <button @click="addTab" class="ml-[6px] -translate-y-[2px] text-[#e8eaed] hover:bg-[#545454] w-7 h-7 flex items-center justify-center rounded-full transition-colors self-end mb-1 shrink-0" @mousedown.stop>
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
              <path d="M12 5v14M5 12h14"/>
            </svg>
          </button>
        </div>
        
        <!-- Window Controls -->
        <div class="ml-auto flex items-center h-full absolute right-0 top-0 text-[#8e8e8e]">
          <button @mousedown.stop @click="minimizeWindow" class="w-[46px] h-full flex items-center justify-center hover:bg-[#545454] hover:text-white transition-colors">
            <svg class="w-2.5 h-2.5" viewBox="0 0 10 10" fill="currentColor"><rect y="4" width="10" height="2"></rect></svg>
          </button>
          <button @mousedown.stop @click="maximizeWindow" class="w-[46px] h-full flex items-center justify-center hover:bg-[#545454] hover:text-white transition-colors">
            <svg class="w-2.5 h-2.5" viewBox="0 0 10 10" fill="none" stroke="currentColor" stroke-width="1">
              <rect x="1.5" y="3" width="5.5" height="5.5" rx="0.75" />
              <path d="M3.5 3V2a0.75 0.75 0 0 1 0.75-0.75h4a0.75 0.75 0 0 1 0.75 0.75v4a0.75 0.75 0 0 1-0.75 0.75H7" stroke-linejoin="round" />
            </svg>
          </button>
          <button @mousedown.stop @click="closeWindow" class="w-[46px] h-full flex items-center justify-center hover:bg-[#e81123] hover:text-white transition-colors">
            <svg class="w-2.5 h-2.5" viewBox="0 0 10 10" stroke="currentColor" stroke-width="2"><path d="M0 0 L10 10 M10 0 L0 10"></path></svg>
          </button>
        </div>
      </div>

      <!-- 2. Address Bar Row -->
      <div class="flex items-center bg-[#3c3c3c] pt-[6px] pb-[7px] pl-2 pr-1.5 border-b border-[#303236]">
        <div class="flex items-center shrink-0 translate-x-[1px] -translate-y-[1px]">
          <button @click="goBack" class="w-7 h-7 flex items-center justify-center text-[#e8eaed] hover:bg-[#545454] rounded-full transition-colors">
            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.85" stroke-linecap="round" stroke-linejoin="round"><path d="M19 12H5"/><path d="M12 19l-7-7 7-7"/></svg>
          </button>
          <button @click="goForward" class="w-7 h-7 ml-[8px] flex items-center justify-center text-zinc-500 rounded-full transition-colors">
            <svg class="w-[18px] h-[18px]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12h14"/><path d="M12 5l7 7-7 7"/></svg>
          </button>
          <button @click="reloadTab" class="w-7 h-7 ml-[8px] translate-y-[1px] flex items-center justify-center text-[#e8eaed] hover:bg-[#545454] rounded-full transition-colors">
            <svg class="w-[13.5px] h-[13.5px]" viewBox="0 0 24 24" fill="none" stroke="currentColor">
              <path d="M21 3v6h-6" stroke-width="2.5" stroke-linejoin="miter" stroke-linecap="square"/>
              <path d="M20.49 15a9 9 0 1 1-2.12-9.36L21 9" stroke-width="2.5" stroke-linecap="round"/>
            </svg>
          </button>
        </div>

        <form @submit.prevent="navigate" class="flex-1 flex items-center ml-[13px] mr-[13px]">
          <div class="relative w-full flex items-center bg-[#282828] border border-transparent focus-within:border-[#3b82f6] rounded-full h-[34px] pl-[5px] pr-3 transition-colors">
            <!-- Settings/Tune Icon inside the URL bar -->
            <div class="flex items-center justify-center bg-[#3c3c3c] hover:bg-[#4a4d51] rounded-full w-[24px] h-[24px] shrink-0 transition-colors -ml-[1px]">
              <button type="button" class="text-white flex items-center justify-center w-full h-full">
                <!-- Authentic Chrome tune icon: top (circle left, gap, line right), bottom (line left, gap, circle right) -->
                <svg class="w-[13.5px] h-[13.5px]" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-linecap="round">
                  <circle cx="3.8" cy="4.5" r="1.9" stroke-width="1.6" />
                  <line x1="8.5" y1="4.5" x2="13.8" y2="4.5" stroke-width="1.5" />
                  <line x1="2.2" y1="12.2" x2="7.2" y2="12.2" stroke-width="1.5" />
                  <circle cx="12.2" cy="12.2" r="1.9" stroke-width="1.6" />
                </svg>
              </button>
            </div>
            <input 
              v-model="inputUrl" 
              type="text" 
              class="w-full bg-transparent border-none outline-none text-zinc-100 text-[14px] placeholder-zinc-500 leading-[34px] ml-2 pr-7 -translate-y-[0px]"
            />
            <button type="button" class="text-[#e8eaed] hover:bg-[#545454] p-1.5 rounded-full absolute right-[8px] transition-colors">
              <svg class="w-[18.5px] h-[18.5px]" viewBox="0 0 24 24" fill="currentColor">
                <path d="M22 9.24l-7.19-.62L12 2 9.19 8.63 2 9.24l5.46 4.73L5.82 21 12 17.27 18.18 21l-1.63-7.03L22 9.24zM12 15.4l-3.76 2.27 1-4.28-3.32-2.88 4.38-.38L12 6.1l1.71 4.04 4.38.38-3.32 2.88 1 4.28L12 15.4z"/>
              </svg>
            </button>
          </div>
        </form>

        <div class="flex items-center space-x-0.5 shrink-0 relative">
          <!-- Profile -->
          <button class="w-8 h-8 flex items-center justify-center text-[#bdc1c6] hover:bg-[#545454] rounded-full transition-colors -translate-x-[3px] -translate-y-[0px]">
            <svg class="w-[19.5px] h-[19.5px]" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="10.4" r="2.8" />
              <path d="M 5.9 18.7 C 7.2 15.2, 16.8 15.2, 18.1 18.7" />
              <circle cx="12" cy="12" r="9" />
            </svg>
          </button>
          <!-- Menu (Three dots) -->
          <button @click="toggleMenu" class="w-8 h-8 flex items-center justify-center text-[#e8eaed] rounded-full transition-colors" :class="showMenu ? 'bg-[#545454]' : 'hover:bg-[#545454]'">
            <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24"><path d="M12 8c1.1 0 2-.9 2-2s-.9-2-2-2-2 .9-2 2 .9 2 2 2zm0 2c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2zm0 6c-1.1 0-2 .9-2 2s.9 2 2 2 2-.9 2-2-.9-2-2-2z"/></svg>
          </button>

          <!-- Chrome Settings Menu Dropdown -->
          <div v-if="showMenu" class="absolute right-0 top-10 mt-1 w-[320px] bg-[#282a2d] border border-[#3b3e41] rounded-lg shadow-2xl py-2 z-50 text-[13px] text-zinc-300 font-sans tracking-wide">
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex justify-between items-center">
              <div class="flex items-center">
                <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M21 3H3c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h18c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H3V5h10v4h8v10z"/></svg>
                <span>Новая вкладка</span>
              </div>
              <span class="text-zinc-500">Ctrl+T</span>
            </div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex justify-between items-center">
              <div class="flex items-center">
                <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M19 3H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14z"/></svg>
                <span>Новое окно</span>
              </div>
              <span class="text-zinc-500">Ctrl+N</span>
            </div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex justify-between items-center">
              <div class="flex items-center">
                <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M12 4.5C7 4.5 2.73 7.61 1 12c1.73 4.39 6 7.5 11 7.5s9.27-3.11 11-7.5c-1.73-4.39-6-7.5-11-7.5zm0 12.5c-2.76 0-5-2.24-5-5s2.24-5 5-5 5 2.24 5 5-2.24 5-5 5zm-3-5a3 3 0 1 0 6 0 3 3 0 0 0-6 0z"/></svg>
                <span>Новое окно в режиме инкогнито</span>
              </div>
              <span class="text-zinc-500">Ctrl+Shift+N</span>
            </div>
            <div class="h-[1px] bg-[#3b3e41] my-1 mx-4"></div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex justify-between items-center bg-[#3b3e41]/30">
              <div class="flex items-center space-x-3">
                <div class="w-6 h-6 rounded-full bg-zinc-600 flex items-center justify-center">
                  <svg class="w-4 h-4 text-zinc-300" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M12 12c2.21 0 4-1.79 4-4s-1.79-4-4-4-4 1.79-4 4 1.79 4 4 4zm0 2c-2.67 0-8 1.34-8 4v2h16v-2c0-2.66-5.33-4-8-4z"/>
                  </svg>
                </div>
                <span>Пользователь 1</span>
              </div>
              <span class="text-blue-400 text-[11px] border border-blue-400 rounded px-2 py-0.5">Вход не выполнен</span>
            </div>
            <div class="h-[1px] bg-[#3b3e41] my-1 mx-4"></div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex items-center">
              <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M12.65 10C11.83 7.67 9.61 6 7 6c-3.31 0-6 2.69-6 6s2.69 6 6 6c2.61 0 4.83-1.67 5.65-4H17v4h4v-4h2v-4H12.65zM7 14c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2z"/></svg>
              <span>Пароли и автозаполнение</span>
            </div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex items-center">
              <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M13 3c-4.97 0-9 4.03-9 9H1l3.89 3.89.07.14L9 12H6c0-3.87 3.13-7 7-7s7 3.13 7 7-3.13 7-7 7c-1.93 0-3.68-.79-4.94-2.06l-1.42 1.42C8.27 19.99 10.51 21 13 21c4.97 0 9-4.03 9-9s-4.03-9-9-9zm-1 5v5l4.28 2.54.72-1.21-3.5-2.08V8H12z"/></svg>
              <span>История</span>
            </div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex justify-between items-center">
              <div class="flex items-center">
                <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M19 9h-4V3H9v6H5l7 7 7-7zM5 18v2h14v-2H5z"/></svg>
                <span>Загрузки</span>
              </div>
              <span class="text-zinc-500">Ctrl+J</span>
            </div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex items-center">
              <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M19 18l2 1V3c0-1.1-.9-2-2-2H8.99C7.89 1 7 1.9 7 3h10c1.1 0 2 .9 2 2v13zM15 5H5c-1.1 0-2 .9-2 2v16l7-3 7 3V7c0-1.1-.9-2-2-2z"/></svg>
              <span>Закладки и списки</span>
            </div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex items-center">
              <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M20.5 11H19V7c0-1.1-.9-2-2-2h-4V3.5C13 2.12 11.88 1 10.5 1S8 2.12 8 3.5V5H4c-1.1 0-1.99.9-1.99 2v3.8H3.5c1.49 0 2.7 1.21 2.7 2.7s-1.21 2.7-2.7 2.7H2V20c0 1.1.9 2 2 2h3.8v-1.5c0-1.49 1.21-2.7 2.7-2.7 1.49 0 2.7 1.21 2.7 2.7V22H17c1.1 0 2-.9 2-2v-4h1.5c1.38 0 2.5-1.12 2.5-2.5s-1.12-2.5-2.5-2.5z"/></svg>
              <span>Расширения</span>
            </div>
            <div class="h-[1px] bg-[#3b3e41] my-1 mx-4"></div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex justify-between items-center">
              <div class="flex items-center">
                <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M16 9v10H8V9h8m-1.5-6h-5l-1 1H5v2h14V4h-3.5l-1-1zM18 7H6v12c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7z"/></svg>
                <span>Удалить данные о работе в браузере...</span>
              </div>
              <span class="text-zinc-500">Ctrl+Shift+Del</span>
            </div>
            <div class="h-[1px] bg-[#3b3e41] my-1 mx-4"></div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex justify-between items-center">
              <div class="flex items-center">
                <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/></svg>
                <span>Масштаб</span>
              </div>
              <div class="flex items-center space-x-2 border border-zinc-600 rounded bg-[#202124] overflow-hidden">
                <button class="px-2 hover:bg-[#3b3e41] pb-0.5">-</button>
                <span class="text-[12px]">100%</span>
                <button class="px-2 hover:bg-[#3b3e41] pb-0.5">+</button>
              </div>
            </div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex justify-between items-center">
              <div class="flex items-center">
                <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M19 8H5c-1.66 0-3 1.34-3 3v6h4v4h12v-4h4v-6c0-1.66-1.34-3-3-3zm-3 11H8v-5h8v5zm3-7c-.55 0-1-.45-1-1s.45-1 1-1 1 .45 1 1-.45 1-1 1zm-1-9H6v4h12V3z"/></svg>
                <span>Печать...</span>
              </div>
              <span class="text-zinc-500">Ctrl+P</span>
            </div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex items-center">
              <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M19.14 12.94c.04-.3.06-.61.06-.94 0-.32-.02-.64-.07-.94l2.03-1.58c.18-.14.23-.41.12-.61l-1.92-3.32c-.12-.22-.37-.29-.59-.22l-2.39.96c-.5-.38-1.03-.7-1.62-.94l-.36-2.54c-.04-.24-.24-.41-.48-.41h-3.84c-.24 0-.43.17-.47.41l-.36 2.54c-.59.24-1.13.57-1.62.94l-2.39-.96c-.22-.08-.47 0-.59.22L2.74 8.87c-.12.21-.08.47.12.61l2.03 1.58c-.05.3-.09.63-.09.94s.02.64.07.94l-2.03 1.58c-.18.14-.23.41-.12.61l1.92 3.32c.12.22.37.29.59.22l2.39-.96c.5.38 1.03.7 1.62.94l.36 2.54c.05.24.24.41.48.41h3.84c.24 0 .44-.17.47-.41l.36-2.54c.59-.24 1.13-.56 1.62-.94l2.39.96c.22.08.47 0 .59-.22l1.92-3.32c.12-.22.07-.47-.12-.61l-2.01-1.58zM12 15.6c-1.98 0-3.6-1.62-3.6-3.6s1.62-3.6 3.6-3.6 3.6 1.62 3.6 3.6-1.62 3.6-3.6 3.6z"/></svg>
              <span>Настройки</span>
            </div>
            <div class="px-4 py-2 hover:bg-[#3b3e41] cursor-default flex items-center">
              <svg class="w-4 h-4 mr-3 text-zinc-400" viewBox="0 0 24 24" fill="currentColor"><path d="M10.09 15.59L11.5 17l5-5-5-5-1.41 1.41L12.67 11H3v2h9.67l-2.58 2.59zM19 3H5c-1.11 0-2 .9-2 2v4h2V5h14v14H5v-4H3v4c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2z"/></svg>
              <span>Выход</span>
            </div>
          </div>
        </div>
      </div>
    </div>
    
    <!-- Area for the stealth Webview -->
    <div class="flex-1 bg-white relative">
      <div v-if="loadingAuth" class="absolute inset-0 bg-[#202124] flex flex-col items-center justify-center z-40">
        <div class="w-8 h-8 border-4 border-blue-500 border-t-transparent rounded-full animate-spin"></div>
        <div class="mt-4 text-zinc-400 font-mono text-xs">Authenticating...</div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useAuthStore } from '../stores/auth.store';

const headerWrapper = ref(null);
let resizeObserver = null;
const showMenu = ref(false);
const inputUrl = ref('google.com');
const loadingAuth = ref(false);

const tabs = ref([{ id: 'fake_tab_1', title: 'Google', url: 'https://google.com', isLoading: true, favicon: 'https://www.google.com/favicon.ico' }]);
const activeTabId = ref('fake_tab_1');

const formatDisplayUrl = (url) => {
  if (!url) return '';
  let displayUrl = url;
  if (displayUrl.startsWith('https://www.')) displayUrl = displayUrl.replace('https://www.', '');
  else if (displayUrl.startsWith('https://')) displayUrl = displayUrl.replace('https://', '');
  else if (displayUrl.startsWith('http://www.')) displayUrl = displayUrl.replace('http://www.', '');
  else if (displayUrl.startsWith('http://')) displayUrl = displayUrl.replace('http://', '');
  if (displayUrl.endsWith('/')) displayUrl = displayUrl.slice(0, -1);
  return displayUrl;
};

const addTab = () => {
  const newId = `fake_tab_${Date.now()}`;
  const newTab = { id: newId, title: 'Новая вкладка', url: 'https://google.com', isLoading: true };
  tabs.value.push(newTab);
  activeTabId.value = newTab.id;
};

const closeTab = async (id) => {
  if (tabs.value.length === 1) return; // don't close last tab
  const index = tabs.value.findIndex(t => t.id === id);
  if (index !== -1) {
    try { await invoke('native_close_tab', { tabId: id }); } catch (_) {}
    tabs.value.splice(index, 1);
    
    if (activeTabId.value === id) {
      const newActive = tabs.value[Math.max(0, index - 1)];
      activeTabId.value = newActive.id;
    }
  }
};

const setActiveTab = (id) => {
  activeTabId.value = id;
};

const minimizeWindow = () => invoke('native_minimize_window').catch(() => {});
const maximizeWindow = () => invoke('native_maximize_window').catch(() => {});
const closeWindow = () => invoke('native_close_window').catch(() => {});
const startDragging = () => invoke('native_start_dragging').catch(() => {});

const authStore = useAuthStore();

const toggleMenu = () => {
  showMenu.value = !showMenu.value;
};

const goBack = () => invoke('native_go_back', { tabId: activeTabId.value }).catch(() => {});
const goForward = () => invoke('native_go_forward', { tabId: activeTabId.value }).catch(() => {});
const reloadTab = () => invoke('native_reload', { tabId: activeTabId.value }).catch(() => {});

watch(() => activeTabId.value, async (newId) => {
  const activeTab = tabs.value.find(t => t.id === newId);
  if (activeTab) {
    inputUrl.value = formatDisplayUrl(activeTab.url || '');
    try {
      await invoke('native_switch_tab', { tabId: newId });
    } catch (e) {
      if (e === 'NOT_FOUND' && activeTab.url) {
        activeTab.isLoading = true;
        invoke('native_navigate_tab', { tabId: newId, url: activeTab.url }).catch(() => {
          activeTab.isLoading = false;
        });
      }
    }
  }
}, { immediate: true });

const navigate = async () => {
  let url = inputUrl.value.trim();
  
  if (url.startsWith('372://')) {
    const code = url.replace('372://', '');
    loadingAuth.value = true;
    const success = await authStore.loginWithOtp(code);
    if (success) return; 
    loadingAuth.value = false;
  }

  let searchUrl = url;
  if (!url.startsWith('http://') && !url.startsWith('https://') && !url.startsWith('data:')) {
    if (url.includes('.') && !url.includes(' ')) {
      searchUrl = `https://${url}`;
    } else {
      searchUrl = `https://www.google.com/search?q=${encodeURIComponent(url)}`;
    }
  } else if (url.startsWith('http://') || url.startsWith('https://')) {
    searchUrl = url;
  }
  
  const currentTab = tabs.value.find(t => t.id === activeTabId.value);
  if (currentTab) {
    currentTab.url = searchUrl;
    currentTab.isLoading = true;
  }
  
  inputUrl.value = formatDisplayUrl(searchUrl);
  invoke('native_navigate_tab', { tabId: activeTabId.value, url: searchUrl }).catch(() => {});
};

let unlistenListeners = [];

onMounted(async () => {
  resizeObserver = new ResizeObserver((entries) => {
    for (let entry of entries) {
      if (entry.target === headerWrapper.value) {
        const height = entry.contentRect.height;
        invoke('native_update_top_bar_height', { height }).catch(() => {});
      }
    }
  });
  if (headerWrapper.value) {
    resizeObserver.observe(headerWrapper.value);
  }

  unlistenListeners.push(await listen('webview-loading', (event) => {
    const { tabId } = event.payload || {};
    const tab = tabs.value.find(t => t.id === tabId);
    if (tab) tab.isLoading = true;
  }));

  unlistenListeners.push(await listen('webview-loaded', (event) => {
    const p = event.payload || {};
    const tId = p.tabId || p.tab_id;
    const tab = tabs.value.find(t => t.id === tId);
    if (tab) {
      tab.isLoading = false;
      if (p.url && !p.url.startsWith('data:')) {
        tab.url = p.url;
        if (activeTabId.value === tId) {
          inputUrl.value = formatDisplayUrl(p.url);
        }
      }
    }
  }));

  unlistenListeners.push(await listen('webview-title-changed', (event) => {
    const { tabId, tab_id, title } = event.payload || {};
    const tId = tabId || tab_id;
    const tab = tabs.value.find(t => t.id === tId);
    if (tab && title) tab.title = title.trim();
  }));

  unlistenListeners.push(await listen('webview-favicon-changed', (event) => {
    const { tabId, tab_id, favicon } = event.payload || {};
    const tId = tabId || tab_id;
    const tab = tabs.value.find(t => t.id === tId);
    if (tab && favicon) tab.favicon = favicon;
  }));

  unlistenListeners.push(await listen('webview-error', (event) => {
    const err = event.payload;
    alert(`[SYSTEM ERROR] Ошибка WebView2:\n${err}`);
    const currentTab = tabs.value.find(t => t.id === activeTabId.value);
    if (currentTab) currentTab.isLoading = false;
  }));
});

onUnmounted(() => {
  if (resizeObserver && headerWrapper.value) resizeObserver.unobserve(headerWrapper.value);
  unlistenListeners.forEach(u => u());
  tabs.value.forEach(t => invoke('native_close_tab', { tabId: t.id }).catch(() => {}));
});
</script>


