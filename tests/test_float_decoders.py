"""Unit tests for floating-point decoder functions."""

import struct

import xtce_codec


class TestIEEE754F32:
    """Test IEEE 754 32-bit floating-point decoder."""

    def test_basic_values(self):
        """Test basic floating-point values."""
        # Test known values using struct to get bit representations
        test_values = [0.0, 1.0, -1.0, 3.14159, -3.14159, 1.5, -1.5]

        for expected in test_values:
            # Get the 32-bit representation
            packed = struct.pack(">f", expected)
            bits_as_int = struct.unpack(">I", packed)[0]

            # Test our decoder
            result = xtce_codec.decode_ieee754_f32(bits_as_int)
            assert abs(result - expected) < 1e-6, f"Expected {expected}, got {result}"

    def test_special_values(self):
        """Test special IEEE 754 values."""
        # Positive zero: 0x00000000
        assert xtce_codec.decode_ieee754_f32(0x00000000) == 0.0

        # Negative zero: 0x80000000
        result = xtce_codec.decode_ieee754_f32(0x80000000)
        assert result == 0.0  # -0.0 == 0.0 in Python

        # Positive infinity: 0x7F800000
        result = xtce_codec.decode_ieee754_f32(0x7F800000)
        assert result == float("inf")

        # Negative infinity: 0xFF800000
        result = xtce_codec.decode_ieee754_f32(0xFF800000)
        assert result == float("-inf")

        # NaN: 0x7FC00000 (one example)
        result = xtce_codec.decode_ieee754_f32(0x7FC00000)
        assert result != result  # NaN != NaN

    def test_edge_cases(self):
        """Test edge cases and additional values."""
        # Test smallest positive normal number
        result = xtce_codec.decode_ieee754_f32(0x00800000)  # Smallest normal f32
        assert result > 0.0

        # Test largest finite number
        result = xtce_codec.decode_ieee754_f32(0x7F7FFFFF)  # Largest finite f32
        assert result < float("inf")


class TestIEEE754F64:
    """Test IEEE 754 64-bit floating-point decoder."""

    def test_basic_values(self):
        """Test basic floating-point values."""
        test_values = [0.0, 1.0, -1.0, 3.141592653589793, -3.141592653589793]

        for expected in test_values:
            # Get the 64-bit representation
            packed = struct.pack(">d", expected)
            bits_as_int = struct.unpack(">Q", packed)[0]

            # Test our decoder
            result = xtce_codec.decode_ieee754_f64(bits_as_int)
            assert abs(result - expected) < 1e-15, f"Expected {expected}, got {result}"

    def test_special_values(self):
        """Test special IEEE 754 f64 values."""
        # Positive infinity
        result = xtce_codec.decode_ieee754_f64(0x7FF0000000000000)
        assert result == float("inf")

        # Negative infinity
        result = xtce_codec.decode_ieee754_f64(0xFFF0000000000000)
        assert result == float("-inf")

        # NaN
        result = xtce_codec.decode_ieee754_f64(0x7FF8000000000000)
        assert result != result  # NaN != NaN


class TestIEEE754F16:
    """Test IEEE 754 16-bit floating-point decoder."""

    def test_basic_values(self):
        """Test basic half-precision values."""
        # Half precision has limited range, test simple values
        test_cases = [
            (0x0000, 0.0),  # Positive zero
            (0x8000, -0.0),  # Negative zero
            (0x3C00, 1.0),  # 1.0
            (0xBC00, -1.0),  # -1.0
            (0x4000, 2.0),  # 2.0
            (0x3800, 0.5),  # 0.5
        ]

        for bits, expected in test_cases:
            result = xtce_codec.decode_ieee754_f16(bits)
            if expected == 0.0:
                assert result == 0.0  # Handle both +0.0 and -0.0
            else:
                assert abs(result - expected) < 1e-3, (
                    f"Expected {expected}, got {result}"
                )

    def test_special_values(self):
        """Test special IEEE 754 f16 values."""
        # Positive infinity: 0x7C00
        result = xtce_codec.decode_ieee754_f16(0x7C00)
        assert result == float("inf")

        # Negative infinity: 0xFC00
        result = xtce_codec.decode_ieee754_f16(0xFC00)
        assert result == float("-inf")

        # NaN: 0x7E00 (one example)
        result = xtce_codec.decode_ieee754_f16(0x7E00)
        assert result != result  # NaN != NaN


class TestIEEE754F128:
    """Test IEEE 754 128-bit floating-point decoder."""

    def test_basic_values(self):
        """Test basic f128 values."""
        # Test 1.0 in f128: Sign=0, Exp=16383, Mantissa=0
        value1 = 0x3FFF0000000000000000000000000000
        result1 = xtce_codec.decode_ieee754_f128(value1)
        assert result1 == 1.0

        # Test 2.0 in f128: Exp=16384
        value2 = 0x40000000000000000000000000000000
        result2 = xtce_codec.decode_ieee754_f128(value2)
        assert result2 == 2.0

        # Test 0.0
        value3 = 0x00000000000000000000000000000000
        result3 = xtce_codec.decode_ieee754_f128(value3)
        assert result3 == 0.0

    def test_special_values(self):
        """Test special f128 values."""
        # Positive infinity: Exp=0x7FFF, Mantissa=0
        value_pinf = 0x7FFF0000000000000000000000000000
        result_pinf = xtce_codec.decode_ieee754_f128(value_pinf)
        assert result_pinf == float("inf")

        # Negative infinity: Sign=1, Exp=0x7FFF, Mantissa=0
        value_ninf = 0xFFFF0000000000000000000000000000
        result_ninf = xtce_codec.decode_ieee754_f128(value_ninf)
        assert result_ninf == float("-inf")

        # NaN: Exp=0x7FFF, Mantissa!=0
        value_nan = 0x7FFF0000000000000000000000000001
        result_nan = xtce_codec.decode_ieee754_f128(value_nan)
        assert result_nan != result_nan  # NaN != NaN


class TestOtherFloatFormats:
    """Test other floating-point format placeholders."""

    def test_unimplemented_formats(self):
        """Test that unimplemented formats raise NotImplementedError."""
        unimplemented_funcs = [
            xtce_codec.decode_ieee754_1985_f32,
            xtce_codec.decode_ieee754_1985_f64,
            xtce_codec.decode_ieee754_1985_f80,
            xtce_codec.decode_mil_std_1750a_f32,
            xtce_codec.decode_mil_std_1750a_f48,
            xtce_codec.decode_dec_f32,
            xtce_codec.decode_dec_f64,
            xtce_codec.decode_ibm_f32,
            xtce_codec.decode_ibm_f64,
            xtce_codec.decode_ti_f32,
            xtce_codec.decode_ti_f40,
        ]

        for func in unimplemented_funcs:
            try:
                func(0x12345678)
                assert False, (
                    f"Function {func.__name__} should raise NotImplementedError"
                )
            except NotImplementedError:
                pass  # Expected
