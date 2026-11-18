# Cardly

> **Cardly** — точный, пиксель-в-пиксель веб-клон приложения Anki Desktop, работающий прямо в браузере, оптимизированный для настольных компьютеров и смартфонов (PWA) и разворачиваемый одной командой через Docker Compose.

Проект создан с соблюдением лицензии GNU AGPLv3. Вся логика планировщика (включая алгоритм FSRS), шаблонизатор карточек, синтаксис поиска и структуры SQLite переиспользуются напрямую из официального ядра Anki (`rslib`, `ts/`, `proto/`) без форка и без повторной реализации.

---

## 📸 Галерея интерфейса (Screenshots)

Все экраны Cardly верифицированы на точное соответствие оригинальному Anki Desktop как в светлой, так и в темной теме, а также на мобильных экранах смартфонов:

### Десктопный режим (Desktop)
| Экран | Светлая тема | Темная тема |
| :--- | :--- | :--- |
| **Список колод (Deck Browser)** | [01-deck-list-light.png](./docs/screenshots/01-deck-list-light.png) | [11-deck-list-dark.png](./docs/screenshots/11-deck-list-dark.png) |
| **Обзор колоды (Deck Overview)** | [02-deck-overview-light.png](./docs/screenshots/02-deck-overview-light.png) | — |
| **Ревьюер: Вопрос (Question)** | [03-reviewer-question-light.png](./docs/screenshots/03-reviewer-question-light.png) | — |
| **Ревьюер: Ответ (Answer + Buttons)** | [04-reviewer-answer-light.png](./docs/screenshots/04-reviewer-answer-light.png) | [13-reviewer-dark.png](./docs/screenshots/13-reviewer-dark.png) |
| **Добавление карточек (Add Cards)** | [05-add-cards-light.png](./docs/screenshots/05-add-cards-light.png) | — |
| **Браузер карточек (Browser 3-pane)** | [06-browser-light.png](./docs/screenshots/06-browser-light.png) | [12-browser-dark.png](./docs/screenshots/12-browser-dark.png) |
| **Статистика (Statistics / Graphs)** | [07-stats-light.png](./docs/screenshots/07-stats-light.png) | — |
| **Опции колоды и FSRS (Deck Options)**| [08-deck-options-fsrs-light.png](./docs/screenshots/08-deck-options-fsrs-light.png) | — |
| **Редактор шаблонов (Card Templates)**| [09-card-templates-light.png](./docs/screenshots/09-card-templates-light.png) | — |
| **Настройки (Preferences)** | [10-preferences-light.png](./docs/screenshots/10-preferences-light.png) | — |

### Мобильный режим и PWA (Mobile Smartphone 390×844)
| Экран | Ссылка на скриншот |
| :--- | :--- |
| **Мобильный список колод** | [14-mobile-deck-list-light.png](./docs/screenshots/14-mobile-deck-list-light.png) |
| **Мобильный обзор колоды** | [15-mobile-deck-overview-light.png](./docs/screenshots/15-mobile-deck-overview-light.png) |
| **Мобильный ревьюер: Вопрос** | [16-mobile-reviewer-question-light.png](./docs/screenshots/16-mobile-reviewer-question-light.png) |
| **Мобильный ревьюер: Ответ (крупные тач-кнопки 44px)** | [17-mobile-reviewer-answer-light.png](./docs/screenshots/17-mobile-reviewer-answer-light.png) |
| **Мобильный ревьюер: Темная тема** | [18-mobile-reviewer-dark.png](./docs/screenshots/18-mobile-reviewer-dark.png) |
| **Мобильный браузер карточек (Sidebar Drawer)** | [19-mobile-browser-light.png](./docs/screenshots/19-mobile-browser-light.png) |

---

## ✨ Ключевые возможности

1. **100% честное ядро Anki (`rslib`)**:
   - Планировщик Anki 23+ и поддержка FSRS v4/v5 (с оптимизатором весов и симулятором).
   - Поддержка Cloze (включая вложенные `{{c1::...}}`), MathJax 3, LaTeX, аудио (`[sound:...]`), HTML5 видео и встроенного синтезатора речи (TTS).
   - Ввод ответа (`{{type:...}}` и `{{type:cloze:...}}`) с посимвольным сравнением и подсветкой опечаток (diff).
2. **Веб-оболочка на Svelte 5 (Runes)**:
   - Полная дизайн-система Desktop Anki (`anki-tokens.css`) в светлом, темном и минималистичном стилях.
   - Все десктопные горячие клавиши: `Space`/`Enter`, `1`–`4`, `d`, `a`, `b`, `t`, `y`, `Ctrl+Z` (Undo), флаги `1`–`7`, метка `*`, заморозка `@`/`!`, скрытие `-`/`=`, сброс `Ctrl+Alt+N`, инфо о карте `i`/`Ctrl+Alt+I`.
   - 3-панельный виртуализированный браузер с поддержкой всего синтаксиса поисковых запросов Anki (`deck:...`, `tag:...`, `is:due`, `prop:reps>5`, `flag:1`).
   - Массовые операции: перемещение между колодами, добавление/удаление тегов, поиск и замена, поиск дубликатов, экспорт.
3. **Мобильная адаптация и PWA**:
   - Установка как автономного приложения на iPhone (Safari: «Поделиться» → «На экран „Домой“») и Android (Chrome: «Установить приложение»).
   - Крупные тач-кнопки оценки ответа (высота 44px, виброотклик при наличии).
   - Сервис-воркер (`sw.js`) и манифест (`manifest.webmanifest`) для кеширования оболочки и работы офлайн.
4. **Встроенный сервер синхронизации Anki Sync Server**:
   - Прямая синхронизация с мобильными приложениями **AnkiDroid** и **AnkiMobile** без сторонних сервисов (по протоколам `/sync/*` и `/msync/*`).
5. **Безопасность и автоматические бэкапы**:
   - Изолированные коллекции для каждого пользователя на защищенном томе данных.
   - Сессионная аутентификация PBKDF2 + HMAC-SHA256 HttpOnly cookie + изоляция токеном `X-Cardly-Api-Token` от вредоносного JS в карточках.
   - Автоматическая ротация бэкапов (хранение последних 50 версий `.colpkg` с возможностью восстановления в один клик).

---

## 🚀 Быстрый запуск через Docker Compose

### 1. Создание файла конфигурации `.env`
```bash
cp .env.example .env
```
Задайте учетные данные пользователей:
```ini
CARDLY_USER1=alice:secret123
SYNC_USER1=alice:secret123
CARDLY_PORT=8080
```

### 2. Запуск контейнера
```bash
docker compose up -d
```
После запуска приложение доступно по адресу: **`http://localhost:8080`**.

---

## 📱 Настройка синхронизации с AnkiDroid и AnkiMobile

Cardly содержит полноценный сервер синхронизации Anki, поэтому вы можете синхронизировать колоды между веб-версией и смартфонами:

### AnkiDroid (Android):
1. Откройте **Настройки** → **Синхронизация** → **Пользовательский сервер синхронизации**.
2. В поле **URL сервера синхронизации** укажите: `http://<IP-вашего-сервера>:8080/sync/`
3. В поле **URL синхронизации медиа** укажите: `http://<IP-вашего-сервера>:8080/msync/`
4. Введите имя пользователя и пароль из `.env`.
5. Нажмите кнопку синхронизации.

### AnkiMobile (iOS):
1. Откройте **Preferences** → **Syncing** → **Custom Server**.
2. Укажите адрес сервера: `http://<IP-вашего-сервера>:8080/sync/`
3. Авторизуйтесь под вашим логином и паролем.

---

## 🛠 Локальная разработка и тестирование

### Требования
- Rust 1.80+ (`cargo`)
- Node.js 20+ / 22+ (`npm`)
- Библиотеки SQLite и protobuf

### Сборка и запуск локально
```bash
# 1. Сборка фронтенд-оболочки (Svelte 5)
cd web
npm install
npm run build
cd ..

# 2. Запуск Rust-сервера
CARDLY_USER1="alice:secret123" \
CARDLY_PORT=8080 \
CARDLY_SHELL_DIR="./web/dist" \
CARDLY_UPSTREAM_WEB_DIR="./anki-upstream/out/qt/_aqt/data/web" \
cargo run --manifest-path server/Cargo.toml
```

### Автоматические тесты
```bash
# Интеграционные тесты Rust сервера (включая протокол синхронизации Anki)
cargo test --manifest-path server/Cargo.toml --test integration

# Проверка типов Svelte 5 компонентов
npm --prefix web run check

# Сквозные Playwright E2E тесты (десктопные и мобильные сценарии)
npm --prefix e2e run test
```

---

## 📄 Лицензия

Проект распространяется под лицензией **GNU AGPLv3 or later** (GNU Affero General Public License v3.0). Текст лицензии доступен в файле [LICENSE](./LICENSE).
Код оригинального Anki используется в соответствии с его лицензией AGPL-3.0-or-later.
