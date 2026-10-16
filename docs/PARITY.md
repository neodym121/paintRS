# PARITY — чек-лист паритета с оригиналом

Колонки:
- **Ориг.** — поведение оригинала: `✓` проверено на оригинале (Computer use, 16.10.2026),
  `м` — видно в меню/подсказке, но не прогнано, `—` **не проверено на оригинале**.
- **Статус** PaintRS: `todo` · `done` · `diff` (осознанное расхождение, см. `behavior/DIFFS.md`) · `skip` (вне рамок).
- **Эт.** — этап плана (раздел 9 ТЗ). **Ссылка** — описание/эталон/тест.

## Окно и раскладка
| ID | Функция | Ориг. | Статус | Эт. | Ссылка |
|---|---|---|---|---|---|
| L1 | Заголовок «имя — PaintRS», «Без имени» | ✓ | done | 1 | file-menu.md |
| L2 | «*» при несохранённых изменениях | ✓ (нет) | diff | 1 | DIFFS.md |
| L3 | Строка меню File/Edit/View + Save/Undo/Redo | ✓ | done (пункты будущих этапов неактивны) | 1 | layout.md |
| L4 | Лента: группы и порядок (Selection, Image, Tools, Brushes, Shapes, Colors, Layers) | ✓ | done (раскладка; действия — по этапам) | 1 | layout.md |
| L5 | Свернуть/автоскрытие ленты | м | todo | 9 | view.md |
| L6 | Рабочая область, холст по центру, рамка | ✓ | done | 1 | layout.md |
| L7 | 8 ручек размера холста | ✓ | todo | 7 | image-menu.md |
| L8 | Боковые плавающие ползунки (Size/Opacity/Tolerance) | ✓ | done (шкала 91 позиция, стопкой/рядом, ↑↓ Home End) | 2 | tools.md, tool::size_scale_matches_the_original |
| L9 | Строка состояния: курсор, выделение, размер, размер файла, масштаб | ✓ | done (размер выделения — этап 5) | 1 | statusbar.md |
| L10 | Страница настроек (тема и др.) | ✓ | done (тема, язык, сглаживание) | 10 | dialogs.md |
| L11 | Тёмная/светлая/системная тема | ✓ | done | 10 | — |
| L12 | Локализация RU/EN | — | todo | 10 | — |
| L13 | DPI-масштабирование интерфейса | — | todo | 1, 9 | — |

## Вид
| ID | Функция | Ориг. | Статус | Эт. | Ссылка |
|---|---|---|---|---|---|
| V1 | Масштаб 10–1000%, шаги 10/25/50 | ✓ | done (тест zoom::steps_match_the_original) | 1 | view.md |
| V2 | Пресеты 1000…12.5% в списке, ввод своего значения | ✓ | done | 1 | view.md |
| V3 | Fit to window | ✓ | done | 1 | view.md |
| V4 | Ctrl+0 / Ctrl+PgUp / Ctrl+PgDn | ✓ | done | 1 | shortcuts.md |
| V5 | Ctrl+колесо, колесо, Shift+колесо | — | done (шаг на щелчок колеса, привязка к курсору) | 1 | — |
| V6 | Линейки Ctrl+R (деления 10/100) | ✓ | todo | 9 | view.md |
| V7 | Сетка Ctrl+G (каждые 10 px) | ✓ | todo | 9 | view.md |
| V8 | Полный экран F11 | м | done | 9 | view.md |
| V9 | Миниатюра (Thumbnail) | — | todo | 9 | view.md |
| V10 | Шахматка прозрачности 8×8 | ✓ | done (свои оттенки) | 1 | view.md |
| V11 | Сглаживание отображения авто/всегда/никогда, сетка пикселей ≥800% | — (расширение) | diff, done | 1, 9 | DIFFS.md |
| V12 | Скрыть строку состояния | м | done | 9 | view.md |

## Инструменты
| ID | Функция | Ориг. | Статус | Эт. | Ссылка |
|---|---|---|---|---|---|
| T1 | Карандаш 1 px, 8-связный, без AA | ✓ | done (сверено попиксельно) | 2 | draw::pencil_1px_line_is_exact |
| T2 | Карандаш >1 px — круглый без AA | ✓ | done (1–10 px: 0 отличий от оригинала) | 2 | tools.md |
| T3 | Карандаш Shift — прямая под 45° | ✓ | done | 2 | draw::snap_to_45 |
| T4 | ЛКМ цвет 1 / ПКМ цвет 2 | ✓ | done | 2 | tools.md |
| T5 | Непрозрачность карандаша/кистей 0–100% | ✓ | done для карандаша (без накопления внутри штриха, 127/63 как в оригинале); кисти — этап 3 | 2 | draw::translucent_pencil_does_not_darken_on_overlap |
| T6 | Ластик ЛКМ → прозрачность, квадрат | ✓ | done (8 px по умолчанию) | 2 | draw::eraser_clears_square_and_replaces_colors |
| T7 | Ластик ПКМ — замена цвета 1→2 | ✓ | done | 2 | ref-eraser-rmb-replace.png |
| T8 | Заливка, допуск 0–100% (по умолч. 0), ПКМ цвет 2 | ✓ | done | 2 | draw::fill_* |
| T9 | Заливка в пределах выделения | — | todo | 5 | tools.md |
| T10 | Пипетка ЛКМ/ПКМ, возврат к прежнему инструменту | ✓ (ЛКМ, возврат) | done | 2 | tools.md |
| T11 | Лупа ЛКМ/ПКМ, остаётся активной | ✓ | done | 2 | tools.md |
| T12 | Клавиши S/P/B/T/E/I/Z | ✓ | done | 2 | shortcuts.md |
| T13 | Курсоры по инструментам | — | частично: отпечаток карандаша под указателем (✓ на оригинале), рамка ластика | 10 | tools.md |

## Кисти
| ID | Функция | Ориг. | Статус | Эт. | Ссылка |
|---|---|---|---|---|---|
| B1 | Brush (круг, край 1 px) | ✓ | done (разница с оригиналом 0,57 уровня) | 3 | brush::round_brush_matches_measured_profile |
| B2 | Calligraphy brush (перо «/») | ✓ | done | 3 | brush::calligraphy_band_height_and_thin_direction |
| B3 | Calligraphy pen (перо «\») | ✓ | done | 3 | brushes.md |
| B4 | Airbrush (бинарные точки, накопление) | ✓ | done (по событиям и по времени) | 3 | brush::airbrush_sprays_binary_dots_inside_the_disc |
| B5 | Oil brush (полосы, сужение в начале) | ✓ | done (текстура приблизительная, см. DIFFS) | 3 | brush::oil_starts_narrow |
| B6 | Crayon (рваный край, зерно) | ✓ | done (текстура приблизительная) | 3 | brushes.md |
| B7 | Marker (квадратное перо, 75%, накопление на пересечениях) | ✓ | done (совпадает) | 3 | brush::marker_is_flat_75_percent_and_builds_up_on_crossing |
| B8 | Natural pencil (зерно, светлый) | ✓ | done (тон совпадает, зерно своё) | 3 | brushes.md |
| B9 | Watercolor (разводы, капля в конце) | ✓ | done без «капли» (не воспроизвелась на оригинале) | 3 | brushes.md |
| B10 | Мин. размер 1 / 4 px, макс. 248 px, нелинейная шкала | ✓ | done | 3 | tool::size_scale_matches_the_original |
| B11 | Эталоны 1/5/20 px, Opacity<100% | ✓ (1/5/10/20/36 px) | done (Opacity<100% у текстурных — не сверено) | 3 | brushes.md |

## Фигуры
| ID | Функция | Ориг. | Статус | Эт. | Ссылка |
|---|---|---|---|---|---|
| S1 | 23 фигуры в порядке галереи | ✓ | todo | 4 | shapes.md |
| S2 | Outline: нет/сплошной/5 кистевых | ✓ | todo | 4 | shapes.md |
| S3 | Fill: нет/сплошная/5 кистевых | ✓ | todo | 4 | shapes.md |
| S4 | Толщина 1/3/5/8 px (по умолч. 5) | ✓ | todo | 4 | shapes.md |
| S5 | Черновая фигура: рамка, 8 ручек, поворот, живое изменение цвета/стиля | ✓ | todo | 4 | shapes.md |
| S6 | Shift: квадрат/круг (по меньшей стороне), линия 45° | ✓ | todo | 4 | ui-rect-shift-square.png, ref-shapes-shift.png |
| S7 | Фигуры без сглаживания (как в оригинале) или с AA (ТЗ) | ✓ | diff? | 4 | DIFFS.md (вопрос) |
| S8 | Кривая (изгибы), многоугольник (замыкание) | — | todo | 4 | shapes.md |
| S9 | Enter — фиксация, Esc — отмена | ✓ (Enter) | todo | 4 | shapes.md |

## Выделение
| ID | Функция | Ориг. | Статус | Эт. | Ссылка |
|---|---|---|---|---|---|
| E1 | Прямоугольное выделение, 8 ручек, ручка поворота | ✓ | todo | 5 | selection.md |
| E2 | Произвольное (лассо) | м | todo | 5 | selection.md |
| E3 | Ctrl+A, Invert selection, Delete → прозрачность | ✓ (A, Delete) | todo | 5 | selection.md |
| E4 | Перемещение: место → прозрачное, наложение с альфой | ✓ | todo | 5 | ui-selection-move.png |
| E5 | Ctrl+drag копия, Shift+drag шлейф, стрелки 1 px | — | todo | 5 | selection.md |
| E6 | Transparent selection | м | todo | 5 | selection.md |
| E7 | Cut/Copy/Paste, Copy visible layers | ✓ (Copy/Paste) | todo | 5 | selection.md |
| E8 | Вставка больше холста — авторасширение без вопроса | ✓ | diff? | 5 | DIFFS.md (вопрос) |
| E9 | Crop (Ctrl+Shift+X) | м | todo | 5 | image-menu.md |
| E10 | Контекстное меню холста | ✓ | todo | 5 | selection.md |
| E11 | Поворот/отражение/размер выделения | — | todo | 5, 7 | selection.md |

## Текст
| ID | Функция | Ориг. | Статус | Эт. | Ссылка |
|---|---|---|---|---|---|
| X1 | Текстовая рамка: клик/протяжка, рост, выход за холст | ✓ | todo | 6 | text.md |
| X2 | Панель: шрифт, размер (8…256), Ж/К/Ч/З, выравнивание, фон | ✓ | todo | 6 | text.md |
| X3 | Форматирование фрагмента (rich text), Ctrl+B/I/U | ✓ | todo | 6 | text.md |
| X4 | Background fill — вся рамка цветом 2 | ✓ | todo | 6 | text.md |
| X5 | Esc — отмена, щелчок вне рамки — запекание | ✓ | todo | 6 | text.md |
| X6 | Изменение размера/перемещение/поворот рамки | — | todo | 6 | text.md |

## Изображение
| ID | Функция | Ориг. | Статус | Эт. | Ссылка |
|---|---|---|---|---|---|
| I1 | Поворот 90°/90°/180° | м | todo | 7 | image-menu.md |
| I2 | Произвольный поворот (по часовой, холст до габарита) | ✓ | todo | 7 | image-menu.md |
| I3 | Отражение по вертикали/горизонтали | м | todo | 7 | image-menu.md |
| I4 | Resize and skew: %/px, пропорции, наклон | ✓ (размер) | todo | 7 | image-menu.md |
| I5 | Сглаживающий ресемплинг (как в оригинале визуально) | ✓ | todo | 7 | image-menu.md |
| I6 | Свойства изображения: единицы, размер, Default | ✓ | todo | 7 | dialogs.md |
| I7 | Ручки холста: изменение размера с любой стороны | ✓ | todo | 7 | image-menu.md |
| I8 | Invert color (Ctrl+Shift+I) | м | todo | 7 | image-menu.md |

## Цвета
| ID | Функция | Ориг. | Статус | Эт. | Ссылка |
|---|---|---|---|---|---|
| C1 | Цвет 1/2, активный слот | ✓ | done | 2 | colors.md |
| C2 | Палитра 10×3: ЛКМ активный, ПКМ неактивный | ✓ | done | 2 | colors.md |
| C3 | Своя палитра значений | ✓ | diff | 2 | DIFFS.md |
| C4 | Диалог цвета: спектр, яркость, HEX, RGB/HSV, 48+24 | ✓ | done (свои 48 основных цветов) | 2 | colors.md |
| C5 | OK → в ряд пользовательских цветов | ✓ | done (без дублей, при переполнении — сдвиг) | 2 | colors.md |

## Слои
| ID | Функция | Ориг. | Статус | Эт. | Ссылка |
|---|---|---|---|---|---|
| Y1 | Модель: фон-цвет + прозрачные слои | ✓ | todo | 1 (ядро), 8 | layers.md |
| Y2 | Панель: +, миниатюры, активный, глаз | ✓ | todo | 8 | layers.md |
| Y3 | Меню слоя: скрыть, дублировать, объединить вниз, выше/ниже, удалить (+клавиши) | ✓ | todo | 8 | layers.md |
| Y4 | Фон: скрыть (шахматка), сменить цвет | ✓ | todo | 8 | layers.md |
| Y5 | Плашка «активный слой скрыт» | ✓ | todo | 8 | layers.md |
| Y6 | Прозрачность слоя | ✓ (нет в ориг.) | diff | 8 | DIFFS.md |
| Y7 | Порядок перетаскиванием | — | todo | 8 | layers.md |

## Файлы
| ID | Функция | Ориг. | Статус | Эт. | Ссылка |
|---|---|---|---|---|---|
| F1 | New/Open/Save/Save as (меню форматов) | ✓ | todo | 1, 8 | file-menu.md |
| F2 | PNG RGBA + 96 DPI | ✓ | done (тест io::png_has_96_dpi) | 1 | file-menu.md |
| F3 | JPEG без диалога, q≈94, 4:2:0 | ✓ | todo | 8 | file-menu.md |
| F4 | BMP, GIF, TIFF, ICO, WebP | — | todo | 8 | file-menu.md |
| F5 | «Сохранить сведённым?» для многослойного | ✓ | todo | 8 | dialogs.md |
| F6 | Вопрос о несохранённых изменениях | ✓ | done | 1 | dialogs.md |
| F7 | Новый холст = последний размер; Default = 0,6 × экран | ✓ | done (Default — этап 7) | 1 | file-menu.md |
| F8 | Проект со слоями (Save as project) | м | diff (свой формат) | 8 | DIFFS.md |
| F9 | Недавние файлы | м | done | 8 | file-menu.md |
| F10 | Drag-and-drop, аргумент командной строки | — | done | 8 | — |
| F11 | Печать / предпросмотр / параметры страницы (или PDF) | м | todo | 10 | file-menu.md |
| F12 | Import to canvas → From a file | м | todo | 8 | file-menu.md |

## История
| ID | Функция | Ориг. | Статус | Эт. | Ссылка |
|---|---|---|---|---|---|
| H1 | Undo/Redo Ctrl+Z/Ctrl+Y | ✓ | done | 2 | history::undo_redo_pixels |
| H2 | Глубина ≈100 шагов (наблюдалось 101) | ✓ | done в ядре (тест history::limit_is_respected) | 2 | — |
| H3 | Создание слоя — не в истории; скрытие — в истории | ✓ | todo | 8 | layers.md |

## Вне рамок (skip)
Copilot, Remove background, Generative erase, ИИ-ластик, ИИ-водяной знак, Share, Feedback,
Set as desktop background, From scanner or camera.
