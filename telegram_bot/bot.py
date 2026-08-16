# telegram_bot/bot.py
import os
import random
import string
import requests
from dotenv import load_dotenv
import telebot

load_dotenv()

BOT_TOKEN = os.getenv("BOT_TOKEN")
API_BASE = os.getenv("API_URL", "http://localhost:8080").replace("/api/auth/register", "").rstrip("/")
API_SECRET = os.getenv("API_SECRET")
DOMAIN = os.getenv("DOMAIN", "https://web.chabupelik.su")

bot = telebot.TeleBot(BOT_TOKEN)

def generate_password(length=12):
    characters = string.ascii_letters + string.digits
    return ''.join(random.choice(characters) for _ in range(length))

def get_username(message):
    return message.from_user.username or f"tg_{message.from_user.id}"

@bot.message_handler(commands=['start', 'help'])
def send_welcome(message):
    username = get_username(message)
    password = generate_password()
    
    headers = {
        "x-bot-token": API_SECRET,
        "Content-Type": "application/json"
    }
    payload = {
        "username": username,
        "password": password
    }
    
    try:
        response = requests.post(f"{API_BASE}/api/auth/register", json=payload, headers=headers, timeout=10)
        if response.status_code == 200:
            welcome_text = (
                f"🎉 *Регистрация в Chabupelik Browser успешна!*\n\n"
                f"Твои учетные данные:\n"
                f"👤 *Логин:* `{username}`\n"
                f"🔑 *Пароль:* `{password}`\n\n"
                f"⚡ *Быстрый вход без пароля:* напиши команду /login чтобы получить одноразовый 6-значный код для входа в браузер.\n\n"
                f"🌐 *Веб-версия:* {DOMAIN}"
            )
            bot.send_message(message.chat.id, welcome_text, parse_mode="Markdown")
        elif response.status_code == 500:
            welcome_text = (
                f"ℹ️ *Вы уже зарегистрированы в системе!*\n\n"
                f"👤 *Логин:* `{username}`\n\n"
                f"🔑 *Получить код для входа:* отправьте команду /login для получения одноразового кода авторизации в браузере."
            )
            bot.send_message(message.chat.id, welcome_text, parse_mode="Markdown")
        else:
            bot.send_message(message.chat.id, "❌ Произошла ошибка при регистрации. Попробуйте позже.")
    except Exception as e:
        print("Error during register:", e)
        bot.send_message(message.chat.id, "❌ Не удалось связаться с сервером авторизации.")

@bot.message_handler(commands=['login', 'code', 'otp'])
def send_otp(message):
    username = get_username(message)
    
    headers = {
        "x-bot-token": API_SECRET,
        "Content-Type": "application/json"
    }
    payload = {
        "username": username
    }
    
    try:
        response = requests.post(f"{API_BASE}/api/auth/otp/generate", json=payload, headers=headers, timeout=10)
        if response.status_code == 200:
            data = response.json()
            code = data.get("code")
            expires = data.get("expiresIn", 120)
            
            otp_text = (
                f"🔑 *Твой одноразовый код для входа:*\n\n"
                f"#️⃣ `{code}`\n\n"
                f"⏳ Код действителен *{expires} секунд*.\n"
                f"Введи этот 6-значный код в окне браузера для входа."
            )
            bot.send_message(message.chat.id, otp_text, parse_mode="Markdown")
        elif response.status_code == 404:
            bot.send_message(
                message.chat.id, 
                "❌ Ваш аккаунт не найден. Отправьте /start для автоматической регистрации."
            )
        else:
            bot.send_message(message.chat.id, "❌ Ошибка сервера при генерации кода. Попробуйте позже.")
    except Exception as e:
        print("Error during OTP generate:", e)
        bot.send_message(message.chat.id, "❌ Не удалось связаться с сервером авторизации.")

if __name__ == "__main__":
    print("Telegram bot is running with OTP support...")
    bot.infinity_polling()