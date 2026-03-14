use encoding_rs::*;
use half::f16;
use pyo3::prelude::*;
use pyo3::types::PyDict;

// Helper function to extract required fields from Python dict
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

// Helper function to extract optional fields from Python dict
fn get_optional_field<T>(dict: &Bound<'_, PyDict>, key: &str) -> PyResult<Option<T>>
where
    T: for<'a, 'py> FromPyObject<'a, 'py, Error = PyErr>,
{
    dict.get_item(key)?.map(|v| v.extract()).transpose()
}

// Apply byte order to bytes
fn apply_byte_order_encode(bytes_in: &[u8], byte_order: &str) -> PyResult<Vec<u8>> {
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

// Helper function to convert an integer value to bytes in big endian order
fn encode_integer_to_bytes(value: u64, bits: u32, reverse_bits: bool) -> PyResult<Vec<u8>> {
    if bits > 64 {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Cannot encode {} bits from a 64-bit value",
            bits
        )));
    }

    let byte_count = ((bits + 7) / 8) as usize;

    let final_value = if reverse_bits {
        // Reverse the bits within the specified bit width
        let mut reversed = 0u64;
        for bit_pos in 0..bits {
            if (value >> bit_pos) & 1 == 1 {
                reversed |= 1u64 << (bits - 1 - bit_pos);
            }
        }
        reversed
    } else {
        value
    };

    // Convert to bytes in big endian order
    let mut bytes = Vec::with_capacity(byte_count);
    for i in (0..byte_count).rev() {
        bytes.push(((final_value >> (i * 8)) & 0xFF) as u8);
    }

    Ok(bytes)
}

// Helper function to convert up to a 128-bit integer value to bytes in big endian order
fn encode_integer_to_bytes_u128(value: u128, bits: u32, reverse_bits: bool) -> PyResult<Vec<u8>> {
    if bits > 128 {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Cannot encode {} bits from a 128-bit value",
            bits
        )));
    }

    let byte_count = ((bits + 7) / 8) as usize;

    let final_value = if reverse_bits {
        // Reverse the bits within the specified bit width
        let mut reversed = 0u128;
        for bit_pos in 0..bits {
            if (value >> bit_pos) & 1 == 1 {
                reversed |= 1u128 << (bits - 1 - bit_pos);
            }
        }
        reversed
    } else {
        value
    };

    // Convert to bytes in big endian order
    let mut bytes = Vec::with_capacity(byte_count);
    for i in (0..byte_count).rev() {
        bytes.push(((final_value >> (i * 8)) & 0xFF) as u8);
    }

    Ok(bytes)
}

// Helper function to normalize raw bytes to a field width and optionally reverse bit order
fn encode_binary_to_bytes(value: &[u8], bits: u32, reverse_bits: bool) -> PyResult<Vec<u8>> {
    let byte_count = ((bits + 7) / 8) as usize;
    if byte_count == 0 {
        return Ok(Vec::new());
    }

    // Keep the rightmost bytes and left-pad with zeros when the input is too short
    let mut masked = vec![0u8; byte_count];
    let src_start = value.len().saturating_sub(byte_count);
    let src = &value[src_start..];
    let dst_start = byte_count - src.len();
    masked[dst_start..].copy_from_slice(src);

    // Mask out bits outside the declared field width
    let unused_top_bits = (byte_count * 8) - bits as usize;
    if unused_top_bits > 0 {
        masked[0] &= 0xFF >> unused_top_bits;
    }

    if !reverse_bits {
        return Ok(masked);
    }

    let total_bits = byte_count * 8;
    let mut reversed = vec![0u8; byte_count];
    for src_lsb_idx in 0..bits as usize {
        let src_abs = total_bits - 1 - src_lsb_idx;
        let src_byte = src_abs / 8;
        let src_bit = 7 - (src_abs % 8);
        let bit = (masked[src_byte] >> src_bit) & 1;
        if bit == 1 {
            let dst_lsb_idx = bits as usize - 1 - src_lsb_idx;
            let dst_abs = total_bits - 1 - dst_lsb_idx;
            let dst_byte = dst_abs / 8;
            let dst_bit = 7 - (dst_abs % 8);
            reversed[dst_byte] |= 1 << dst_bit;
        }
    }

    Ok(reversed)
}

#[pyfunction]
pub fn build_packet(py: Python, recipe: Vec<Py<PyAny>>) -> PyResult<Vec<u8>> {
    let mut max_bit_offset = 0usize;
    for item_obj in &recipe {
        let recipe_item: &Bound<'_, PyDict> = item_obj.cast_bound::<PyDict>(py)?;
        let param_bits: u32 = get_required_field(recipe_item, "bits")?;
        let param_offset_bits: Option<usize> = get_optional_field(recipe_item, "offset_bits")?;

        let offset = param_offset_bits.unwrap_or(max_bit_offset);
        let end_offset = offset + param_bits as usize;
        if end_offset > max_bit_offset {
            max_bit_offset = end_offset;
        }
    }

    let packet_size = (max_bit_offset + 7) / 8;
    let mut packet_bytes = vec![0u8; packet_size];
    let mut current_offset_bits = 0usize;

    for item_obj in recipe {
        let recipe_item: &Bound<'_, PyDict> = item_obj.cast_bound::<PyDict>(py)?;

        // Extract recipe fields
        let param_name: String = get_required_field(recipe_item, "name")?;
        let param_encoding: String = get_required_field(recipe_item, "encoding")?;
        let param_bits: u32 = get_required_field(recipe_item, "bits")?;
        let byte_order: String = get_required_field(recipe_item, "byte_order")?;
        let reverse_bits: bool = get_required_field(recipe_item, "reverse_bits")?;
        let param_offset_bits: Option<usize> = get_optional_field(recipe_item, "offset_bits")?;

        // Get the value from the recipe item dict
        let value = recipe_item.get_item("value")?.ok_or_else(|| {
            pyo3::exceptions::PyKeyError::new_err(format!(
                "Recipe item '{}' missing required field: 'value'",
                param_name
            ))
        })?;

        // Determine the actual offset for this parameter
        let actual_offset_bits = param_offset_bits.unwrap_or(current_offset_bits);
        current_offset_bits = actual_offset_bits + param_bits as usize;

        // Encode the value based on encoding type
        let encoded_bytes = match param_encoding.to_lowercase().as_str() {
            // Integer encodings
            "unsigned" => {
                let int_value: u64 = value.extract()?;
                let max_value = if param_bits >= 64 {
                    u64::MAX
                } else {
                    (1u64 << param_bits) - 1
                };
                if int_value > max_value {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Value {} is too large for {} bits",
                        int_value, param_bits
                    )));
                }
                encode_integer_to_bytes(int_value, param_bits, reverse_bits)?
            }
            "ones_complement" | "ones-complement" | "onescomplement" => {
                let int_value: i64 = value.extract()?;
                let encoded = encode_ones_complement(int_value, param_bits)?;
                encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
            }
            "sign_magnitude" | "sign-magnitude" | "signmagnitude" => {
                let int_value: i64 = value.extract()?;
                let encoded = encode_sign_magnitude(int_value, param_bits)?;
                encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
            }
            "twos_complement" | "twos-complement" | "twoscomplement" => {
                let int_value: i64 = value.extract()?;
                let encoded = encode_twos_complement(int_value, param_bits)?;
                encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
            }

            // BCD encodings
            "bcd" => {
                let int_value: i64 = value.extract()?;
                let encoded = encode_unpacked_bcd(int_value, param_bits, false)?;
                encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
            }
            "bcd_signed" | "bcd-signed" | "bcdsigned" => {
                let int_value: i64 = value.extract()?;
                let encoded = encode_unpacked_bcd(int_value, param_bits, true)?;
                encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
            }
            "packed_bcd" | "packed-bcd" | "packedbcd" => {
                let int_value: i64 = value.extract()?;
                let encoded = encode_packed_bcd(int_value, param_bits, false)?;
                encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
            }
            "packed_bcd_signed" | "packed-bcd-signed" | "packedbcdsigned" => {
                let int_value: i64 = value.extract()?;
                let encoded = encode_packed_bcd(int_value, param_bits, true)?;
                encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
            }

            // Raw binary bytes
            "binary" => {
                let binary_value: Vec<u8> = value.extract()?;
                encode_binary_to_bytes(&binary_value, param_bits, reverse_bits)?
            }

            // Float encodings
            "ieee_754" | "ieee-754" | "ieee754" => match param_bits {
                16 => {
                    let float_value: f32 = value.extract()?;
                    let encoded = encode_ieee754_f16(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                32 => {
                    let float_value: f32 = value.extract()?;
                    let encoded = encode_ieee754_f32(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                64 => {
                    let float_value: f64 = value.extract()?;
                    let encoded = encode_ieee754_f64(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                128 => {
                    let float_value: f64 = value.extract()?;
                    let encoded = encode_ieee754_f128(float_value)?;
                    encode_integer_to_bytes_u128(encoded, param_bits, reverse_bits)?
                }
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unsupported IEEE 754 bit width: {} (supported: 16, 32, 64, 128)",
                        param_bits
                    )));
                }
            },
            "ieee754_1985" | "ieee754-1985" | "ieee_754_1985" | "ieee_754-1985" => match param_bits
            {
                32 => {
                    let float_value: f32 = value.extract()?;
                    let encoded = encode_ieee754_1985_f32(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                64 => {
                    let float_value: f64 = value.extract()?;
                    let encoded = encode_ieee754_1985_f64(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                80 => {
                    let float_value: f64 = value.extract()?;
                    let encoded = encode_ieee754_1985_f80(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unsupported IEEE 754-1985 bit width: {} (supported: 32, 64, 80)",
                        param_bits
                    )));
                }
            },
            "mil_std_1750a" | "mil-std-1750a" | "mil_std-1750a" => match param_bits {
                32 => {
                    let float_value: f32 = value.extract()?;
                    let encoded = encode_mil_std_1750a_f32(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                48 => {
                    let float_value: f64 = value.extract()?;
                    let encoded = encode_mil_std_1750a_f48(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unsupported MIL-STD-1750A bit width: {} (supported: 32, 48)",
                        param_bits
                    )));
                }
            },
            "dec" => match param_bits {
                32 => {
                    let float_value: f32 = value.extract()?;
                    let encoded = encode_dec_f32(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                64 => {
                    let float_value: f64 = value.extract()?;
                    let encoded = encode_dec_f64(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unsupported DEC float bit width: {} (supported: 32, 64)",
                        param_bits
                    )));
                }
            },
            "ibm" => match param_bits {
                32 => {
                    let float_value: f32 = value.extract()?;
                    let encoded = encode_ibm_f32(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                64 => {
                    let float_value: f64 = value.extract()?;
                    let encoded = encode_ibm_f64(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unsupported IBM float bit width: {} (supported: 32, 64)",
                        param_bits
                    )));
                }
            },
            "ti" => match param_bits {
                32 => {
                    let float_value: f32 = value.extract()?;
                    let encoded = encode_ti_f32(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                40 => {
                    let float_value: f64 = value.extract()?;
                    let encoded = encode_ti_f40(float_value)?;
                    encode_integer_to_bytes(encoded, param_bits, reverse_bits)?
                }
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unsupported TI float bit width: {} (supported: 32, 40)",
                        param_bits
                    )));
                }
            },

            // String encodings
            "us_ascii" | "us-ascii" | "usascii" => {
                let string_value: String = value.extract()?;
                encode_us_ascii(&string_value)?
            }
            "iso_8859_1" | "iso-8859-1" | "iso8859_1" | "iso8859-1" => {
                let string_value: String = value.extract()?;
                encode_iso_8859_1(&string_value)?
            }
            "windows_1252" | "windows-1252" | "windows1252" => {
                let string_value: String = value.extract()?;
                encode_windows_1252(&string_value)?
            }
            "utf_8" | "utf-8" | "utf8" => {
                let string_value: String = value.extract()?;
                encode_utf8(&string_value)?
            }
            "utf_16" | "utf-16" | "utf16" => {
                let string_value: String = value.extract()?;
                encode_utf16(&string_value)?
            }
            "utf_16_le" | "utf-16le" | "utf16le" => {
                let string_value: String = value.extract()?;
                encode_utf16le(&string_value)?
            }
            "utf_16_be" | "utf-16be" | "utf16be" => {
                let string_value: String = value.extract()?;
                encode_utf16be(&string_value)?
            }
            "utf_32" | "utf-32" | "utf32" => {
                let string_value: String = value.extract()?;
                encode_utf32(&string_value)?
            }
            "utf_32_le" | "utf-32le" | "utf32le" => {
                let string_value: String = value.extract()?;
                encode_utf32le(&string_value)?
            }
            "utf_32_be" | "utf-32be" | "utf32be" => {
                let string_value: String = value.extract()?;
                encode_utf32be(&string_value)?
            }

            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unsupported encoding type: {}",
                    param_encoding
                )));
            }
        };

        // Apply byte order
        let ordered_bytes = apply_byte_order_encode(&encoded_bytes, &byte_order)?;

        // Write bits into packet at the specified offset.
        // When the source is wider than param_bits, keep the rightmost param_bits.
        // When the source is narrower than param_bits, pad trailing bits with zero.
        let source_total_bits = ordered_bytes.len() * 8;
        let field_bits = param_bits as usize;
        let source_start_bit = source_total_bits.saturating_sub(field_bits);

        for bit_idx in 0..field_bits {
            let source_bit_abs = source_start_bit + bit_idx;
            let source_bit = if source_bit_abs < source_total_bits {
                let source_byte_idx = source_bit_abs / 8;
                let source_bit_in_byte = 7 - (source_bit_abs % 8);
                (ordered_bytes[source_byte_idx] >> source_bit_in_byte) & 1
            } else {
                0
            };

            let dest_bit_pos = actual_offset_bits + bit_idx;
            let dest_byte_idx = dest_bit_pos / 8;
            let dest_bit_in_byte = 7 - (dest_bit_pos % 8); // MSB is bit 7

            if dest_byte_idx < packet_bytes.len() {
                if source_bit == 1 {
                    packet_bytes[dest_byte_idx] |= 1 << dest_bit_in_byte;
                } else {
                    packet_bytes[dest_byte_idx] &= !(1 << dest_bit_in_byte);
                }
            }
        }
    }

    Ok(packet_bytes)
}

#[pyfunction]
pub fn encode_ones_complement(value: i64, bits: u32) -> PyResult<u64> {
    if bits > 64 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "bits must be <= 64 for ones' complement encoding",
        ));
    }

    if value >= 0 {
        // Positive values are stored as is
        let unsigned_value = value as u64;
        let max_value = if bits == 64 {
            i64::MAX as u64
        } else {
            (1u64 << (bits - 1)) - 1
        };

        if unsigned_value > max_value {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Value {} is too large for {} bits",
                value, bits
            )));
        }
        Ok(unsigned_value)
    } else {
        // Negative values use bitwise NOT
        let magnitude = -value as u64;
        let max_magnitude = if bits == 64 {
            i64::MAX as u64
        } else {
            (1u64 << (bits - 1)) - 1
        };

        if magnitude > max_magnitude {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Value {} is too large (magnitude) for {} bits",
                value, bits
            )));
        }

        let mask = if bits == 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        Ok((mask - magnitude) & mask)
    }
}

#[pyfunction]
pub fn encode_twos_complement(value: i64, bits: u32) -> PyResult<u64> {
    if bits > 64 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "bits must be <= 64 for two's complement encoding",
        ));
    }

    if bits == 64 {
        return Ok(value as u64);
    }

    let min_value = -(1i64 << (bits - 1));
    let max_value = (1i64 << (bits - 1)) - 1;

    if value < min_value || value > max_value {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Value {} out of range for {} bits (range: {} to {})",
            value, bits, min_value, max_value
        )));
    }

    let mask = (1u64 << bits) - 1;
    Ok((value as u64) & mask)
}

#[pyfunction]
pub fn encode_sign_magnitude(value: i64, bits: u32) -> PyResult<u64> {
    if bits < 2 || bits > 64 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "bits must be between 2 and 64 for sign-magnitude encoding",
        ));
    }

    let magnitude = value.abs() as u64;
    let max_magnitude = (1u64 << (bits - 1)) - 1;

    if magnitude > max_magnitude {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Magnitude of {} is too large for {} bits",
            value, bits
        )));
    }

    if value < 0 {
        // Set sign bit
        Ok(magnitude | (1u64 << (bits - 1)))
    } else {
        Ok(magnitude)
    }
}

#[pyfunction]
#[pyo3(signature = (value, bits, signed = false))]
pub fn encode_unpacked_bcd(value: i64, bits: u32, signed: bool) -> PyResult<u64> {
    let num_digits = (bits / 8) as usize;
    if num_digits == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "bits must be >= 8 for unpacked BCD",
        ));
    }

    let (is_negative, magnitude) = if value < 0 {
        (true, (-value) as u64)
    } else {
        (false, value as u64)
    };

    if signed && is_negative && !signed {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Cannot encode negative value in unsigned BCD",
        ));
    }

    // Extract decimal digits
    let mut temp = magnitude;
    let mut digits = Vec::new();
    for _ in 0..num_digits {
        digits.push((temp % 10) as u8);
        temp /= 10;
    }

    if temp > 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Value {} has too many digits for {} BCD digits",
            value, num_digits
        )));
    }

    // Encode to unpacked BCD
    let mut result = 0u64;
    if signed {
        // Zoned decimal format
        for (i, &digit) in digits.iter().enumerate() {
            let byte_val = if i == 0 {
                // First (rightmost) byte contains sign and digit
                if is_negative {
                    0xD0 | digit // EBCDIC negative
                } else {
                    0xC0 | digit // EBCDIC positive
                }
            } else {
                0xF0 | digit // EBCDIC zone
            };
            result |= (byte_val as u64) << (i * 8);
        }
    } else {
        // Plain BCD (one digit per byte)
        for (i, &digit) in digits.iter().enumerate() {
            result |= (digit as u64) << (i * 8);
        }
    }

    Ok(result)
}

#[pyfunction]
#[pyo3(signature = (value, bits, signed = false))]
pub fn encode_packed_bcd(value: i64, bits: u32, signed: bool) -> PyResult<u64> {
    let num_digits = (bits / 4) as usize;
    if num_digits == 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "bits must be >= 4 for packed BCD",
        ));
    }

    let (is_negative, magnitude) = if value < 0 {
        (true, (-value) as u64)
    } else {
        (false, value as u64)
    };

    if !signed && is_negative {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Cannot encode negative value in unsigned BCD",
        ));
    }

    // Extract decimal digits
    let max_digits = if signed { num_digits - 1 } else { num_digits };
    let mut temp = magnitude;
    let mut digits = Vec::new();
    for _ in 0..max_digits {
        digits.push((temp % 10) as u8);
        temp /= 10;
    }

    if temp > 0 {
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Value {} has too many digits for {} BCD digits",
            value, max_digits
        )));
    }

    // Encode to packed BCD
    let mut result = 0u64;
    if signed {
        // Add sign nibble at the end
        let sign_nibble = if is_negative { 0xD } else { 0xC };
        result |= sign_nibble;

        for (i, &digit) in digits.iter().enumerate() {
            result |= (digit as u64) << ((i + 1) * 4);
        }
    } else {
        for (i, &digit) in digits.iter().enumerate() {
            result |= (digit as u64) << (i * 4);
        }
    }

    Ok(result)
}

#[pyfunction]
pub fn encode_ieee754_f16(value: f32) -> PyResult<u64> {
    let f16_value = f16::from_f32(value);
    Ok(f16_value.to_bits() as u64)
}

#[pyfunction]
pub fn encode_ieee754_f32(value: f32) -> PyResult<u64> {
    Ok(value.to_bits() as u64)
}

#[pyfunction]
pub fn encode_ieee754_f64(value: f64) -> PyResult<u64> {
    Ok(value.to_bits())
}

#[pyfunction]
pub fn encode_ieee754_f128(_value: f64) -> PyResult<u128> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "IEEE 754 f128 encoding not yet implemented",
    ))
}

#[pyfunction]
pub fn encode_ieee754_1985_f32(_value: f32) -> PyResult<u64> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "IEEE 754-1985 f32 encoding not yet implemented",
    ))
}

#[pyfunction]
pub fn encode_ieee754_1985_f64(_value: f64) -> PyResult<u64> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "IEEE 754-1985 f64 encoding not yet implemented",
    ))
}

#[pyfunction]
pub fn encode_ieee754_1985_f80(_value: f64) -> PyResult<u64> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "IEEE 754-1985 f80 encoding not yet implemented",
    ))
}

#[pyfunction]
pub fn encode_mil_std_1750a_f32(_value: f32) -> PyResult<u64> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "MIL-STD-1750A f32 encoding not yet implemented",
    ))
}

#[pyfunction]
pub fn encode_mil_std_1750a_f48(_value: f64) -> PyResult<u64> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "MIL-STD-1750A f48 encoding not yet implemented",
    ))
}

#[pyfunction]
pub fn encode_dec_f32(_value: f32) -> PyResult<u64> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "DEC f32 encoding not yet implemented",
    ))
}

#[pyfunction]
pub fn encode_dec_f64(_value: f64) -> PyResult<u64> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "DEC f64 encoding not yet implemented",
    ))
}

#[pyfunction]
pub fn encode_ibm_f32(_value: f32) -> PyResult<u64> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "IBM f32 encoding not yet implemented",
    ))
}

#[pyfunction]
pub fn encode_ibm_f64(_value: f64) -> PyResult<u64> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "IBM f64 encoding not yet implemented",
    ))
}

#[pyfunction]
pub fn encode_ti_f32(_value: f32) -> PyResult<u64> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "TI f32 encoding not yet implemented",
    ))
}

#[pyfunction]
pub fn encode_ti_f40(_value: f64) -> PyResult<u64> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "TI f40 encoding not yet implemented",
    ))
}

#[pyfunction]
pub fn encode_us_ascii(text: &str) -> PyResult<Vec<u8>> {
    for ch in text.chars() {
        if ch as u32 > 127 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Character '{}' (U+{:04X}) is not valid US-ASCII",
                ch, ch as u32
            )));
        }
    }
    Ok(text.as_bytes().to_vec())
}

#[pyfunction]
pub fn encode_iso_8859_1(text: &str) -> PyResult<Vec<u8>> {
    let mut result = Vec::new();
    for ch in text.chars() {
        let code = ch as u32;
        if code > 255 {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Character '{}' (U+{:04X}) is not valid ISO-8859-1",
                ch, code
            )));
        }
        result.push(code as u8);
    }
    Ok(result)
}

#[pyfunction]
pub fn encode_windows_1252(text: &str) -> PyResult<Vec<u8>> {
    let (cow, _encoding_used, had_errors) = WINDOWS_1252.encode(text);
    if had_errors {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "Failed to encode to Windows-1252",
        ));
    }
    Ok(cow.into_owned())
}

#[pyfunction]
pub fn encode_utf8(text: &str) -> PyResult<Vec<u8>> {
    Ok(text.as_bytes().to_vec())
}

#[pyfunction]
pub fn encode_utf16(text: &str) -> PyResult<Vec<u8>> {
    let mut result = vec![0xFE, 0xFF]; // BOM for big-endian
    result.extend(encode_utf16be(text)?);
    Ok(result)
}

#[pyfunction]
pub fn encode_utf16le(text: &str) -> PyResult<Vec<u8>> {
    let u16_vec: Vec<u16> = text.encode_utf16().collect();
    let mut result = Vec::with_capacity(u16_vec.len() * 2);
    for code_unit in u16_vec {
        result.extend_from_slice(&code_unit.to_le_bytes());
    }
    Ok(result)
}

#[pyfunction]
pub fn encode_utf16be(text: &str) -> PyResult<Vec<u8>> {
    let u16_vec: Vec<u16> = text.encode_utf16().collect();
    let mut result = Vec::with_capacity(u16_vec.len() * 2);
    for code_unit in u16_vec {
        result.extend_from_slice(&code_unit.to_be_bytes());
    }
    Ok(result)
}

#[pyfunction]
pub fn encode_utf32(text: &str) -> PyResult<Vec<u8>> {
    let mut result = vec![0x00, 0x00, 0xFE, 0xFF]; // BOM for big-endian
    result.extend(encode_utf32be(text)?);
    Ok(result)
}

#[pyfunction]
pub fn encode_utf32le(text: &str) -> PyResult<Vec<u8>> {
    let mut result = Vec::new();
    for ch in text.chars() {
        result.extend_from_slice(&(ch as u32).to_le_bytes());
    }
    Ok(result)
}

#[pyfunction]
pub fn encode_utf32be(text: &str) -> PyResult<Vec<u8>> {
    let mut result = Vec::new();
    for ch in text.chars() {
        result.extend_from_slice(&(ch as u32).to_be_bytes());
    }
    Ok(result)
}
