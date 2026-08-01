import os
import random
import string
import requests
from dotenv import load_dotenv
import telebot

load_dotenv()

BOT_TOKEN = os.getenv("BOT_TOKEN")
API_URL = os.getenv("API_URL", "http://localhost:8080/api/auth/register")
API_SECRET = os.getenv("API_SECRET")
DOMAIN = os.getenv("DOMAIN", "https://web.chabupelik.su")

bot = telebot.TeleBot(BOT_TOKEN)

# Функция генерации надежного случайного пароля
def generate_password(length=12):
    characters = string.ascii_letters + string.digits
    return ''.join(random.choice(characters) for _ in range(length))

@bot.message_handler(commands=['start', 'help'])
def send_welcome(message):
    username = message.from_user.username
    if not username:
        # Если у пользователя нет юзернейма в Telegram, используем его ID
        username = f"tg_{message.from_user.id}"
    
    password = generate_password()
    
    # Делаем запрос на наш бэкенд для регистрации
    headers = {
        "x-bot-token": API_SECRET,
        "Content-Type": "application/json"
    }
    payload = {
        "username": username,
        "password": password
    }
    
    try:
        response = requests.post(API_URL, json=payload, headers=headers, timeout=10)
        if response.status_code == 200:
            welcome_text = (
                f"🎉 *Регистрация успешна!*\n\n"
                f"Твои учетные данные для входа в прокси-браузер:\n"
                f"👤 *Логин:* `{username}`\n"
                f"🔑 *Пароль:* `{password}`\n\n"
                f"🌐 *Ссылка:* {DOMAIN}\n\n"
                f"⚠️ *Внимание:* Обязательно входи в приватной вкладке (инкогнито) на общих компьютерах колледжа, чтобы сессия автоматически стиралась после закрытия вкладки!"
            )
            bot.send_message(message.chat.id, welcome_text, parse_mode="Markdown")
        elif response.status_code == 500:
            # Если юзернейм уже занят (UNIQUE CONSTRAINT в БД), значит пользователь уже зарегистрирован
            welcome_text = (
                f"ℹ️ *Вы уже зарегистрированы в системе!*\n\n"
                f"Ваш логин в прокси-браузере: `{username}`\n\n"
                f"Если вы забыли свой пароль, обратитесь к администратору для сброса."
            )
            bot.send_message(message.chat.id, welcome_text, parse_mode="Markdown")
        else:
            bot.send_message(message.chat.id, "❌ Произошла ошибка при регистрации на сервере. Попробуйте позже.")
    except Exception as e:
        print("Error during registration:", e)
        bot.send_message(message.chat.id, "❌ Не удалось связаться с сервером авторизации. Проверьте статус VPS бэкенда.")

if __name__ == "__main__":
    print("Telegram bot is running...")
    bot.infinity_polling()