/**
 * Cardly Landing Page Application Script
 * Theme toggle, Copy button, Multilingual i18n (EN, RU, KK), FAQ Accordion
 */

// ============================================================================
// Global Configuration (Single Source of Truth for URLs)
// ============================================================================
const CONFIG = {
  // Public live demo URL (HTTPS required for PWA installation on mobile)
  DEMO_URL: "https://my-cardly.duckdns.org",
  // Official open-source repository
  REPO_URL: "https://github.com/cardly-anki/cardly",
  // Frequency of demo data purge & sample re-seeding
  RESET_INTERVAL_HOURS: 2,
};

const translations = {
  en: {
    nav_features: "Features",
    nav_demo: "Showcase",
    nav_comparison: "Comparison",
    nav_faq: "FAQ",
    badge_anki: "Anki Core Compatible",
    badge_fsrs: "FSRS-6 Core",
    badge_selfhosted: "Self-Hosted Docker",
    badge_agpl: "GNU AGPL-3.0",
    hero_title: "The Open-Source Spaced Repetition App <br /><span class=\"gradient-text\">for Web &amp; Mobile</span>",
    hero_subtitle: "A faithful, self-hosted web app for Anki collections powered directly by Anki's official Rust core (rslib, Anki 26.09.4). FSRS-6 scheduling (same as upstream Anki 26.09.4), installable PWA, and Anki-compatible sync server.",
    cta_demo: "Try Live Demo",
    cta_github: "View on GitHub",
    cta_quickstart: "Quickstart via Docker",
    demo_reset_note: "Public Demo: data resets every 2 hours • No registration required",
    demo_offline_title: "Live Demo Server Cycling",
    demo_offline_desc: "The demo server is currently executing its scheduled reset or temporarily offline. You can preview all features in the workflow video below or run Cardly locally via Docker in 10 seconds.",
    cta_watch_video: "Watch Video Preview",
    cta_quickstart_link: "Docker Quickstart",
    snippet_label: "One-Command Quickstart",
    copy_btn: "Copy",
    copied_toast: "Copied 'docker compose up -d' to clipboard!",
    demo_kicker: "SHOWCASE & EXPERIENCE",
    demo_title: "Desktop-Grade Experience in Your Browser",
    demo_desc: "Watch a complete study workflow: login, rich card creation, review with FSRS ease intervals, 3-pane browser search, and statistics.",
    feat_kicker: "CORE CAPABILITIES",
    feat_title: "Built on Upstream Anki Core",
    feat_desc: "Cardly does not rewrite or approximate Anki's algorithms. It links directly against upstream Rust libraries (Anki 26.09.4) and reusable Svelte components.",
    feat_1_title: "Official Anki Core (rslib)",
    feat_1_desc: "Directly links the official Rust core. Every schedule calculation, database schema mutation, and SQLite transaction is executed by upstream Anki code.",
    feat_2_title: "FSRS-6 Scheduler",
    feat_2_desc: "FSRS-6 scheduler, same as upstream Anki 26.09.4 (21 parameters, backward compatible with older FSRS v4/v5 parameters), with in-browser optimizer and retention simulator.",
    feat_3_title: "MathJax 3, Cloze & Audio",
    feat_3_desc: "Client-side MathJax formulas, nested cloze deletions, typed answer character diffing, HTML5 audio, video, and Web Speech API text-to-speech.",
    feat_4_title: "3-Pane Virtualized Browser",
    feat_4_desc: "Desktop-grade card browser with full search syntax (deck:, tag:, is:due), sortable columns, and bulk batch actions.",
    feat_5_title: "Mobile PWA & Touch Controls",
    feat_5_desc: "Installable on iPhone and Android with standalone display, service worker offline shell, safe area insets, and ergonomic 44px touch rating buttons.",
    feat_6_title: "Built-in Sync Server",
    feat_6_desc: "Uses Anki's own sync protocol (/sync/* and /msync/*, verified with Anki Desktop). AnkiDroid and AnkiMobile: not yet tested, reports welcome.",
    comp_kicker: "HONEST ARCHITECTURE",
    comp_title: "How Cardly Compares",
    comp_desc: "A transparent look at the differences between Cardly, official AnkiWeb, and desktop Anki.",
    th_feature: "Feature / Dimension",
    r_deployment: "Deployment & Access",
    r_deployment_cardly: "Self-hosted Web & Mobile PWA",
    r_deployment_ankiweb: "Proprietary Anki Cloud",
    r_deployment_desktop: "Native OS Install (PyQt6/Python)",
    r_engine: "Core Scheduling Engine",
    r_engine_cardly: "Official Rust rslib linked directly",
    r_engine_ankiweb: "Custom Web Backend",
    r_engine_desktop: "Native Rust rslib",
    r_fsrs: "FSRS-6 & Optimizer",
    r_fsrs_cardly: "Full support + online optimizer & simulator",
    r_fsrs_ankiweb: "Review only (no optimizer)",
    r_fsrs_desktop: "Full support + optimizer",
    r_browser: "Card Browser",
    r_browser_cardly: "Full 3-pane virtualized with bulk actions",
    r_browser_ankiweb: "Read-only basic search list",
    r_browser_desktop: "Full 3-pane Qt browser",
    r_templates: "Card Template Editor",
    r_templates_cardly: "Full split-view HTML/CSS editor with live preview",
    r_templates_ankiweb: "Very limited",
    r_templates_desktop: "Full split-view editor",
    r_sync: "Mobile Sync Protocol",
    r_sync_cardly: "Built-in Sync Server (verified with Desktop)",
    r_sync_ankiweb: "Sync server for all apps",
    r_sync_desktop: "Sync client",
    r_privacy: "Data Privacy & Control",
    r_privacy_cardly: "Fully self-hosted on your hardware",
    r_privacy_ankiweb: "Stored on AnkiWeb servers",
    r_privacy_desktop: "Stored locally on computer",
    r_addons: "Add-on Support",
    r_addons_cardly: "Web-safe components & Svelte extensions",
    r_addons_ankiweb: "No add-ons",
    r_addons_desktop: "Full Python / PyQt add-ons",
    faq_kicker: "FREQUENTLY ASKED QUESTIONS",
    faq_title: "Got Questions? We Have Answers",
    faq_1_q: "Is Cardly an official Anki product?",
    faq_1_a: "No. Cardly is an independent, open-source project. It is not affiliated with, sponsored by, or endorsed by Damien Elmes or Ankitects Pty Ltd. It links directly against the open-source Rust libraries of Anki in full compliance with GNU AGPL-3.0.",
    faq_2_q: "How does Cardly calculate intervals compared to Anki Desktop?",
    faq_2_a: "Cardly directly invokes Anki's official Rust core (rslib) rather than approximating spaced repetition math in JavaScript. Every interval, FSRS power formula, and scheduling state transition runs through upstream Anki code.",
    faq_3_q: "How much does it cost to self-host Cardly?",
    faq_3_a: "Cardly is completely free and open-source software. You can host it on any cheap cloud VPS ($4–$5/month on Hetzner, DigitalOcean, Linode), deploy it on a home Raspberry Pi, or run it on modern serverless containers (Fly.io, Railway) within their free/hobby tiers.",
    faq_4_q: "How does sync work with AnkiDroid (Android) and AnkiMobile (iOS)?",
    faq_4_a: "Cardly uses Anki's own HTTP sync protocol (verified with Anki Desktop). AnkiDroid and AnkiMobile are not yet tested, reports welcome. You can enter your Cardly URL (http://<IP>:8080/sync/) under Custom Sync Server in client settings.",
    faq_5_q: "What open-source license does Cardly use?",
    faq_5_a: "Cardly is licensed under the GNU Affero General Public License v3 or later (AGPL-3.0-or-later), matching the upstream licensing of the Anki project. See the NOTICE file for complete contributor and third-party credits.",
    footer_desc: "The open-source spaced repetition app for web and mobile. Built with love on official Anki core.",
    col_project: "Project",
    col_community: "Community",
  },
  ru: {
    nav_features: "Возможности",
    nav_demo: "Демо",
    nav_comparison: "Сравнение",
    nav_faq: "Вопросы и ответы",
    badge_anki: "Совместимо с ядром Anki",
    badge_fsrs: "Ядро FSRS-6",
    badge_selfhosted: "Self-Hosted Docker",
    badge_agpl: "Лицензия GNU AGPL-3.0",
    hero_title: "Веб-приложение интервального повторения <br /><span class=\"gradient-text\">для компьютеров и смартфонов</span>",
    hero_subtitle: "Веб-клиент для колод Anki на базе официального ядра Anki на Rust (rslib, Anki 26.09.4). Планировщик FSRS-6 (как в upstream Anki 26.09.4), автономный PWA и встроенный сервер синхронизации Anki.",
    cta_demo: "Открыть демо-версию",
    cta_github: "Код на GitHub",
    cta_quickstart: "Запуск через Docker",
    demo_reset_note: "Публичное демо: сброс данных каждые 2 часа • Без регистрации",
    demo_offline_title: "Демо-сервер перезапускается",
    demo_offline_desc: "Демо-сервер сейчас выполняет плановый сброс данных или временно недоступен. Вы можете посмотреть видео интерфейса ниже или запустить Cardly локально через Docker за 10 секунд.",
    cta_watch_video: "Смотреть видео",
    cta_quickstart_link: "Docker инструкция",
    snippet_label: "Запуск одной командой",
    copy_btn: "Копировать",
    copied_toast: "Команда 'docker compose up -d' скопирована в буфер обмена!",
    demo_kicker: "ДЕМОНСТРАЦИЯ ИНТЕРФЕЙСА",
    demo_title: "Возможности Desktop Anki в вашем браузере",
    demo_desc: "Посмотрите реальный сценарий обучения: авторизация, создание карточек с Cloze, повторение с интервалами FSRS, поиск в 3-панельном браузере и графики статистики.",
    feat_kicker: "КЛЮЧЕВЫЕ ВОЗМОЖНОСТИ",
    feat_title: "На базе оригинального ядра Anki",
    feat_desc: "Cardly не эмулирует алгоритмы Anki на JS. Сервер напрямую подключает официальное ядро Rust (Anki 26.09.4) и компоненты Svelte.",
    feat_1_title: "Оригинальное ядро Anki (rslib)",
    feat_1_desc: "Прямой линкинг Rust-библиотеки Anki. Все расчеты интервалов, изменения структуры SQLite и транзакции выполняются кодом Anki.",
    feat_2_title: "Планировщик FSRS-6",
    feat_2_desc: "Планировщик FSRS-6, как в upstream Anki 26.09.4 (21 параметр, обратная совместимость со старыми параметрами FSRS v4/v5), с оптимизатором и симулятором удержания.",
    feat_3_title: "MathJax 3, Cloze и медиа",
    feat_3_desc: "Формулы MathJax 3, вложенные пропуски Cloze, посимвольная проверка ввода ответа, HTML5 аудио/видео и синтез речи TTS.",
    feat_4_title: "3-панельный браузер карточек",
    feat_4_desc: "Виртуализированный браузер с поддержкой всего синтаксиса поиска Anki (deck:, tag:, is:due), сортировкой колонок и массовыми правками.",
    feat_5_title: "Мобильный PWA и тач-контрол",
    feat_5_desc: "Установка на iPhone и Android как автономного приложения, сервис-воркер офлайн-оболочки и крупные кнопки оценки 44px.",
    feat_6_title: "Встроенный сервер синхронизации",
    feat_6_desc: "Использует собственный протокол синхронизации Anki (/sync/* и /msync/*, проверено с Anki Desktop). AnkiDroid и AnkiMobile пока не тестировались, будем рады отчётам.",
    comp_kicker: "ЧЕСТНОЕ СРАВНЕНИЕ",
    comp_title: "Чем Cardly отличается от аналогов",
    comp_desc: "Наглядное сопоставление возможностей Cardly, официального сервиса AnkiWeb и десктопной программы Anki.",
    th_feature: "Функция / Параметр",
    r_deployment: "Развертывание и доступ",
    r_deployment_cardly: "Self-hosted веб-приложение и PWA",
    r_deployment_ankiweb: "Закрытый облачный сервис Anki",
    r_deployment_desktop: "Локальная установка (PyQt6/Python)",
    r_engine: "Ядро планировщика",
    r_engine_cardly: "Официальное ядро Rust rslib напрямую",
    r_engine_ankiweb: "Собственный веб-бэкенд",
    r_engine_desktop: "Нативное ядро Rust rslib",
    r_fsrs: "FSRS-6 и оптимизатор",
    r_fsrs_cardly: "Полная поддержка + онлайн оптимизатор",
    r_fsrs_ankiweb: "Только повторения (без оптимизатора)",
    r_fsrs_desktop: "Полная поддержка + оптимизатор",
    r_browser: "Браузер карточек",
    r_browser_cardly: "3-панельный с массовыми операциями",
    r_browser_ankiweb: "Упрощенный список только для чтения",
    r_browser_desktop: "3-панельный на Qt",
    r_templates: "Редактор шаблонов",
    r_templates_cardly: "Полный split-редактор с предпросмотром",
    r_templates_ankiweb: "Сильно ограничен",
    r_templates_desktop: "Полный редактор шаблонов",
    r_sync: "Синхронизация с телефонами",
    r_sync_cardly: "Встроенный сервер (проверено с Desktop)",
    r_sync_ankiweb: "Сервер синхронизации для всех",
    r_sync_desktop: "Клиент синхронизации",
    r_privacy: "Приватность и владение данными",
    r_privacy_cardly: "Полный контроль на вашем собственном сервере",
    r_privacy_ankiweb: "Хранится на серверах AnkiWeb",
    r_privacy_desktop: "Хранится локально на ПК",
    r_addons: "Поддержка плагинов",
    r_addons_cardly: "Веб-компоненты и расширения Svelte",
    r_addons_ankiweb: "Нет плагинов",
    r_addons_desktop: "Полная поддержка Python / PyQt",
    faq_kicker: "ЧАСТО ЗАДАВАЕМЫЕ ВОПРОСЫ",
    faq_title: "Ответы на популярные вопросы",
    faq_1_q: "Является ли Cardly официальным продуктом Anki?",
    faq_1_a: "Нет. Cardly — независимый проект с открытым кодом. Он не аффилирован с Дэмиеном Элмсом и компанией Ankitects Pty Ltd. Проект использует открытый код ядра Anki в соответствии с условиями лицензии GNU AGPL-3.0.",
    faq_2_q: "Как Cardly рассчитывает интервалы по сравнению с Anki Desktop?",
    faq_2_a: "Cardly напрямую вызывает официальное Rust-ядро Anki (rslib), а не приблизительно воспроизводит формулы на JS. Все формулы FSRS и транзакции SQLite исполняются кодом Anki.",
    faq_3_q: "Сколько стоит хостинг Cardly?",
    faq_3_a: "Cardly полностью бесплатен и имеет открытый исходный код. Вы можете развернуть его на недорогом VPS ($4–$5/мес на Hetzner, DigitalOcean), домашнем сервере Raspberry Pi или бесплатно на Fly.io и Railway в рамках базовых тарифов.",
    faq_4_q: "Как настроить синхронизацию с AnkiDroid и AnkiMobile?",
    faq_4_a: "Cardly использует собственный HTTP-протокол синхронизации Anki (проверено с Anki Desktop). AnkiDroid и AnkiMobile пока не тестировались, будем рады отчётам. В настройках клиента укажите адрес вашего Cardly (http://<IP>:8080/sync/).",
    faq_5_q: "Какая лицензия используется?",
    faq_5_a: "Cardly распространяется по лицензии GNU Affero General Public License v3 or later (AGPL-3.0-or-later), полностью соответствуя условиям базового репозитория Anki. Подробности указаны в файле NOTICE.",
    footer_desc: "Приложение интервального повторения для компьютеров и смартфонов. Создано на официальном ядре Anki.",
    col_project: "Проект",
    col_community: "Сообщество",
  },
  kk: {
    nav_features: "Мүмкіндіктер",
    nav_demo: "Демо",
    nav_comparison: "Салыстыру",
    nav_faq: "Сұрақ-жауап",
    badge_anki: "Anki өзегімен үйлесімді",
    badge_fsrs: "FSRS-6 өзегі",
    badge_selfhosted: "Self-Hosted Docker",
    badge_agpl: "GNU AGPL-3.0",
    hero_title: "Компьютер мен смартфондарға арналған <br /><span class=\"gradient-text\">интервалды қайталау жүйесі</span>",
    hero_subtitle: "Ресми Anki Rust өзегіне (rslib, Anki 26.09.4) негізделген Anki веб-клиенті. Anki 26.09.4 нұсқасындағыдай FSRS-6 жоспарлағышы, дербес PWA және кірістірілген синхрондау сервері.",
    cta_demo: "Демо нұсқасын көру",
    cta_github: "GitHub репозиторийі",
    cta_quickstart: "Docker арқылы қосу",
    demo_reset_note: "Ашық демо: деректер әр 2 сағат сайын жаңарады • Тіркелусіз",
    demo_offline_title: "Демо-сервер қайта іске қосылуда",
    demo_offline_desc: "Демо-сервер жоспарлы жаңарту кезеңінде. Төмендегі шолу видеосын көре аласыз немесе Cardly-ді 10 секундта Docker арқылы қоса аласыз.",
    cta_watch_video: "Видеоны көру",
    cta_quickstart_link: "Docker нұсқаулығы",
    snippet_label: "Бір командамен іске қосу",
    copy_btn: "Көшіру",
    copied_toast: "'docker compose up -d' командасы алмасу буферіне көшірілді!",
    demo_kicker: "ИНТЕРФЕЙС ШОЛУЫ",
    demo_title: "Браузердегі ыңғайлы десктоп деңгейі",
    demo_desc: "Нақты оқу процесін қараңыз: жүйеге кіру, Cloze карточкаларын жасау, FSRS аралықтарымен қайталау, 3-панельді шолғыш және статистика.",
    feat_kicker: "НЕГІЗГІ ЕРЕКШЕЛІКТЕР",
    feat_title: "Сапаға бағытталған сенімді архитектура",
    feat_desc: "Cardly алгоритмдерді жуықтап жасамайды. Сервер ресми Rust кітапханаларын және Svelte компоненттерін тікелей пайдаланады.",
    feat_1_title: "Түпнұсқа Anki өзегі (rslib)",
    feat_1_desc: "Ресми Rust өзегіне тікелей қосылған. Барлық есептеулер, мәліметтер базасының жазбалары түпнұсқа Anki кодымен орындалады.",
    feat_2_title: "FSRS-6 алгоритмі",
    feat_2_desc: "Anki 26.09.4 нұсқасындағыдай FSRS-6 жоспарлағышы (21 параметр, ескі FSRS v4/v5 параметрлерімен кері үйлесімді), оңтайландырғыш және есте сақтау симуляторы.",
    feat_3_title: "MathJax 3, Cloze және медиа",
    feat_3_desc: "MathJax 3 формулалары, құрамдас Cloze-бос орындар, жауапты теруді тексеру, HTML5 аудио/видео және мәтінді дауыстау (TTS).",
    feat_4_title: "3-панельді карточкалар шолғышы",
    feat_4_desc: "Anki іздеу синтаксисін (deck:, tag:, is:due), бағандарды сұрыптауды және жаппай өзгерістерді қолдайтын виртуалданған шолғыш.",
    feat_5_title: "Мобильді PWA және тач-басқару",
    feat_5_desc: "iPhone мен Android-қа автономды бағдарлама ретінде орнату, офлайн кэш және саусақпен басуға арналған 44px ірі батырмалар.",
    feat_6_title: "Кірістірілген синхрондау сервері",
    feat_6_desc: "Anki-дің ресми /sync/* және /msync/* протоколдарын қолданады (Anki Desktop-пен тексерілген). AnkiDroid пен AnkiMobile әлі тексерілмеген.",
    comp_kicker: "ШЫНАЙЫ САЛЫСТЫРУ",
    comp_title: "Cardly-дің өзге нұсқалардан айырмашылығы",
    comp_desc: "Cardly, ресми AnkiWeb қызметі мен Anki Desktop бағдарламаларының мүмкіндіктерін салыстыру.",
    th_feature: "Параметр / Функция",
    r_deployment: "Орнату және қолжетімділік",
    r_deployment_cardly: "Өз серверіңіздегі веб-қосымша және PWA",
    r_deployment_ankiweb: "Жабық AnkiWeb бұлттық жүйесі",
    r_deployment_desktop: "Компьютерге орнатылатын бағдарлама",
    r_engine: "Жоспарлағыш өзегі",
    r_engine_cardly: "Ресми Rust rslib кітапханасы",
    r_engine_ankiweb: "Арнайы веб-бэкенд",
    r_engine_desktop: "Нативті Rust rslib өзегі",
    r_fsrs: "FSRS-6 және оңтайландырғыш",
    r_fsrs_cardly: "Толық қолдау + онлайн баптау симуляторы",
    r_fsrs_ankiweb: "Тек қайталау (оңтайландырғыш жоқ)",
    r_fsrs_desktop: "Толық қолдау + оңтайландырғыш",
    r_browser: "Карточкалар шолғышы",
    r_browser_cardly: "Жаппай әрекеттері бар 3-панельді шолғыш",
    r_browser_ankiweb: "Тек көруге арналған қарапайым тізім",
    r_browser_desktop: "Qt негізіндегі 3-панельді шолғыш",
    r_templates: "Үлгілер редакторы",
    r_templates_cardly: "Алдын ала көруі бар толық split-редактор",
    r_templates_ankiweb: "Өте шектеулі",
    r_templates_desktop: "Толық үлгілер редакторы",
    r_sync: "Телефонмен синхрондау",
    r_sync_cardly: "Кірістірілген сервер (Desktop-пен тексерілген)",
    r_sync_ankiweb: "Барлық қосымшаға арналған сервер",
    r_sync_desktop: "Клиент ретінде қосылады",
    r_privacy: "Деректер құпиялылығы",
    r_privacy_cardly: "Өз бақылауыңызда (Self-hosted)",
    r_privacy_ankiweb: "AnkiWeb серверлерінде сақталады",
    r_privacy_desktop: "Тек өз компьютеріңізде сақталады",
    r_addons: "Қосымша плагиндер",
    r_addons_cardly: "Қауіпсіз веб-компоненттер мен Svelte",
    r_addons_ankiweb: "Плагиндер жоқ",
    r_addons_desktop: "Python / PyQt плагиндері",
    faq_kicker: "ЖИІ ҚОЙЫЛАТЫН СҰРАҚТАР",
    faq_title: "Маңызды сұрақтарға жауаптар",
    faq_1_q: "Cardly ресми Anki өнімі ме?",
    faq_1_a: "Жоқ. Cardly — тәуелсіз ашық бастапқы кодты жоба. Ол Дэмиен Элмспен немесе Ankitects компаниясымен ресми байланыспаған. Жоба Anki-дің ашық кодын GNU AGPL-3.0 лицензиясына сай пайдаланады.",
    faq_2_q: "Cardly қайталау аралықтарын қалай есептейді?",
    faq_2_a: "Басқа веб-жобалардан айырмашылығы — Cardly ресми Rust кітапханасына (rslib) тікелей сүйенеді. Барлық формулалар түпнұсқа Anki кодымен есептеледі.",
    faq_3_q: "Серверді ұстап тұру құны қанша?",
    faq_3_a: "Cardly толықтай тегін және ашық кодты бағдарлама. Оны кез келген арзан VPS серверде ($4–$5/айына Hetzner, DigitalOcean), үй шаруасындағы Raspberry Pi-де немесе Fly.io, Railway тегін тарифтерінде іске қосуға болады.",
    faq_4_q: "AnkiDroid және AnkiMobile синхрондауы қалай қосылады?",
    faq_4_a: "Cardly Anki-дің ресми HTTP синхрондау протоколын қолданады (Anki Desktop-пен тексерілген; AnkiDroid пен AnkiMobile әлі тексерілмеген). Клиент баптауларында Cardly мекенжайын көрсетіңіз (http://<IP>:8080/sync/).",
    faq_5_q: "Бағдарлама қандай лицензиямен таратылады?",
    faq_5_a: "Cardly ресми Anki жобасы сияқты GNU Affero General Public License v3 or later (AGPL-3.0-or-later) бойынша таратылады. Толық мәліметтер NOTICE құжатында көрсетілген.",
    footer_desc: "Веб пен мобильді құрылғыларға арналған интервалды қайталау қосымшасы. Ресми Anki өзегінде жасалған.",
    col_project: "Жоба",
    col_community: "Қоғамдастық",
  },
};

document.addEventListener("DOMContentLoaded", () => {
  // 1. Language Switcher
  let currentLang = localStorage.getItem("cardly_lang") || "en";
  if (!translations[currentLang]) currentLang = "en";

  function applyLanguage(lang) {
    currentLang = lang;
    localStorage.setItem("cardly_lang", lang);
    document.documentElement.lang = lang;

    document.querySelectorAll(".lang-btn").forEach((btn) => {
      btn.classList.toggle("active", btn.dataset.lang === lang);
    });

    const dict = translations[lang] || translations.en;
    document.querySelectorAll("[data-i18n]").forEach((el) => {
      const key = el.getAttribute("data-i18n");
      if (dict[key] !== undefined) {
        // If string contains HTML (e.g. hero_title), use innerHTML, else textContent
        if (dict[key].includes("<")) {
          el.innerHTML = dict[key];
        } else {
          el.textContent = dict[key];
        }
      }
    });
  }

  document.querySelectorAll(".lang-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      const lang = btn.dataset.lang;
      if (lang && translations[lang]) {
        applyLanguage(lang);
      }
    });
  });

  applyLanguage(currentLang);

  // 2. Theme Toggle
  const themeToggle = document.getElementById("themeToggle");
  let currentTheme = localStorage.getItem("cardly_theme") || "dark";

  function applyTheme(theme) {
    currentTheme = theme;
    localStorage.setItem("cardly_theme", theme);
    document.documentElement.setAttribute("data-theme", theme);
  }

  applyTheme(currentTheme);

  if (themeToggle) {
    themeToggle.addEventListener("click", () => {
      const nextTheme = currentTheme === "dark" ? "light" : "dark";
      applyTheme(nextTheme);
    });
  }

  // 3. Copy-to-clipboard snippet & toast
  const copyBtn = document.getElementById("copyBtn");
  const toast = document.getElementById("toast");

  function showToast(msg) {
    if (!toast) return;
    toast.textContent = msg;
    toast.classList.add("show");
    setTimeout(() => {
      toast.classList.remove("show");
    }, 2800);
  }

  if (copyBtn) {
    copyBtn.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText("docker compose up -d");
        const dict = translations[currentLang] || translations.en;
        showToast(dict.copied_toast || "Copied to clipboard!");
      } catch (err) {
        showToast("docker compose up -d");
      }
    });
  }

  // 4. Interactive FAQ Accordion
  document.querySelectorAll(".accordion-trigger").forEach((trigger) => {
    trigger.addEventListener("click", () => {
      const isExpanded = trigger.getAttribute("aria-expanded") === "true";
      const panel = trigger.nextElementSibling;

      // Close all other panels
      document.querySelectorAll(".accordion-trigger").forEach((other) => {
        if (other !== trigger) {
          other.setAttribute("aria-expanded", "false");
          if (other.nextElementSibling) {
            other.nextElementSibling.style.maxHeight = null;
          }
        }
      });

      // Toggle current panel
      if (isExpanded) {
        trigger.setAttribute("aria-expanded", "false");
        panel.style.maxHeight = null;
      } else {
        trigger.setAttribute("aria-expanded", "true");
        panel.style.maxHeight = panel.scrollHeight + "px";
      }
    });
  });

  // 5. Dynamic URL Injection from CONFIG
  document.querySelectorAll(".btn-live-demo").forEach((el) => {
    el.setAttribute("href", CONFIG.DEMO_URL);
  });
  document.querySelectorAll(".btn-view-github, .nav-gh").forEach((el) => {
    el.setAttribute("href", CONFIG.REPO_URL);
  });

  // 6. Demo Availability Healthcheck with Fallback
  async function checkDemoAvailability() {
    const noticeEl = document.getElementById("demoOfflineNotice");
    if (!noticeEl || CONFIG.DEMO_URL.includes("<YOUR_SUBDOMAIN>")) return;
    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), 2500);
      const res = await fetch(`${CONFIG.DEMO_URL}/healthz`, {
        method: "GET",
        mode: "cors",
        signal: controller.signal,
      });
      clearTimeout(timer);
      if (!res.ok) {
        noticeEl.style.display = "flex";
      }
    } catch {
      noticeEl.style.display = "flex";
    }
  }

  void checkDemoAvailability();
});
