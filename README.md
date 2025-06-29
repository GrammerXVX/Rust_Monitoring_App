# 🖥️ Rust Monitoring App

**Кроссплатформенное приложение на Tauri для мониторинга логов.**  
Поддерживает быстрый просмотр, параллельное чтение файлов, копирование, сортировку, а также отслеживание состояния железа!

---

## 📦 Структура проекта

<details>
<summary>src-tauri/</summary>

- **commands/**
  - logs.rs
  - mod.rs
  - system.rs
- **models/**
  - log_entry.rs
  - mod.rs
  - system_info.rs
- **monitoring/**
  - file_monitor.rs
  - mod.rs
- **state/**
  - logs.rs
  - mod.rs
  - system.rs
- **utils/**
  - encoding.rs
  - hashing.rs
  - log_parser.rs
  - mod.rs
- lib.rs
- main.rs

</details>


## 🚀 Как запустить
1. **Установите Rust:**
[Установка Rust оффициальный сайт](https://forge.rust-lang.org/infra/other-installation-methods.html)

3. **Установите зависимости:** 
`npm install`

4. **Запуск в режиме разработки:** 
`npx tauri dev`
3. **Готово!**  
Интерфейс откроется автоматически.

---

## 📝 Описание интерфейса и функций

### 🟢 Основные кнопки

- **Start / Stop**  
▶️ / ⏹️ Запускает или останавливает мониторинг выбранного лог-файла в реальном времени.

- **Clear Log**  
🗑️ Очищает отображаемые логи на экране (не удаляет файл).

- **Select File**  
📂 Открывает диалог выбора лог-файла для мониторинга и анализа.

- **Copy Logs**  
📋 Копирует все текущие логи из таблицы в буфер обмена в удобном текстовом формате.

---

### 🟣 Табы в приложении

- **Real-time Logs**  
⚡ Мгновенный просмотр логов в реальном времени, с подсветкой по уровню (ERROR, INFO, DEBUG и т.д.).  
**Рядом с количеством строк динамически отображается счётчик логов по типам (например: ERROR: 123, WARNING: 7 и т.д.)**  
![1](https://github.com/user-attachments/assets/e330073c-0e4f-468e-8323-91639e3b4e90)

- **Sorted Logs**  
📊 Просмотр логов в структурированном и отсортированном виде (например, по времени, по уровню или тексту).  
![Monitoring 2025-06-28 21-29-41](https://github.com/user-attachments/assets/6dab1acf-8178-41dd-9f88-8b9d3656631f)


- **System Monitor**  
🖥️ Мониторинг состояния системы: загрузка CPU, использование памяти, информация о процессах, температуре, видеокарте и т.д.  
![Monitoring 2025-06-28 21-35-07](https://github.com/user-attachments/assets/c4e3a4ee-50bf-42e0-8b9a-643f1f7cac82)


---

## ⚙️ Возможности

- Загрузка и парсинг лог-файлов (до 100 тысяч строк)
- Отмена загрузки в любой момент (через кнопку Cancel)
- Автоматическая подсветка и подсчёт ошибок/предупреждений/инфо/дебагов в шапке
- Архитектура на Rust — легко дорабатывать и поддерживать
- Кроссплатформенная сборка (Windows, Linux)

---

## 🛠️ Для разработчиков

**Реализация модулей:**
- Все команды для Tauri выносятся в папку `commands/`
- Общие структуры и типы — в `models/`
- Логика мониторинга и работы с файлами — в `monitoring/`
- Глобальные состояния (`Arc<Mutex<T>>`) — в `state/`
- Мелкие утилиты, парсеры, кодировки — в `utils/`
- Все зависимости импортируются через `mod.rs` и используются в `main.rs`

---

## 📦 Установить приложение

## 🖥️ Скачать установщик (Windows)

- [⬇️ Rust Monitoring App – Windows Setup (EXE)](https://github.com/GrammerXVX/Rust_Monitoring_App/releases/download/v0.6.2/setup.exe)
- [⬇️ Rust Monitoring App – Windows Installer (MSI)](https://github.com/GrammerXVX/Rust_Monitoring_App/releases/download/v0.6.2/setup.msi)

> Выберите нужный формат для установки на Windows.  
> После загрузки просто запустите файл и следуйте инструкции.

## 🖥️ Скачать установщик (Linux)
> Установщик временно отсутсвует, требуется произвести сборку проекта из системы Linux командой `npx tauri build`.  

---

## 📚 Лицензия

MIT License

Copyright (c) 2025 GrammerXVX

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

---

**Разработка: [GrammerXVX](https://github.com/GrammerXVX)**  
