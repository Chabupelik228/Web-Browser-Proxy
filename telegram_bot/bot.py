# telegram_bot/bot.py
import os
import asyncio
import aiohttp
import ipaddress
from dotenv import load_dotenv
from aiogram import Bot, Dispatcher, types, BaseMiddleware, F
from aiogram.filters import Command
from aiogram.client.default import DefaultBotProperties
from aiogram.types import InlineKeyboardMarkup, InlineKeyboardButton, CallbackQuery
from aiogram.fsm.context import FSMContext
from aiogram.fsm.state import State, StatesGroup
from aiogram.fsm.storage.memory import MemoryStorage

load_dotenv()

BOT_TOKEN = os.getenv("BOT_TOKEN")
API_BASE = os.getenv("API_URL", "http://localhost:8080").replace("/api/auth/register", "").rstrip("/")
API_SECRET = os.getenv("API_SECRET")
ADMIN_ID = str(os.getenv("ADMIN_ID"))

bot = Bot(token=BOT_TOKEN, default=DefaultBotProperties(parse_mode="Markdown"))
dp = Dispatcher(storage=MemoryStorage())

# Локальный кэш белых списков
cached_ips = []
cached_tg_ids = {}

async def fetch_whitelists():
    global cached_ips, cached_tg_ids
    headers = {"x-bot-token": API_SECRET}
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(f"{API_BASE}/api/admin/whitelists", headers=headers, timeout=10) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    cached_ips = data.get("ips", [])
                    cached_tg_ids = {str(item["id"]): item.get("name") for item in data.get("tg_ids", [])}
                    print("[BOT] Whitelists loaded from backend:", len(cached_ips), "IPs,", len(cached_tg_ids), "TGs")
    except Exception as e:
        print("[BOT] Failed to load whitelists on startup:", e)

class WhitelistMiddleware(BaseMiddleware):
    async def __call__(self, handler, event: types.Message, data: dict):
        if not event or not getattr(event, 'from_user', None):
            return await handler(event, data)
            
        user_id = str(event.from_user.id)
        
        if "all" not in cached_tg_ids:
            if user_id not in cached_tg_ids and user_id != ADMIN_ID:
                print(f"[FIREWALL BOT] Blocked message from unapproved ID: {user_id}")
                try:
                    await event.answer("❌ У вас нет доступа к этому боту.")
                except Exception:
                    pass
                return # Игнорируем
                
        return await handler(event, data)

dp.message.middleware(WhitelistMiddleware())

def get_username(message: types.Message):
    return message.from_user.username or f"tg_{message.from_user.id}"

# ----------------- ADMIN FSM -----------------
class AdminStates(StatesGroup):
    waiting_for_ip = State()
    waiting_for_tg_id = State()
    waiting_for_name = State()
    waiting_for_user_new_name = State()

def get_admin_keyboard():
    markup = InlineKeyboardMarkup(inline_keyboard=[
        [InlineKeyboardButton(text="🧑‍💻 Зарегистрированные юзеры", callback_data="admin_dbusers")],
        [InlineKeyboardButton(text="🌐 Управление IP", callback_data="admin_ip")],
        [InlineKeyboardButton(text="👥 Управление TG ID", callback_data="admin_tg")],
        [InlineKeyboardButton(text="🔄 Обновить кэш с сервера", callback_data="admin_refresh")]
    ])
    return markup

def get_list_keyboard(type_name):
    markup = InlineKeyboardMarkup(inline_keyboard=[
        [InlineKeyboardButton(text="➕ Добавить", callback_data=f"add_{type_name}")],
        [InlineKeyboardButton(text="➖ Удалить", callback_data=f"del_{type_name}")],
        [InlineKeyboardButton(text="Включить/Выключить ALL", callback_data=f"toggle_{type_name}_all")],
        [InlineKeyboardButton(text="🔙 Назад", callback_data="admin_back")]
    ])
    return markup

@dp.message(Command("admin"))
async def admin_panel(message: types.Message, state: FSMContext):
    if str(message.from_user.id) != ADMIN_ID:
        return
    await state.clear()
    sent = await message.answer("🛠 *Панель администратора*\n\nВыберите раздел для управления белыми списками:", reply_markup=get_admin_keyboard())
    asyncio.create_task(delete_messages_later(message, sent, 300))

async def render_admin_list(message: types.Message, type_name: str):
    if type_name == "ip":
        status = "🟢 Фильтр ОТКЛЮЧЕН (ALL)" if "all" in cached_ips else "🔴 Фильтр ВКЛЮЧЕН"
        text = f"🌐 *Управление IP*\n\n{status}\n\n*Текущий список:*\n`" + "`\n`".join(cached_ips) + "`"
        if not cached_ips: text += "Пусто"
        await message.edit_text(text, reply_markup=get_list_keyboard("ip"))
    elif type_name == "tg":
        status = "🟢 Фильтр ОТКЛЮЧЕН (ALL)" if "all" in cached_tg_ids else "🔴 Фильтр ВКЛЮЧЕН"
        text = f"👥 *Управление TG ID*\n\n{status}\n\n*Текущий список:*\n"
        if not cached_tg_ids: 
            text += "Пусто"
        else:
            for tid, tname in cached_tg_ids.items():
                text += f"`{tid}` - {tname or 'Без имени'}\n"
        await message.edit_text(text, reply_markup=get_list_keyboard("tg"))

@dp.callback_query(F.data.startswith("admin_"))
async def admin_callbacks(call: CallbackQuery, state: FSMContext):
    if str(call.from_user.id) != ADMIN_ID:
        return await call.answer("Доступ запрещен", show_alert=True)
        
    action = call.data.split("_")[1]
    
    if action == "back":
        await call.message.edit_text("🛠 *Панель администратора*\n\nВыберите раздел для управления белыми списками:", reply_markup=get_admin_keyboard())
    elif action == "refresh":
        await fetch_whitelists()
        await call.answer("Кэш обновлен!", show_alert=True)
    elif action in ["ip", "tg"]:
        await render_admin_list(call.message, action)
    elif action == "dbusers":
        await render_dbusers_page(call.message, 0)
        await call.answer()

async def api_manage_whitelist(action, type_name, value, name=None):
    headers = {"x-bot-token": API_SECRET, "Content-Type": "application/json"}
    payload = {"type": type_name, "value": value}
    if name:
        payload["name"] = name
        
    try:
        async with aiohttp.ClientSession() as session:
            if action == "add":
                async with session.post(f"{API_BASE}/api/admin/whitelists", json=payload, headers=headers) as resp:
                    return resp.status == 200
            elif action == "del":
                async with session.delete(f"{API_BASE}/api/admin/whitelists", json=payload, headers=headers) as resp:
                    return resp.status == 200
    except Exception as e:
        print("API error:", e)
        return False
    return False

async def api_manage_users(action, user_id=None, name=None):
    headers = {"x-bot-token": API_SECRET}
    try:
        async with aiohttp.ClientSession() as session:
            if action == "list":
                async with session.get(f"{API_BASE}/api/admin/users", headers=headers) as resp:
                    if resp.status == 200:
                        data = await resp.json()
                        return data.get("users", [])
            elif action == "edit":
                payload = {"username": name}
                headers["Content-Type"] = "application/json"
                async with session.put(f"{API_BASE}/api/admin/users/{user_id}", json=payload, headers=headers) as resp:
                    return resp.status == 200
            elif action == "del":
                async with session.delete(f"{API_BASE}/api/admin/users/{user_id}", headers=headers) as resp:
                    status = resp.status
                    text = await resp.text()
                    print(f"DEBUG DELETE user {user_id}: status={status} text={text}")
                    return status == 200
    except Exception as e:
        print("API error users:", e)
        return [] if action == "list" else False
    return [] if action == "list" else False

async def api_fetch_traffic():
    headers = {"x-bot-token": API_SECRET}
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(f"{API_BASE}/api/admin/traffic", headers=headers, timeout=10) as resp:
                if resp.status == 200:
                    data = await resp.json()
                    return {str(u["id"]): u for u in data.get("users", [])}
    except Exception as e:
        print("API error traffic:", e)
    return {}

async def api_send_command(user_id, command):
    headers = {"x-bot-token": API_SECRET, "Content-Type": "application/json"}
    payload = {"user_id": user_id, "command": command}
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(f"{API_BASE}/api/admin/command", json=payload, headers=headers) as resp:
                return resp.status == 200
    except Exception as e:
        print("API error command:", e)
        return False

def format_bytes(b):
    if b < 1024:
        return f"{b} B"
    elif b < 1024 * 1024:
        return f"{b / 1024:.1f} KB"
    elif b < 1024 * 1024 * 1024:
        return f"{b / (1024*1024):.1f} MB"
    else:
        return f"{b / (1024*1024*1024):.2f} GB"

async def render_dbusers_page(message: types.Message, page: int):
    users = await api_manage_users("list")
    per_page = 8
    total_pages = max(1, (len(users) + per_page - 1) // per_page)
    if page >= total_pages: page = total_pages - 1
    if page < 0: page = 0
    
    start_idx = page * per_page
    end_idx = start_idx + per_page
    page_users = users[start_idx:end_idx]
    
    kb = []
    for u in page_users:
        kb.append([InlineKeyboardButton(text=f"👤 {u['username']} (ID: {u['id']})", callback_data=f"u_view:{u['id']}")])
        
    nav_row = []
    if page > 0:
        nav_row.append(InlineKeyboardButton(text="⬅️ Назад", callback_data=f"u_page:{page-1}"))
    if page < total_pages - 1:
        nav_row.append(InlineKeyboardButton(text="Вперед ➡️", callback_data=f"u_page:{page+1}"))
        
    if nav_row: kb.append(nav_row)
    kb.append([InlineKeyboardButton(text="🔙 Назад в меню", callback_data="admin_back")])
    
    text = f"🧑‍💻 *Зарегистрированные юзеры*\n\nВсего в базе: {len(users)}\nСтраница: {page+1}/{total_pages}"
    await message.edit_text(text, reply_markup=InlineKeyboardMarkup(inline_keyboard=kb))



@dp.callback_query(F.data.startswith("u_"))
async def handle_dbusers_callbacks(call: CallbackQuery, state: FSMContext):
    if str(call.from_user.id) != ADMIN_ID: return
    parts = call.data.split(":")
    action = parts[0]
    val = parts[1]
    
    if action == "u_page":
        await render_dbusers_page(call.message, int(val))
    elif action == "u_view":
        users = await api_manage_users("list")
        u = next((x for x in users if str(x['id']) == val), None)
        if not u:
            return await call.answer("Пользователь не найден", show_alert=True)
        
        # Fetch traffic data
        traffic_data = await api_fetch_traffic()
        t = traffic_data.get(val, {})
        
        connected = t.get("connected", False)
        domain = t.get("domain", None)
        session_bytes = t.get("sessionBytes", 0)
        total_bytes = t.get("totalBytes", 0)
        
        status_emoji = "🟢" if connected else "🔴"
        status_text = "Онлайн" if connected else "Офлайн"
        
        text = (
            f"👤 *Профиль пользователя*\n\n"
            f"*ID в базе:* `{u['id']}`\n"
            f"*TG ID:* `{u.get('telegram_id', 'Не привязан')}`\n"
            f"*Имя/Логин:* {u['username']}\n\n"
            f"{status_emoji} *Статус:* {status_text}\n"
        )
        
        if connected and domain:
            text += f"🌐 *Поддомен:* `{domain}`\n"
        
        text += (
            f"📥 *Трафик (сессия):* {format_bytes(session_bytes)}\n"
            f"📊 *Трафик (всего):* {format_bytes(total_bytes)}"
        )
        
        kb = InlineKeyboardMarkup(inline_keyboard=[
            [InlineKeyboardButton(text="🔍 Найти устройство", callback_data=f"u_cmd:find:{u['id']}")],
            [InlineKeyboardButton(text="🚪 Обычный режим", callback_data=f"u_cmd:norm:{u['id']}")],
            [InlineKeyboardButton(text="✒️ Изменить Имя", callback_data=f"u_edit:{u['id']}")],
            [InlineKeyboardButton(text="❌ Удалить", callback_data=f"u_del:{u['id']}")],
            [InlineKeyboardButton(text="🔙 К списку", callback_data="admin_dbusers")]
        ])
        await call.message.edit_text(text, reply_markup=kb)
    elif action == "u_del":
        success = await api_manage_users("del", user_id=val)
        if success:
            await call.answer("Пользователь удален!", show_alert=True)
            await render_dbusers_page(call.message, 0)
        else:
            await call.answer("Ошибка сервера", show_alert=True)
    elif action == "u_cmd":
        cmd = parts[1]
        uid = parts[2]
        success = await api_send_command(uid, cmd)
        if success:
            await call.answer("Команда отправлена!", show_alert=True)
        else:
            await call.answer("Ошибка отправки команды", show_alert=True)
    elif action == "u_edit":
        await state.update_data(edit_user_id=val)
        await state.set_state(AdminStates.waiting_for_user_new_name)
        
        sent = await call.message.answer("✏️ Введите новое имя для этого пользователя:")
        asyncio.create_task(delete_messages_later(sent, delay=300))
        await call.answer()
    
    try: await call.answer()
    except: pass

@dp.message(AdminStates.waiting_for_user_new_name)
async def process_edit_user_name(message: types.Message, state: FSMContext):
    data = await state.get_data()
    uid = data.get("edit_user_id")
    new_name = message.text.strip()
    
    success = await api_manage_users("edit", user_id=uid, name=new_name)
    if success:
        sent = await message.answer(f"✅ Имя пользователя обновлено на '{new_name}'.")
    else:
        sent = await message.answer("❌ Ошибка сервера при обновлении имени.")
    asyncio.create_task(delete_messages_later(message, sent, 300))
    await state.clear()

@dp.callback_query(F.data.startswith("toggle_"))
async def toggle_all_callback(call: CallbackQuery):
    if str(call.from_user.id) != ADMIN_ID:
        return
    type_name = call.data.split("_")[1]
    
    if type_name == "ip":
        if "all" in cached_ips:
            success = await api_manage_whitelist("del", "ip", "all")
            if success:
                cached_ips.remove("all")
                await call.answer("Значение ALL удалено (фильтр ВКЛЮЧЕН)", show_alert=True)
        else:
            success = await api_manage_whitelist("add", "ip", "all")
            if success:
                cached_ips.append("all")
                await call.answer("Значение ALL добавлено (фильтр ОТКЛЮЧЕН)", show_alert=True)
    else:
        if "all" in cached_tg_ids:
            success = await api_manage_whitelist("del", "tg_id", "all")
            if success:
                cached_tg_ids.pop("all", None)
                await call.answer("Значение ALL удалено (фильтр ВКЛЮЧЕН)", show_alert=True)
        else:
            success = await api_manage_whitelist("add", "tg_id", "all", "ALL")
            if success:
                cached_tg_ids["all"] = "ALL"
                await call.answer("Значение ALL добавлено (фильтр ОТКЛЮЧЕН)", show_alert=True)
            
    # Refresh menu
    await render_admin_list(call.message, type_name)

@dp.callback_query(F.data.startswith("add_"))
async def add_callback(call: CallbackQuery, state: FSMContext):
    if str(call.from_user.id) != ADMIN_ID:
        return
    type_name = call.data.split("_")[1]
    
    if type_name == "ip":
        await state.update_data(del_type="")
        await state.set_state(AdminStates.waiting_for_ip)
        sent = await call.message.answer("Введите IP или CIDR для ДОБАВЛЕНИЯ (например, 192.168.1.1 или 10.0.0.0/24):")
        asyncio.create_task(delete_messages_later(sent, delay=300))
    else:
        await state.update_data(del_type="")
        await state.set_state(AdminStates.waiting_for_tg_id)
        sent = await call.message.answer("Введите Telegram ID для ДОБАВЛЕНИЯ:")
        asyncio.create_task(delete_messages_later(sent, delay=300))
    await call.answer()

@dp.callback_query(F.data.startswith("del_"))
async def del_callback(call: CallbackQuery, state: FSMContext):
    if str(call.from_user.id) != ADMIN_ID:
        return
    type_name = call.data.split("_")[1]
    
    if type_name == "ip":
        await state.update_data(del_type="ip")
        await state.set_state(AdminStates.waiting_for_ip)
        sent = await call.message.answer("Введите IP для УДАЛЕНИЯ:")
        asyncio.create_task(delete_messages_later(sent, delay=300))
    else:
        await state.update_data(del_type="tg")
        await state.set_state(AdminStates.waiting_for_tg_id)
        sent = await call.message.answer("Введите TG ID для УДАЛЕНИЯ:")
        asyncio.create_task(delete_messages_later(sent, delay=300))
    await call.answer()

@dp.message(AdminStates.waiting_for_ip)
async def process_ip_input(message: types.Message, state: FSMContext):
    data = await state.get_data()
    action = "del" if data.get("del_type") else "add"
    ip_str = message.text.strip()
    
    try:
        if action == "add" and ip_str != "all":
            if "/" in ip_str: ipaddress.ip_network(ip_str, strict=False)
            else: ipaddress.ip_address(ip_str)
            
        success = await api_manage_whitelist(action, "ip", ip_str)
        if success:
            if action == "add" and ip_str not in cached_ips: cached_ips.append(ip_str)
            elif action == "del" and ip_str in cached_ips: cached_ips.remove(ip_str)
            sent = await message.answer(f"✅ IP {ip_str} {'добавлен' if action == 'add' else 'удален'}.")
        else:
            sent = await message.answer("❌ Ошибка сервера.")
    except ValueError:
        sent = await message.answer("❌ Некорректный формат IP.")
        
    asyncio.create_task(delete_messages_later(message, sent, 300))
    await state.clear()

@dp.message(AdminStates.waiting_for_tg_id)
async def process_tg_input(message: types.Message, state: FSMContext):
    data = await state.get_data()
    action = "del" if data.get("del_type") else "add"
    tg_str = message.text.strip()
    
    if tg_str != "all" and not tg_str.isdigit():
        sent = await message.answer("❌ TG ID должен быть числом.")
        asyncio.create_task(delete_messages_later(message, sent, 300))
        await state.clear()
        return
        
    if action == "del":
        success = await api_manage_whitelist("del", "tg_id", tg_str)
        if success:
            cached_tg_ids.pop(tg_str, None)
            sent = await message.answer(f"✅ TG ID {tg_str} удален.")
        else:
            sent = await message.answer("❌ Ошибка сервера.")
        asyncio.create_task(delete_messages_later(message, sent, 300))
        await state.clear()
    else:
        # add action
        if tg_str == "all":
            success = await api_manage_whitelist("add", "tg_id", "all", "ALL")
            if success:
                cached_tg_ids["all"] = "ALL"
                sent = await message.answer("✅ ALL добавлены.")
                asyncio.create_task(delete_messages_later(message, sent, 300))
            await state.clear()
        else:
            await state.update_data(tg_id=tg_str)
            await state.set_state(AdminStates.waiting_for_name)
            sent = await message.answer("Укажите имя для этого TG ID (например, Иван Иванов):")
            asyncio.create_task(delete_messages_later(message, sent, 300))

@dp.message(AdminStates.waiting_for_name)
async def process_tg_name(message: types.Message, state: FSMContext):
    data = await state.get_data()
    tg_str = data.get("tg_id")
    tg_name = message.text.strip()
    
    success = await api_manage_whitelist("add", "tg_id", tg_str, tg_name)
    if success:
        cached_tg_ids[tg_str] = tg_name
        sent = await message.answer(f"✅ TG ID {tg_str} сохранен под именем '{tg_name}'.")
    else:
        sent = await message.answer("❌ Ошибка сервера.")
        
    asyncio.create_task(delete_messages_later(message, sent, 300))
    await state.clear()

# ----------------- REGULAR USERS -----------------

async def delete_messages_later(msg1: types.Message, msg2: types.Message = None, delay: int = 120):
    await asyncio.sleep(delay)
    for m in (msg1, msg2):
        if m:
            try:
                await m.delete()
            except Exception as e:
                pass

@dp.message(Command("start", "help"))
async def send_welcome(message: types.Message):
    username = get_username(message)
    
    headers = {
        "x-bot-token": API_SECRET,
        "Content-Type": "application/json"
    }
    payload = {
        "username": username,
        "user_id": str(message.from_user.id)
    }
    
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(f"{API_BASE}/api/auth/register", json=payload, headers=headers, timeout=10) as response:
                status = response.status
        
        if status == 200:
            welcome_text = (
                f"🎉 *Регистрация в Chabupelik Browser успешна!*\n\n"
                f"Твои учетные данные отправлены. Бэкенд автоматически определит твоё имя!\n"
                f"⚡ *Быстрый вход:* напиши команду /login чтобы получить одноразовый 6-значный код для входа в браузер."
            )
            sent_message = await message.answer(welcome_text)
            asyncio.create_task(delete_messages_later(message, sent_message))
        elif status == 409 or status == 500:
            welcome_text = (
                f"ℹ️ *Вы уже зарегистрированы в системе!*\n\n"
                f"🔑 *Получить код для входа:* отправьте команду /login для получения одноразового кода авторизации в браузере."
            )
            sent_message = await message.answer(welcome_text)
            asyncio.create_task(delete_messages_later(message, sent_message))
        else:
            sent_message = await message.answer("❌ Произошла ошибка при регистрации. Попробуйте позже.")
            asyncio.create_task(delete_messages_later(message, sent_message))
    except Exception as e:
        print("Error during register:", e)
        sent_message = await message.answer("❌ Не удалось связаться с сервером авторизации.")
        asyncio.create_task(delete_messages_later(message, sent_message))

@dp.message(Command("login", "code", "otp"))
async def send_otp(message: types.Message):
    username = get_username(message)
    
    headers = {
        "x-bot-token": API_SECRET,
        "Content-Type": "application/json"
    }
    payload = {
        "username": username,
        "user_id": str(message.from_user.id)
    }
    
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(f"{API_BASE}/api/auth/otp/generate", json=payload, headers=headers, timeout=10) as response:
                status = response.status
                if status == 200:
                    data = await response.json()
        
        if status == 200:
            code = data.get("code")
            expires = data.get("expiresIn", 120)
            
            otp_text = (
                f"🔑 *Твой одноразовый код для входа:*\n\n"
                f"#️⃣ `{code}`\n\n"
                f"⏳ Код действителен *{expires} секунд*.\n"
                f"Введи этот 6-значный код в окне браузера для входа."
            )
            sent_message = await message.answer(otp_text)
            asyncio.create_task(delete_messages_later(message, sent_message, expires))
        elif status == 404:
            sent_message = await message.answer("❌ Ваш аккаунт не найден. Отправьте /start для автоматической регистрации.")
            asyncio.create_task(delete_messages_later(message, sent_message))
        else:
            sent_message = await message.answer("❌ Ошибка сервера при генерации кода. Попробуйте позже.")
            asyncio.create_task(delete_messages_later(message, sent_message))
    except Exception as e:
        print("Error during OTP generate:", e)
        sent_message = await message.answer("❌ Не удалось связаться с сервером авторизации.")
        asyncio.create_task(delete_messages_later(message, sent_message))

@dp.message()
async def delete_unknown(message: types.Message):
    # Удаляем любое сообщение, кроме команд выше, через 120 секунд
    asyncio.create_task(delete_messages_later(message))

async def main():
    await fetch_whitelists()
    print("Telegram bot (aiogram) is running with OTP support...")
    await dp.start_polling(bot)

if __name__ == "__main__":
    asyncio.run(main())
