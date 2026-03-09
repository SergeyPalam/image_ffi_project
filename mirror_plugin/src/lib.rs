//! # Mirror Plugin
//!
//! Это нативный C-compatible плагин для зеркального отражения изображений в формате RGBA.
//! Он предоставляет функцию `process_image`, которая применяет горизонтальное, вертикальное
//! или комбинированное отражение к изображению.
//!
//! Плагин предназначен для интеграции с внешними системами через FFI (Foreign Function Interface)
//! и поддерживает динамическую загрузку, например, из хост-приложения `image_processor`.
//!
//! ## Особенности
//!
//! - Поддержка горизонтального отражения.
//! - Поддержка вертикального отражения.
//! - Поддержка одновременного отражения по обоим осям (180° поворот).
//! - Работает напрямую с сырыми RGBA-пикселями (`u8`).
//! - Отражение выполняется in-place — без дополнительных аллокаций памяти.
//!
//! ## Использование
//!
//! Этот плагин экспортирует одну функцию — `process_image`, которая вызывается из C-кода
//! или через `libloading` в Rust. Он принимает параметры в формате JSON.
//!
//! ## Формат параметров (JSON)
//!
//! Поддерживаются следующие поля:
//!
//! - `"horizontal"`: если `true`, изображение отражается по горизонтали (слева направо). По умолчанию — `false`.
//! - `"vertical"`: если `true`, изображение отражается по вертикали (сверху вниз). По умолчанию — `false`.
//!
//! Примеры:
//! ```json
//! {"horizontal": true}
//! ```
//! ```json
//! {"vertical": true}
//! ```
//! ```json
//! {"horizontal": true, "vertical": true}
//! ```
//!
//! Если оба параметра `false`, изображение остаётся без изменений.
//!
//! ## Безопасность
//!
//! - Функция использует `unsafe` для:
//!   - чтения C-строки через `CStr::from_ptr`
//!   - доступа к сырому указателю `rgba_data`
//! - Предполагается, что:
//!   - `rgba_data` указывает на валидный буфер размером `width * height * 4` байт.
//!   - `params` — это null-terminated C-строка с валидным UTF-8 и корректным JSON.
//! - Невалидные данные могут привести к панике или неопределённому поведению.
//!
//! ## Алгоритм
//!
//! - **Горизонтальное отражение**: для каждой строки пиксели отражаются относительно вертикальной оси.
//! - **Вертикальное отражение**: строки изображения отражаются относительно горизонтальной оси.
//! - **Оба отражения**: эквивалентно повороту на 180°, но реализовано как комбинированное отражение.
//! - Обработка границ выполняется корректно, включая нечётные размеры изображения.
//!
//! ## Производительность
//!
//! Сложность: `O(n)`, где `n = width * height`. Каждый пиксель участвует в обмене ровно один раз.
//! Используется `swap` для безопасного обмена байтами каналов RGBA.
//!
//! ## Пример использования (на стороне C)
//!
//! ```c
//! uint8_t image[800 * 600 * 4]; // RGBA buffer
//! const char* params = "{\"horizontal\":true,\"vertical\":false}";
//! process_image(800, 600, image, params);
//! ```
//!
//! После выполнения, изображение будет отражено по горизонтали.
//!

#![warn(missing_docs)]

use serde_json;
use std::ffi::CStr;
use std::ffi::{c_char, c_uchar, c_ulong};

/// Основная функция обработки изображения — применяет зеркальное отражение по заданным осям.
///
/// Эта функция экспортируется для вызова из C-кода и является точкой входа плагина.
///
/// # Параметры
///
/// - `width`: ширина изображения в пикселях.
/// - `height`: высота изображения в пикселях.
/// - `rgba_data`: указатель на массив байтов длиной `width * height * 4`, содержащий
///   пиксели в формате RGBA (по 1 байту на канал).
/// - `params`: указатель на C-строку (null-terminated), содержащую JSON с параметрами отражения.
///
/// # Поддерживаемые параметры в JSON
///
/// - `"horizontal"`: `bool` — отражать по горизонтали.
/// - `"vertical"`: `bool` — отражать по вертикали.
///
/// Если параметр отсутствует, используется значение по умолчанию (`false`).
///
/// # Поведение
///
/// - Если `horizontal == true`, строки изображения отражаются слева направо.
/// - Если `vertical == true`, строки изображения переставляются сверху вниз.
/// - Если оба — `true`, выполняется полный разворот (аналог поворота на 180°).
/// - Если оба — `false`, изображение не изменяется.
///
/// # Безопасность
///
/// - Функция содержит `unsafe` блоки:
///   - Чтение строки параметров через `CStr::from_ptr`.
///   - Прямой доступ к памяти через `std::slice::from_raw_parts_mut`.
/// - Вызывающий должен гарантировать:
///   - Корректность указателей.
///   - Валидность JSON.
///   - Размер буфера `rgba_data`.
///
/// # Примеры (в тестах)
///
/// См. модуль `tests` для примеров:
/// - `test_mirror_horizontal`
/// - `test_mirror_vertical`
/// - `test_mirror_both`
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_image(
    width: c_ulong,
    height: c_ulong,
    rgba_data: *mut c_uchar,
    params: *const c_char,
) {
    let params_str = unsafe { CStr::from_ptr(params).to_str().unwrap() };
    let params: serde_json::Value = serde_json::from_str(params_str).unwrap();
    let horizontal = params["horizontal"].as_bool().unwrap_or(false);
    let vertical = params["vertical"].as_bool().unwrap_or(false);

    let data = unsafe { std::slice::from_raw_parts_mut(rgba_data, (width * height * 4) as usize) };

    let width = width as usize;
    let height = height as usize;

    if horizontal && vertical {
        // Оба отражения — полный разворот
        for y in 0..height / 2 {
            for x in 0..width {
                let src_idx = (y * width + x) * 4;
                let dst_idx = ((height - 1 - y) * width + (width - 1 - x)) * 4;
                for c in 0..4 {
                    data.swap(src_idx + c, dst_idx + c);
                }
            }
        }
        // Центральная строка при нечетной высоте
        if height % 2 == 1 {
            let mid_y = height / 2;
            for x in 0..width / 2 {
                let src_idx = (mid_y * width + x) * 4;
                let dst_idx = (mid_y * width + (width - 1 - x)) * 4;
                for c in 0..4 {
                    data.swap(src_idx + c, dst_idx + c);
                }
            }
        }
    } else if horizontal {
        // Отражение по горизонтали
        for y in 0..height {
            for x in 0..width / 2 {
                let src_idx = (y * width + x) * 4;
                let dst_idx = (y * width + (width - 1 - x)) * 4;
                for c in 0..4 {
                    data.swap(src_idx + c, dst_idx + c);
                }
            }
        }
    } else if vertical {
        // Отражение по вертикали
        for y in 0..height / 2 {
            for x in 0..width {
                let src_idx = (y * width + x) * 4;
                let dst_idx = ((height - 1 - y) * width + x) * 4;
                for c in 0..4 {
                    data.swap(src_idx + c, dst_idx + c);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_mirror_horizontal() {
        let mut data = vec![
            255, 0, 0, 255, // Красный
            0, 255, 0, 255, // Зеленый
            0, 0, 255, 255, // Синий
            255, 255, 0, 255, // Желтый
        ];

        let expected = vec![
            0, 255, 0, 255, // Зеленый
            255, 0, 0, 255, // Красный
            255, 255, 0, 255, // Желтый
            0, 0, 255, 255, // Синий
        ];

        let width = 2;
        let height = 2;
        let params = r#"{"horizontal":true,"vertical":false}"#;
        let params = CString::new(params).unwrap();

        let data_ptr = data.as_mut_ptr();

        unsafe {
            process_image(width, height, data_ptr, params.as_ptr());
        }

        assert_eq!(data, expected);
    }

    #[test]
    fn test_mirror_vertical() {
        let mut data = vec![
            255, 0, 0, 255, // Красный
            0, 255, 0, 255, // Зеленый
            0, 0, 255, 255, // Синий
            255, 255, 0, 255, // Желтый
        ];

        let expected = vec![
            0, 0, 255, 255, // Синий
            255, 255, 0, 255, // Желтый
            255, 0, 0, 255, // Красный
            0, 255, 0, 255, // Зеленый
        ];

        let width = 2;
        let height = 2;
        let params = r#"{"horizontal":false,"vertical":true}"#;
        let params = CString::new(params).unwrap();

        let data_ptr = data.as_mut_ptr();

        unsafe {
            process_image(width, height, data_ptr, params.as_ptr());
        }

        assert_eq!(data, expected);
    }

    #[test]
    fn test_mirror_both() {
        let mut data = vec![
            255, 0, 0, 255, // Красный
            0, 255, 0, 255, // Зеленый
            0, 0, 255, 255, // Синий
            255, 255, 0, 255, // Желтый
        ];

        let expected = vec![
            255, 255, 0, 255, // Желтый
            0, 0, 255, 255, // Синий
            0, 255, 0, 255, // Зеленый
            255, 0, 0, 255, // Красный
        ];

        let width = 2;
        let height = 2;
        let params = r#"{"horizontal":true,"vertical":true}"#;
        let params = CString::new(params).unwrap();

        let data_ptr = data.as_mut_ptr();

        unsafe {
            process_image(width, height, data_ptr, params.as_ptr());
        }

        assert_eq!(data, expected);
    }
}
