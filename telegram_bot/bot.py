# telegram_bot/bot.py
import os
import asyncio
import aiohttp
from dotenv import load_dotenv
from aiogram import Bot, Dispatcher, types
from aiogram.filters import Command
from aiogram.client.default import DefaultBotProperties

load_dotenv()

BOT_TOKEN = os.getenv("BOT_TOKEN")
API_BASE = os.getenv("API_URL", "http://localhost:8080").replace("/api/auth/register", "").rstrip("/")
API_SECRET = os.getenv("API_SECRET")
DOMAIN = os.getenv("DOMAIN")

bot = Bot(token=BOT_TOKEN, default=DefaultBotProperties(parse_mode="Markdown"))
dp = Dispatcher()

def get_username(message: types.Message):
    return message.from_user.username or f"tg_{message.from_user.id}"

@dp.message(Command("start", "help"))
async def send_welcome(message: types.Message):
    username = get_username(message)
    
    headers = {
        "x-bot-token": API_SECRET,
        "Content-Type": "application/json"
    }
    payload = {
        "username": username
    }
    
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(f"{API_BASE}/api/auth/register", json=payload, headers=headers, timeout=10) as response:
                status = response.status
        
        if status == 200:
            welcome_text = (
                f"🎉 *Регистрация в Chabupelik Browser успешна!*\n\n"
                f"Твои учетные данные:\n"
                f"👤 *Логин:* `{username}`\n\n"
                f"⚡ *Быстрый вход:* напиши команду /login чтобы получить одноразовый 6-значный код для входа в браузер.\n\n"
                f"🌐 *Веб-версия:* {DOMAIN}"
            )
            await message.answer(welcome_text)
        elif status == 500:
            welcome_text = (
                f"ℹ️ *Вы уже зарегистрированы в системе!*\n\n"
                f"👤 *Логин:* `{username}`\n\n"
                f"🔑 *Получить код для входа:* отправьте команду /login для получения одноразового кода авторизации в браузере."
            )
            await message.answer(welcome_text)
        else:
            await message.answer("❌ Произошла ошибка при регистрации. Попробуйте позже.")
    except Exception as e:
        print("Error during register:", e)
        await message.answer("❌ Не удалось связаться с сервером авторизации.")

@dp.message(Command("login", "code", "otp"))
async def send_otp(message: types.Message):
    username = get_username(message)
    
    headers = {
        "x-bot-token": API_SECRET,
        "Content-Type": "application/json"
    }
    payload = {
        "username": username
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
            
            async def delete_later(msg: types.Message, delay: int):
                await asyncio.sleep(delay)
                try:
                    await msg.delete()
                except Exception as e:
                    print(f"Failed to delete message: {e}")
            
            asyncio.create_task(delete_later(sent_message, expires))
        elif status == 404:
            await message.answer("❌ Ваш аккаунт не найден. Отправьте /start для автоматической регистрации.")
        else:
            await message.answer("❌ Ошибка сервера при генерации кода. Попробуйте позже.")
    except Exception as e:
        print("Error during OTP generate:", e)
        await message.answer("❌ Не удалось связаться с сервером авторизации.")

async def main():
    print("Telegram bot (aiogram) is running with OTP support...")
    await dp.start_polling(bot)

if __name__ == "__main__":
    asyncio.run(main())