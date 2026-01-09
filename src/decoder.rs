use encoding_rs::*;
use half::f16;
use pyo3::prelude::*;
use pyo3::types::PyDict;

// Helper function to extract required fields from a Python dict
fn get_required_field<T>(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<T>
where
    T: for<'a, 'py> FromPyObject<'a, 'py, Error = PyErr>,
{
    dict.get_item(key)?
        .ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!(
                "Recipe missing required field: '{}'",
                key
            ))
        })?
        .extract()
}

// Helper function to extract optional fields from a Python dict
fn get_optional_field<T>(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<T>>
where
    T: for<'a, 'py> FromPyObject<'a, 'py, Error = PyErr>,
{
    dict.get_item(key)?.map(|v| v.extract()).transpose()
}

// Apply byte order to bytes
fn apply_byte_order(bytes_in: &[u8], byte_order: &str) -> PyResult<Vec<u8>> {
    match byte_order.to_lowercase().as_str() {
        "big" => Ok(bytes_in.to_vec()),
        "little" => {
            let mut reversed = bytes_in.to_vec();
            reversed.reverse();
            Ok(reversed)
        }
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Invalid byte_order '{}': must be 'big' or 'little'",
            byte_order
        ))),
    }
}

#[pyfunction]
pub fn process_recipe(
    py: Python,
    packet_bytes: &[u8],
    offset_bits: usize,
    recipe: Vec<Py<PyAny>>,
) -> PyResult<Py<PyDict>> {
    let results: Bound<'_, PyDict> = PyDict::new(py);

    let mut current_offset_bits = offset_bits;

    for item_obj in recipe {
        let recipe_item: &Bound<'_, PyDict> = item_obj.cast_bound::<PyDict>(py)?;

        // Extract values
        let param_name: String = get_required_field(recipe_item, "name")?;
        let param_encoding: String = get_required_field(recipe_item, "encoding")?;
        let param_bits: u32 = get_required_field(recipe_item, "bits")?;
        let byte_order: String = get_required_field(recipe_item, "byte_order")?;
        let reverse_bits: bool = get_required_field(recipe_item, "reverse_bits")?;

        // Check if this parameter has an explicit offset
        let param_offset_bits: Option<usize> = get_optional_field(recipe_item, "offset_bits")?;

        // Determine the actual offset for this parameter
        let actual_offset_bits = if let Some(relative_offset) = param_offset_bits {
            // Use explicit offset
            offset_bits + relative_offset
        } else {
            // Use sequential offset
            current_offset_bits
        };

        // Extract bytes for this parameter
        let byte_start = actual_offset_bits / 8;
        let byte_count = (param_bits as usize + 7) / 8;
        let source_bytes = &packet_bytes[byte_start..(byte_start + byte_count)];

        // Apply byte order
        let ordered_bytes = apply_byte_order(source_bytes, &byte_order)?;

        // Update offset for next sequential parameter
        current_offset_bits = actual_offset_bits + param_bits as usize;

        // Convert bytes to raw integer value
        let mut raw_value: u64 = 0;
        for (i, &byte) in ordered_bytes.iter().enumerate() {
            if i < 8 {
                // Limit to 64 bits
                raw_value = (raw_value << 8) | (byte as u64);
            }
        }

        // Apply bit reversal if needed
        let final_value: u64 = if reverse_bits {
            // Reverse the bits within the specified bit width
            let mut reversed: u64 = 0u64;
            for bit_pos in 0..param_bits {
                if (raw_value >> bit_pos) & 1 == 1 {
                    reversed |= 1u64 << (param_bits - 1 - bit_pos);
                }
            }
            reversed
        } else {
            // Mask to the specified bit width
            if param_bits >= 64 {
                raw_value
            } else {
                raw_value & ((1u64 << param_bits) - 1)
            }
        };

        // Decode based on encoding type and store as appropriate Python value
        match param_encoding.to_lowercase().as_str() {
            // Integer encodings
            "unsigned" => {
                results.set_item(&param_name, final_value)?;
            }
            "ones_complement" => {
                let signed_value = decode_ones_complement(final_value, param_bits)?;
                results.set_item(&param_name, signed_value)?;
            }
            "sign_magnitude" => {
                let signed_value = decode_sign_magnitude(final_value, param_bits)?;
                results.set_item(&param_name, signed_value)?;
            }
            "twos_complement" => {
                let signed_value = decode_twos_complement(final_value, param_bits)?;
                results.set_item(&param_name, signed_value)?;
            }

            // BCD encodings
            "unpacked_bcd" => {
                let value = decode_unpacked_bcd(final_value, param_bits, false)?;
                results.set_item(&param_name, value)?;
            }
            "unpacked_bcd_signed" => {
                let value = decode_unpacked_bcd(final_value, param_bits, true)?;
                results.set_item(&param_name, value)?;
            }
            "packed_bcd" => {
                let value = decode_packed_bcd(final_value, param_bits, false)?;
                results.set_item(&param_name, value)?;
            }
            "packed_bcd_signed" => {
                let value = decode_packed_bcd(final_value, param_bits, true)?;
                results.set_item(&param_name, value)?;
            }

            // IEEE 754 floating point encodings
            "ieee754_f16" => {
                let float_value = decode_ieee754_f16(final_value)?;
                results.set_item(&param_name, float_value)?;
            }
            "ieee754_f32" => {
                let float_value = decode_ieee754_f32(final_value)?;
                results.set_item(&param_name, float_value)?;
            }
            "ieee754_f64" => {
                let float_value = decode_ieee754_f64(final_value)?;
                results.set_item(&param_name, float_value)?;
            }
            "ieee754_f128" => {
                // Need to handle 128-bit values specially
                if param_bits > 64 {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "IEEE 754 f128 requires special handling for >64 bits (not yet implemented in process_recipe)"
                    ));
                }
                let float_value = decode_ieee754_f128(final_value as u128)?;
                results.set_item(&param_name, float_value)?;
            }

            // IEEE 754-1985 encodings
            "ieee754_1985_f32" => {
                let float_value = decode_ieee754_1985_f32(final_value)?;
                results.set_item(&param_name, float_value)?;
            }
            "ieee754_1985_f64" => {
                let float_value = decode_ieee754_1985_f64(final_value)?;
                results.set_item(&param_name, float_value)?;
            }
            "ieee754_1985_f80" => {
                let float_value = decode_ieee754_1985_f80(final_value)?;
                results.set_item(&param_name, float_value)?;
            }

            // MIL-STD-1750A encodings
            "mil_std_1750a_f32" => {
                let float_value = decode_mil_std_1750a_f32(final_value)?;
                results.set_item(&param_name, float_value)?;
            }
            "mil_std_1750a_f48" => {
                let float_value = decode_mil_std_1750a_f48(final_value)?;
                results.set_item(&param_name, float_value)?;
            }

            // DEC encodings
            "dec_f32" => {
                let float_value = decode_dec_f32(final_value)?;
                results.set_item(&param_name, float_value)?;
            }
            "dec_f64" => {
                let float_value = decode_dec_f64(final_value)?;
                results.set_item(&param_name, float_value)?;
            }

            // IBM encodings
            "ibm_f32" => {
                let float_value = decode_ibm_f32(final_value)?;
                results.set_item(&param_name, float_value)?;
            }
            "ibm_f64" => {
                let float_value = decode_ibm_f64(final_value)?;
                results.set_item(&param_name, float_value)?;
            }

            // Texas Instruments encodings
            "ti_f32" => {
                let float_value = decode_ti_f32(final_value)?;
                results.set_item(&param_name, float_value)?;
            }
            "ti_f40" => {
                let float_value = decode_ti_f40(final_value)?;
                results.set_item(&param_name, float_value)?;
            }

            // String encodings
            "us_ascii" => {
                let string_value = decode_us_ascii(&ordered_bytes)?;
                results.set_item(&param_name, string_value)?;
            }
            "iso_8859_1" | "latin1" => {
                let string_value = decode_iso_8859_1(&ordered_bytes)?;
                results.set_item(&param_name, string_value)?;
            }
            "windows_1252" => {
                let string_value = decode_windows_1252(&ordered_bytes)?;
                results.set_item(&param_name, string_value)?;
            }
            "utf8" => {
                let string_value = decode_utf8(&ordered_bytes)?;
                results.set_item(&param_name, string_value)?;
            }
            "utf16" => {
                let string_value = decode_utf16(&ordered_bytes)?;
                results.set_item(&param_name, string_value)?;
            }
            "utf16le" => {
                let string_value = decode_utf16le(&ordered_bytes)?;
                results.set_item(&param_name, string_value)?;
            }
            "utf16be" => {
                let string_value = decode_utf16be(&ordered_bytes)?;
                results.set_item(&param_name, string_value)?;
            }
            "utf32" => {
                let string_value = decode_utf32(&ordered_bytes)?;
                results.set_item(&param_name, string_value)?;
            }
            "utf32le" => {
                let string_value = decode_utf32le(&ordered_bytes)?;
                results.set_item(&param_name, string_value)?;
            }
            "utf32be" => {
                let string_value = decode_utf32be(&ordered_bytes)?;
                results.set_item(&param_name, string_value)?;
            }

            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unsupported encoding type: {}",
                    param_encoding
                )));
            }
        }
    }

    Ok(results.into())
}

#[pyfunction]
pub fn decode_ones_complement(value: u64, bits: u32) -> PyResult<i64> {
    // Handle edge case for 64-bit values to avoid overflow
    if bits >= 64 {
        return Ok(if (value >> 63) & 1 == 1 {
            // All bits set means negative zero in ones' complement
            if value == u64::MAX {
                0
            } else {
                -(!value as i64)
            }
        } else {
            value as i64
        });
    }

    // Check if the sign bit is set
    if (value >> (bits - 1)) & 1_u64 == 1_u64 {
        // If the sign bit is 1, the number is negative
        let mask: u64 = (1_u64 << bits) - 1_u64;
        let magnitude: u64 = mask - value;

        // Return the negative of the magnitude
        Ok(-(magnitude as i64))
    } else {
        // If the sign bit is 0, it's a positive number
        Ok(value as i64)
    }
}

#[pyfunction]
pub fn decode_twos_complement(value: u64, bits: u32) -> PyResult<i64> {
    // Handle edge case for 64-bit values to avoid overflow
    if bits >= 64 {
        return Ok(value as i64); // Direct cast for 64-bit two's complement
    }

    // Check if the sign bit is set
    if (value >> (bits - 1)) & 1_u64 == 1_u64 {
        // If the sign bit is 1, the number is negative
        let mask: u64 = (1_u64 << bits) - 1_u64;
        let magnitude: u64 = (!value + 1_u64) & mask;

        // Return the negative of the magnitude
        Ok(-(magnitude as i64))
    } else {
        // If the sign bit is 0, it's a positive number
        Ok(value as i64)
    }
}

#[pyfunction]
pub fn decode_sign_magnitude(value: u64, bits: u32) -> PyResult<i64> {
    // Extract the sign bit and magnitude
    let sign_bit: u64 = (value >> (bits - 1u32)) & 1_u64;
    let magnitude: u64 = value & ((1_u64 << (bits - 1u32)) - 1_u64);

    // Return the signed value based on the sign bit
    if sign_bit == 1_u64 {
        Ok(-(magnitude as i64))
    } else {
        Ok(magnitude as i64)
    }
}

#[pyfunction]
#[pyo3(signature = (value, bits, signed = false))]
pub fn decode_unpacked_bcd(value: u64, bits: u32, signed: bool) -> PyResult<i64> {
    let num_digits: usize = (bits / 8) as usize;
    if num_digits == 0 {
        return Ok(0);
    }

    let mut digits: Vec<u8> = Vec::with_capacity(num_digits);
    let mut raw_value: u64 = value;
    let mut is_negative = false;

    // Handle signed BCD with zoned decimal format if requested
    if signed {
        // For zoned decimal, each byte represents one decimal digit in EBCDIC/ASCII format
        // The rightmost byte contains both sign (upper nibble) and digit (lower nibble)
        for i in 0..num_digits {
            let byte_val = (raw_value & 0xFF) as u8;

            if i == 0 {
                // First (rightmost) byte contains both sign and digit in zoned decimal format
                let (digit, sign_positive) = match byte_val {
                    // EBCDIC positive signed digits (0xC0-0xC9)
                    0xC0..=0xC9 => (byte_val - 0xC0, true),
                    // EBCDIC negative signed digits (0xD0-0xD9)  
                    0xD0..=0xD9 => (byte_val - 0xD0, false),
                    // EBCDIC alternate positive (0xF0-0xF9) treated as positive
                    0xF0..=0xF9 => (byte_val - 0xF0, true),
                    // ASCII-style adaptations
                    0x30..=0x39 => (byte_val - 0x30, true), // Normal ASCII digits treated as positive
                    _ => {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "Invalid zoned decimal signed byte: 0x{:02X} (expected 0x30-0x39, 0xC0-0xC9, 0xD0-0xD9, or 0xF0-0xF9)",
                            byte_val
                        )))
                    }
                };

                is_negative = !sign_positive;
                digits.push(digit);
            } else {
                // Other bytes should be standard zoned digits
                let digit = match byte_val {
                    0x30..=0x39 => byte_val - 0x30, // ASCII digits
                    0xF0..=0xF9 => byte_val - 0xF0, // EBCDIC digits  
                    _ => {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "Invalid zoned decimal byte: 0x{:02X} (expected ASCII 0x30-0x39 or EBCDIC 0xF0-0xF9)",
                            byte_val
                        )))
                    }
                };
                digits.push(digit);
            }

            raw_value >>= 8;
        }

        // Convert to decimal using Horner's method
        let mut result: u64 = 0;
        for &digit in digits.iter().rev() {
            result = result.saturating_mul(10).saturating_add(digit as u64);
        }

        return Ok(if is_negative {
            -(result as i64)
        } else {
            result as i64
        });
    }

    // Extract and validate all digits for unsigned BCD
    for _ in 0..num_digits {
        let digit = (raw_value & 0xFF) as u8;
        if digit > 9 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid BCD digit: {} (must be 0-9)",
                digit
            )));
        }
        digits.push(digit);
        raw_value >>= 8;
    }

    // Convert to decimal using Horner's method
    let mut result: u64 = 0;
    for &digit in digits.iter().rev() {
        result = result.saturating_mul(10).saturating_add(digit as u64);
    }

    Ok(result as i64)
}

#[pyfunction]
#[pyo3(signature = (value, bits, signed = false))]
pub fn decode_packed_bcd(value: u64, bits: u32, signed: bool) -> PyResult<i64> {
    let num_digits = (bits / 4) as usize;
    if num_digits == 0 {
        return Ok(0);
    }

    // Handle signed BCD if requested
    if signed {
        // For packed BCD, the last nibble (rightmost) contains the sign
        let sign_nibble = (value & 0x0F) as u8;

        let is_negative = match sign_nibble {
            0xD | 0xB => true,              // Negative signs
            0xC | 0xF | 0xA | 0xE => false, // Positive signs
            0x0..=0x9 => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Invalid BCD sign nibble: expected 0xA-0xF for signed packed BCD",
                ))
            }
            _ => unreachable!(), // nibble can only be 0x0-0xF
        };

        // Extract digits
        let mut digits: Vec<u8> = Vec::with_capacity(num_digits - 1);
        let mut raw_value = value >> 4; // Skip the sign nibble

        for _ in 0..(num_digits - 1) {
            let digit = (raw_value & 0x0F) as u8;
            if digit > 9 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid BCD digit: {} (must be 0-9)",
                    digit
                )));
            }
            digits.push(digit);
            raw_value >>= 4;
        }

        let mut result: u64 = 0;
        for &digit in digits.iter().rev() {
            result = result.saturating_mul(10).saturating_add(digit as u64);
        }

        return Ok(if is_negative {
            -(result as i64)
        } else {
            result as i64
        });
    }

    // Handle unsigned BCD
    let mut digits: Vec<u8> = Vec::with_capacity(num_digits);
    let mut raw_value: u64 = value;

    // Extract and validate all digits
    for _ in 0..num_digits {
        let digit: u8 = (raw_value & 0x0F) as u8;
        if digit > 9 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid BCD digit: {} (must be 0-9)",
                digit
            )));
        }
        digits.push(digit);
        raw_value >>= 4;
    }

    // Convert to decimal using Horner's method
    let mut result: u64 = 0;
    for &digit in digits.iter().rev() {
        result = result.saturating_mul(10).saturating_add(digit as u64);
    }

    Ok(result as i64)
}

#[pyfunction]
pub fn decode_ieee754_f16(value: u64) -> PyResult<f32> {
    // Convert the 16-bit value to f16 and then to f32 for Python compatibility
    let f16_bits = value as u16;
    let f16_value = f16::from_bits(f16_bits);
    Ok(f16_value.to_f32())
}

#[pyfunction]
pub fn decode_ieee754_f32(value: u64) -> PyResult<f32> {
    let float_val = f32::from_bits(value as u32);
    Ok(float_val)
}

#[pyfunction]
pub fn decode_ieee754_f64(value: u64) -> PyResult<f64> {
    let float_val = f64::from_bits(value);
    Ok(float_val)
}

#[pyfunction]
pub fn decode_ieee754_f128(value: u128) -> PyResult<f64> {
    // Extract IEEE 754 f128 components
    let sign = (value >> 127) & 1;
    let exponent = ((value >> 112) & 0x7FFF) as u16;
    let mantissa = value & ((1u128 << 112) - 1);

    // Handle special cases first
    match exponent {
        0x7FFF => {
            // Infinity or NaN
            return Ok(if mantissa == 0 {
                if sign == 1 {
                    f64::NEG_INFINITY
                } else {
                    f64::INFINITY
                }
            } else {
                f64::NAN
            });
        }
        0 if mantissa == 0 => {
            // Zero
            return Ok(if sign == 1 { -0.0 } else { 0.0 });
        }
        _ => {}
    }

    // Convert to f64
    let result = if exponent == 0 {
        // Subnormal number
        let mantissa_f64 = mantissa as f64;
        let scale = 2f64.powi(-16382 - 112);
        mantissa_f64 * scale
    } else {
        // Normalized number
        let actual_exponent = (exponent as i32) - 16383;

        // Fast range check and clamp
        if actual_exponent > 1023 {
            f64::INFINITY
        } else if actual_exponent < -1022 {
            0.0
        } else {
            // Mantissa conversion, shift to get top 52 bits
            let mantissa_bits = (mantissa >> 60) as u64;
            let mantissa_f64 = mantissa_bits as f64 * (1.0 / (1u64 << 52) as f64);
            let mantissa_value = 1.0 + mantissa_f64;
            mantissa_value * 2f64.powi(actual_exponent)
        }
    };

    Ok(if sign == 1 { -result } else { result })
}

#[pyfunction]
pub fn decode_ieee754_1985_f32(_value: u64) -> PyResult<f32> {
    // TODO
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "IEEE 754-1985 f32 format not yet implemented",
    ))
}

#[pyfunction]
pub fn decode_ieee754_1985_f64(_value: u64) -> PyResult<f64> {
    // TODO
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "IEEE 754-1985 f64 format not yet implemented",
    ))
}

#[pyfunction]
pub fn decode_ieee754_1985_f80(_value: u64) -> PyResult<f64> {
    // TODO
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "IEEE 754-1985 f80 format not yet implemented",
    ))
}

#[pyfunction]
pub fn decode_mil_std_1750a_f32(_value: u64) -> PyResult<f32> {
    // TODO
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "MIL-STD-1750A f32 format not yet implemented",
    ))
}

#[pyfunction]
pub fn decode_mil_std_1750a_f48(_value: u64) -> PyResult<f64> {
    // TODO
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "MIL-STD-1750A f48 format not yet implemented",
    ))
}

#[pyfunction]
pub fn decode_dec_f32(_value: u64) -> PyResult<f32> {
    // TODO
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "DEC f32 format not yet implemented",
    ))
}

#[pyfunction]
pub fn decode_dec_f64(_value: u64) -> PyResult<f64> {
    // TODO
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "DEC f64 format not yet implemented",
    ))
}

#[pyfunction]
pub fn decode_ibm_f32(_value: u64) -> PyResult<f32> {
    // TODO
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "IBM f32 format not yet implemented",
    ))
}

#[pyfunction]
pub fn decode_ibm_f64(_value: u64) -> PyResult<f64> {
    // TODO
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "IBM f64 format not yet implemented",
    ))
}

#[pyfunction]
pub fn decode_ti_f32(_value: u64) -> PyResult<f32> {
    // TODO
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "TI f32 format not yet implemented",
    ))
}

#[pyfunction]
pub fn decode_ti_f40(_value: u64) -> PyResult<f64> {
    // TODO
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "TI f40 format not yet implemented",
    ))
}

#[pyfunction]
pub fn decode_us_ascii(bytes: &[u8]) -> PyResult<String> {
    // US-ASCII is valid if all bytes are <= 127
    for &byte in bytes {
        if byte > 127 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid US-ASCII byte: 0x{:02X} (must be <= 0x7F)",
                byte
            )));
        }
    }

    Ok(String::from_utf8_lossy(bytes).into_owned())
}

#[pyfunction]
pub fn decode_iso_8859_1(bytes: &[u8]) -> PyResult<String> {
    // ISO-8859-1 is a direct mapping from bytes to Unicode codepoints 0-255
    let result: String = bytes.iter().map(|&b| b as char).collect();
    Ok(result)
}

#[pyfunction]
pub fn decode_windows_1252(bytes: &[u8]) -> PyResult<String> {
    let (cow, _encoding_used, had_errors) = WINDOWS_1252.decode(bytes);
    if had_errors {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Failed to decode Windows-1252 bytes",
        ));
    }
    Ok(cow.into_owned())
}

#[pyfunction]
pub fn decode_utf8(bytes: &[u8]) -> PyResult<String> {
    match std::str::from_utf8(bytes) {
        Ok(s) => Ok(s.to_string()),
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Invalid UTF-8 bytes: {}",
            e
        ))),
    }
}

#[pyfunction]
pub fn decode_utf16(bytes: &[u8]) -> PyResult<String> {
    if bytes.len() < 2 || bytes.len() % 2 != 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "UTF-16 requires an even number of bytes with at least 2 bytes",
        ));
    }

    // Check for BOM to determine endianness
    let (u16_bytes, little_endian) = if bytes.len() >= 2 {
        if bytes[0] == 0xFF && bytes[1] == 0xFE {
            // Little-endian BOM, skip it
            (&bytes[2..], true)
        } else if bytes[0] == 0xFE && bytes[1] == 0xFF {
            // Big-endian BOM, skip it
            (&bytes[2..], false)
        } else {
            // No BOM, assume big-endian
            (bytes, false)
        }
    } else {
        (bytes, false)
    };

    decode_utf16_with_endianness(u16_bytes, little_endian)
}

#[pyfunction]
pub fn decode_utf16le(bytes: &[u8]) -> PyResult<String> {
    decode_utf16_with_endianness(bytes, true)
}

#[pyfunction]
pub fn decode_utf16be(bytes: &[u8]) -> PyResult<String> {
    decode_utf16_with_endianness(bytes, false)
}

fn decode_utf16_with_endianness(bytes: &[u8], little_endian: bool) -> PyResult<String> {
    if bytes.len() % 2 != 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "UTF-16 requires an even number of bytes",
        ));
    }

    let u16_vec: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| {
            if little_endian {
                u16::from_le_bytes([chunk[0], chunk[1]])
            } else {
                u16::from_be_bytes([chunk[0], chunk[1]])
            }
        })
        .collect();

    match String::from_utf16(&u16_vec) {
        Ok(s) => Ok(s),
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Invalid UTF-16 sequence: {}",
            e
        ))),
    }
}

#[pyfunction]
pub fn decode_utf32(bytes: &[u8]) -> PyResult<String> {
    if bytes.len() < 4 || bytes.len() % 4 != 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "UTF-32 requires a multiple of 4 bytes with at least 4 bytes",
        ));
    }

    // Check for BOM to determine endianness
    let (u32_bytes, little_endian) = if bytes.len() >= 4 {
        if bytes[0] == 0xFF && bytes[1] == 0xFE && bytes[2] == 0x00 && bytes[3] == 0x00 {
            // Little-endian BOM, skip it
            (&bytes[4..], true)
        } else if bytes[0] == 0x00 && bytes[1] == 0x00 && bytes[2] == 0xFE && bytes[3] == 0xFF {
            // Big-endian BOM, skip it
            (&bytes[4..], false)
        } else {
            // No BOM, assume big-endian
            (bytes, false)
        }
    } else {
        (bytes, false)
    };

    decode_utf32_with_endianness(u32_bytes, little_endian)
}

#[pyfunction]
pub fn decode_utf32le(bytes: &[u8]) -> PyResult<String> {
    decode_utf32_with_endianness(bytes, true)
}

#[pyfunction]
pub fn decode_utf32be(bytes: &[u8]) -> PyResult<String> {
    decode_utf32_with_endianness(bytes, false)
}

fn decode_utf32_with_endianness(bytes: &[u8], little_endian: bool) -> PyResult<String> {
    if bytes.len() % 4 != 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "UTF-32 requires a multiple of 4 bytes",
        ));
    }

    let u32_vec: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|chunk| {
            if little_endian {
                u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
            } else {
                u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
            }
        })
        .collect();

    // Convert u32 codepoints to String
    let mut result = String::new();
    for codepoint in u32_vec {
        match std::char::from_u32(codepoint) {
            Some(c) => result.push(c),
            None => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Invalid UTF-32 codepoint: 0x{:08X}",
                    codepoint
                )));
            }
        }
    }
    Ok(result)
}
