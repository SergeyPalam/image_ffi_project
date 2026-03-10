//! # Модуль загрузки плагинов
//!
//! Этот модуль отвечает за динамическую загрузку внешних плагинов и получение доступа к их функциям.
//! Он использует библиотеку `libloading` для загрузки динамических библиотек (`.dll`, `.so`, `.dylib`)
//! и получения указателей на экспортированные функции, совместимые с C.
//!
//! ## Архитектура
//!
//! - `Plugin`: Обёртка над динамической библиотекой.
//! - `PluginInterface`: Структура, содержащая функцию `process_image`, экспортируемую плагином.
//!
//! Это позволяет главному приложению `image_processor` вызывать код плагина без знания его внутренней реализации.

use libloading::{Library, Symbol, library_filename};

use super::error::PluginError;
use std::ffi::{c_char, c_uchar, c_ulong, c_int};
use std::path::Path;

/// Интерфейс плагина.
#[repr(C)]
pub struct PluginInterface<'a> {
    /// Указатель на функцию обработки изображения, экспортируемую плагином.
    ///
    /// Эта функция должна иметь C-совместимую сигнатуру:
    ///
    /// ```c
    /// void process_image(
    ///     unsigned long width,
    ///     unsigned long height,
    ///     unsigned char* rgba_data,
    ///     const char* params_json
    /// );
    /// ```
    ///
    /// Плагин обязан экспортировать функцию с именем `process_image`.
    pub process_image: Symbol<
        'a,
        unsafe extern "C" fn(
            width: c_ulong,
            height: c_ulong,
            rgba_data: *mut c_uchar,
            params: *const c_char,
        ) -> c_int,
    >,
}

/// Обёртка над динамической библиотекой плагина.
///
/// Содержит загруженную библиотеку и предоставляет методы для получения
/// интерфейса плагина. Управляет временем жизни библиотеки.
/// При удалении объекта `Plugin`, библиотека автоматически выгружается.
pub struct Plugin {
    plugin: Library,
}

impl Plugin {
    /// Создаёт новый экземпляр `Plugin`, загружая динамическую библиотеку из файла.
    ///
    /// # Параметры
    ///
    /// - `filename`: путь к файлу динамической библиотеки (`.dll`, `.so`, `.dylib`).
    ///
    /// # Ошибки
    ///
    /// Возвращает `PluginError` в случае:
    ///
    /// - Невозможности загрузить библиотеку (файл не найден, повреждён, несовместим).
    /// - Ошибки при инициализации библиотеки.
    ///
    /// # Безопасность
    ///
    /// Использует `unsafe`, так как загрузка сторонней библиотеки потенциально опасна.
    /// Вызывающий должен убедиться в доверенности источника плагина.
    ///
    /// # Пример
    ///
    /// ``` no_run
    /// use std::path::Path;
    /// use image_processor::plugin_loader::Plugin;
    /// 
    /// let plugin = Plugin::new(Path::new("/dir/to/plugin"), "plugin_name")
    ///     .expect("Не удалось загрузить плагин");
    /// ```
    pub fn new(plugin_dir: &Path, plugin_name: &str) -> Result<Self, PluginError> {
        let lib_name = library_filename(plugin_name);
        Ok(Plugin {
            plugin: unsafe { Library::new(plugin_dir.join(lib_name)) }?,
        })
    }
    /// Получает интерфейс плагина, предоставляя доступ к экспортированным функциям.
    ///
    /// # Возврат
    ///
    /// Возвращает `PluginInterface`, содержащий функцию `process_image`.
    ///
    /// # Ошибки
    ///
    /// Возвращает `PluginError`, если:
    ///
    /// - Функция `process_image` не найдена в библиотеке.
    /// - Произошла ошибка при получении символа из библиотеки.
    ///
    /// # Безопасность
    ///
    /// Использует `unsafe`, так как получение символа из сторонней библиотеки требует доверия.
    ///
    /// # Пример
    ///
    /// ``` no_run
    /// use std::path::Path;
    /// use image_processor::plugin_loader::Plugin;
    /// 
    /// let plugin = Plugin::new(Path::new("/dir/to/plugin"), "plugin_name")
    ///     .expect("Не удалось загрузить плагин");
    /// let interface = plugin.interface()
    ///     .expect("Не удалось получить интерфейс плагина");
    /// // Теперь можно вызывать (interface.process_image)(...);
    /// ```
    pub fn interface(&self) -> Result<PluginInterface<'_>, PluginError> {
        Ok(PluginInterface {
            process_image: unsafe { self.plugin.get("process_image") }?,
        })
    }
}
