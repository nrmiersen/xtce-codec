use pyo3::prelude::*;
use pyo3::types::PyDict;

mod decoder;
mod encoder;

// Unpack packet parameters
#[pyfunction]
fn unpack_parameters(
    py: Python,
    packet_bytes: &[u8],
    offset_bits: usize,
    recipe: Vec<Py<PyAny>>,
) -> PyResult<Py<PyDict>> {
    match decoder::process_recipe(py, packet_bytes, offset_bits, recipe) {
        Ok(results_dict) => Ok(results_dict.into()),
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
    }
}

// Pack parameters into a packet
#[pyfunction]
fn pack_parameters(
    py: Python,
    recipe: Vec<Py<PyAny>>,
    values: &Bound<'_, PyDict>,
) -> PyResult<Vec<u8>> {
    match encoder::build_packet(py, recipe, values) {
        Ok(packet_bytes) => Ok(packet_bytes),
        Err(e) => Err(pyo3::exceptions::PyValueError::new_err(e.to_string())),
    }
}

// The xtce-codec module implemented in Rust
#[pymodule]
fn xtce_codec(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    // General functions
    m.add_function(wrap_pyfunction!(unpack_parameters, m)?)?;
    m.add_function(wrap_pyfunction!(pack_parameters, m)?)?;

    // Integer decoding functions
    m.add_function(wrap_pyfunction!(decoder::decode_ones_complement, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_twos_complement, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_sign_magnitude, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_unpacked_bcd, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_packed_bcd, m)?)?;

    // Floating-point decoding functions
    m.add_function(wrap_pyfunction!(decoder::decode_ieee754_f16, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_ieee754_f32, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_ieee754_f64, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_ieee754_f128, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_ieee754_1985_f32, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_ieee754_1985_f64, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_ieee754_1985_f80, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_mil_std_1750a_f32, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_mil_std_1750a_f48, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_dec_f32, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_dec_f64, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_ibm_f32, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_ibm_f64, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_ti_f32, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_ti_f40, m)?)?;

    // String decoding functions
    m.add_function(wrap_pyfunction!(decoder::decode_us_ascii, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_iso_8859_1, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_windows_1252, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_utf8, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_utf16, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_utf16le, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_utf16be, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_utf32, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_utf32le, m)?)?;
    m.add_function(wrap_pyfunction!(decoder::decode_utf32be, m)?)?;

    // Integer encoding functions
    m.add_function(wrap_pyfunction!(encoder::encode_ones_complement, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_twos_complement, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_sign_magnitude, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_unpacked_bcd, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_packed_bcd, m)?)?;

    // Floating-point encoding functions
    m.add_function(wrap_pyfunction!(encoder::encode_ieee754_f16, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_ieee754_f32, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_ieee754_f64, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_ieee754_f128, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_ieee754_1985_f32, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_ieee754_1985_f64, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_ieee754_1985_f80, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_mil_std_1750a_f32, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_mil_std_1750a_f48, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_dec_f32, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_dec_f64, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_ibm_f32, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_ibm_f64, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_ti_f32, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_ti_f40, m)?)?;

    // String encoding functions
    m.add_function(wrap_pyfunction!(encoder::encode_us_ascii, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_iso_8859_1, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_windows_1252, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_utf8, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_utf16, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_utf16le, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_utf16be, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_utf32, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_utf32le, m)?)?;
    m.add_function(wrap_pyfunction!(encoder::encode_utf32be, m)?)?;

    Ok(())
}
