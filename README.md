# Grand Eden Launcher

Кастомный лаунчер для Minecraft-сервера Grand Eden, построенный на Tauri 2 и Svelte.

![Version](https://img.shields.io/badge/version-1.0.0-blue)
![Minecraft](https://img.shields.io/badge/minecraft-1.20.1-green)
![Forge](https://img.shields.io/badge/forge-47.2.0-orange)

## ✨ Особенности

- 🚀 **Автоматическая загрузка** — Java и игровые файлы скачиваются с GitHub
- 📦 **Все включено** — моды, конфиги и ресурсы в одном пакете
- 🎨 **Современный UI** — красивый интерфейс с частицами и анимациями
- ⚙️ **Настройки** — RAM, разрешение, fullscreen и другие параметры
- 🌐 **Статус сервера** — онлайн, количество игроков, пинг

## 🛠 Технологии

**Frontend:**
- Svelte 4 + TypeScript
- Vite
- Кастомная система частиц

**Backend:**
- Tauri 2 (Rust)
- Плагины: dialog, fs, http, shell, process

## 📦 Структура репозитория

```
ed-launcher/
├── java/              # Java 17 для запуска игры
├── version/           # Игровые файлы
│   ├── mods/         # Моды для Minecraft 1.20.1
│   └── config/       # Конфигурация модов
├── src/              # Frontend (Svelte)
└── src-tauri/        # Backend (Rust)
```

## 🚀 Как использовать

### Для игроков

1. Скачайте последнюю версию лаунчера из [Releases](../../releases)
2. Запустите `grand-eden-launcher.exe`
3. Введите никнейм
4. Нажмите "Play" — лаунчер автоматически скачает всё необходимое
5. Играйте!

### Для разработчиков

**Требования:**
- [Node.js 18+](https://nodejs.org/)
- [Rust](https://www.rust-lang.org/tools/install)
- [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) (обычно уже установлен на Windows 10/11)

**Установка:**

```bash
# Клонировать репозиторий
git clone https://github.com/supminer/ed-launcher.git
cd ed-launcher

# Установить зависимости
npm install

# Запустить в режиме разработки
npm run tauri:dev

# Собрать для продакшена
npm run tauri:build
```

## 📝 Конфигурация

Настройки GitHub репозитория находятся в `src-tauri/src/lib.rs`:

```rust
const GITHUB_REPO_OWNER: &str = "supminer";
const GITHUB_REPO_NAME: &str = "ed-launcher";
const GITHUB_BRANCH: &str = "main";
```

## 🎮 Моды в сборке

<details>
<summary>Список модов (70+)</summary>

- Alex's Mobs
- Better Combat
- Combat Roll
- Create + дополнения
- Farmer's Delight
- Fantasy Armor & Weapons
- Emotecraft
- Voice Chat
- Embeddium + Oculus (оптимизация)
- И многие другие...

</details>

## 🔧 Как обновить игровые файлы

1. Замените файлы в папках `java/` и `version/`
2. Закоммитьте изменения:
```bash
git add java/ version/
git commit -m "Update game files"
git push origin main
```
3. Лаунчер автоматически скачает обновления при следующем запуске

## 📄 Лицензия

Этот проект создан для сервера Grand Eden.

## 🙏 Благодарности

- [Tauri](https://tauri.app/) — фреймворк для десктопных приложений
- [Svelte](https://svelte.dev/) — реактивный UI-фреймворк
- Все создатели модов в сборке
