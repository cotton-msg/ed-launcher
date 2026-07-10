# Установка зависимостей для Tauri 2

## 1. Установка Rust

Скачайте и установите Rust с официального сайта:
https://www.rust-lang.org/tools/install

Или используйте winget:
```powershell
winget install Rustlang.Rustup
```

После установки перезапустите терминал и проверьте:
```bash
rustc --version
cargo --version
```

## 2. Установка WebView2 (для Windows)

Tauri требует Microsoft Edge WebView2. Обычно уже установлен на Windows 10/11.
Проверить: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

## 3. Установка зависимостей Node.js

```bash
npm install
```

## 4. Установка Tauri CLI

```bash
npm install -D @tauri-apps/cli@next
```

## 5. Инициализация Tauri

```bash
npm run tauri init
```

## 6. Запуск приложения

```bash
npm run tauri dev
```

## 7. Сборка для продакшена

```bash
npm run tauri build
```
