//! # Blur Plugin
//!
//! Это нативный C-compatible плагин для размытия изображений в формате RGBA.
//! Он предоставляет функцию `process_image`, которая применяет эффект размытия,
//! подобный взвешенному усреднению пикселей в окрестности заданного радиуса.
//!
//! Плагин предназначен для интеграции с внешними системами через FFI (Foreign Function Interface)
//! и принимает параметры в формате JSON.
//!
//! ## Особенности
//!
//! - Поддержка настраиваемого радиуса размытия.
//! - Поддержка нескольких итераций для усиления эффекта.
//! - Работает напрямую с сырыми RGBA-пикселями.
//! - Безопасная обработка границ изображения с использованием `saturating_sub` и `min`.
//!
//! ## Алгоритм
//!
//! Размытие реализовано как взвешенное среднее по соседним пикселям,
//! где вес обратно пропорционален расстоянию (в виде `1/distance`).
//! Это приближение к гауссовому размытию, но без настоящей экспоненциальной функции.
//! Веса масштабируются на `1000.0` для работы с целыми числами.
//!

#![warn(missing_docs)]

use serde_json;
use std::ffi::CStr;
use std::ffi::{c_char, c_uchar, c_ulong, c_int};

const SUCCESS: c_int = 0;
const INVALID_STRING: c_int = -1;
const NULL_PTR: c_int = -2;
const INVALID_JSON: c_int = -3;
const INVALID_PARAMS: c_int = -4;
const TO_BIG_IMAGE: c_int = -5;

/// Основная функция обработки изображения — применяет размытие по заданным параметрам.
///
/// Эта функция экспортируется для вызова из C-кода и является точкой входа плагина.
///
/// # Параметры
///
/// - `width`: ширина изображения в пикселях.
/// - `height`: высота изображения в пикселях.
/// - `rgba_data`: указатель на массив байтов длиной `width * height * 4`, содержащий
///   пиксели в формате RGBA (по 1 байту на канал).
/// - `params`: указатель на C-строку (null-terminated), содержащую JSON с параметрами размытия.
///
/// # Формат параметров (JSON)
///
/// Поддерживаются следующие поля:
/// - `"radius"`: радиус размытия в пикселях (целое число). По умолчанию — `1`.
/// - `"iterations"`: количество проходов размытия (целое число). По умолчанию — `1`.
///
/// Пример:
/// ```json
/// {"radius": 2, "iterations": 3}
/// ```
///
/// # Безопасность
///
/// - Функция использует `unsafe` для:
///   - чтения C-строки через `CStr::from_ptr`
///   - доступа к сырым буферам пикселей
/// - Вызывающий должен гарантировать:
///   - C-строка параметров - это null-терминированная строка.
///   - Размер буфера `rgba_data` должен быть width * height * 4.
///
/// # Пример использования (на стороне C)
///
/// ```c
/// uint8_t image[400 * 300 * 4]; // RGBA buffer
/// const char* params = "{\"radius\": 2, \"iterations\": 2}";
/// process_image(400, 300, image, params);
/// ```
///
/// # Примечания
///
/// - Чем больше `iterations`, тем сильнее и равномернее размытие.
/// - Большой радиус может значительно замедлить обработку из-за `O(n²)` сложности на пиксель.
/// - Прозрачность (альфа-канал) также участвует в размытии.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_image(
    width: c_ulong,
    height: c_ulong,
    rgba_data: *mut c_uchar,
    params: *const c_char,
) -> c_int{
    let width = width as usize;
    let height = height as usize;

    if rgba_data.is_null() || params.is_null() {
        return NULL_PTR;
    }

    if width == 0 || height == 0 {
        return INVALID_PARAMS;
    }

    let params_str = unsafe {
        match CStr::from_ptr(params).to_str() {
            Ok(val) => val,
            Err(_) => {
                return INVALID_STRING;
            }
        }
    };

    let params: serde_json::Value = match serde_json::from_str(params_str){
        Ok(val) => val,
        Err(_) => {
            return INVALID_JSON;
        }
    };

    let radius = params["radius"].as_u64().unwrap_or(1) as usize;
    let iterations = params["iterations"].as_u64().unwrap_or(1) as usize;

    // Проверка на переполнение для 32-битных систем
    let Some(len) = width.checked_mul(height).and_then(|res| res.checked_mul(4)) else {
        return TO_BIG_IMAGE;
    };

    let data = unsafe { std::slice::from_raw_parts_mut(rgba_data, len) };    

    for _ in 0..iterations {
        for y in 0..height {
            for x in 0..width {
                let mut r = 0u32;
                let mut g = 0u32;
                let mut b = 0u32;
                let mut a = 0u32;
                let mut total_weight = 0u32;

                for dy in y.saturating_sub(radius)..std::cmp::min(height, y + radius + 1) {
                    for dx in x.saturating_sub(radius)..std::cmp::min(width, x + radius + 1) {
                        let distance =
                            ((dx as i32 - x as i32).pow(2) + (dy as i32 - y as i32).pow(2)) as f32;
                        let weight = if distance == 0.0 { 1.0 } else { 1.0 / distance };
                        let weight_u32 = (weight * 1000.0) as u32; // Масштабируем для использования целых чисел

                        let idx = (dy * width + dx) * 4;
                        r += (data[idx] as u32) * weight_u32;
                        g += (data[idx + 1] as u32) * weight_u32;
                        b += (data[idx + 2] as u32) * weight_u32;
                        a += (data[idx + 3] as u32) * weight_u32;
                        total_weight += weight_u32;
                    }
                }

                let idx = (y * width + x) * 4;
                if total_weight > 0 {
                    data[idx] = (r / total_weight) as u8;
                    data[idx + 1] = (g / total_weight) as u8;
                    data[idx + 2] = (b / total_weight) as u8;
                    data[idx + 3] = (a / total_weight) as u8;
                }
            }
        }
    }

    SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_blur_simple() {
        // Создаем изображение 3x3 с центральным белым пикселем
        let mut data = vec![
            255, 0, 0, 255, // Красный
            0, 255, 0, 255, // Зеленый
            0, 0, 255, 255, // Синий
            255, 255, 0, 255, // Желтый
            255, 255, 255, 255, // Белый (центр)
            255, 0, 255, 255, // Пурпурный
            0, 255, 255, 255, // Голубой
            128, 128, 128, 255, // Серый
            255, 255, 255, 255, // Белый
        ];
        let width = 3;
        let height = 3;
        let params = r#"{"radius":1,"iterations":1}"#;
        let params = CString::new(params).unwrap();

        let data_ptr = data.as_mut_ptr();

        unsafe {
            process_image(width, height, data_ptr, params.as_ptr());
        }

        // Центральный пиксель должен стать средним цветом окружающих
        let center_idx = (1 * 3 + 1) * 4; // [1][1]
        // Окружение: не все пиксели равны, центр должен стать не белым
        assert!(data[center_idx] < 255 || data[center_idx + 1] < 255 || data[center_idx + 2] < 255);
    }

    #[test]
    fn test_blur_radius_and_iterations() {
        // Простое изображение 2x2
        let mut data = vec![
            255, 0, 0, 255, // Красный
            0, 255, 0, 255, // Зеленый
            0, 0, 255, 255, // Синий
            255, 255, 0, 255, // Желтый
        ];
        let width = 2;
        let height = 2;
        let params = r#"{"radius":1, "iterations":2}"#;
        let params = CString::new(params).unwrap();

        let data_ptr = data.as_mut_ptr();

        unsafe {
            process_image(width, height, data_ptr, params.as_ptr());
        }
        // При двух итерациях размытие должно быть сильнее
        // Все пиксели должны стать более однородными
        for i in 0..4 {
            let idx = i * 4;
            // Проверим, что значения не являются чистыми первоначальными цветами
            assert!(data[idx] != 255 || data[idx + 1] != 0 || data[idx + 2] != 0); // Не чисто красный
            assert!(data[idx] != 0 || data[idx + 1] != 255 || data[idx + 2] != 0); // Не чисто зеленый
            assert!(data[idx] != 0 || data[idx + 1] != 0 || data[idx + 2] != 255); // Не чисто синий
            assert!(data[idx] != 255 || data[idx + 1] != 255 || data[idx + 2] != 0); // Не чисто желтый
        }
    }
}
