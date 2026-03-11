"""Unit tests for integer decoder functions."""

import pytest

import xtce_codec


class TestUnsignedDecoding:
    """Test unsigned integer decoding (built into Python, but test our interface)."""

    def test_basic_unsigned(self):
        """Test basic unsigned integer conversion."""
        # 8-bit: 255 = 0xFF
        assert int.from_bytes(b"\xff", "big") == 255
        assert int.from_bytes(b"\x00", "big") == 0
        assert int.from_bytes(b"\x80", "big") == 128

        # 16-bit: 65535 = 0xFFFF
        assert int.from_bytes(b"\xff\xff", "big") == 65535
        assert int.from_bytes(b"\x00\x00", "big") == 0
        assert int.from_bytes(b"\x80\x00", "big") == 32768


class TestOnesComplement:
    """Test ones' complement decoder."""

    def test_positive_numbers(self):
        """Test positive numbers (MSB = 0)."""
        # 8-bit positive numbers
        assert xtce_codec.decode_ones_complement(0b01111111, 8) == 127  # Max positive
        assert xtce_codec.decode_ones_complement(0b00000001, 8) == 1
        assert xtce_codec.decode_ones_complement(0b00000000, 8) == 0  # Positive zero

        # 16-bit positive numbers
        assert xtce_codec.decode_ones_complement(0b0111111111111111, 16) == 32767

    def test_negative_numbers(self):
        """Test negative numbers (MSB = 1)."""
        # 8-bit negative numbers
        assert xtce_codec.decode_ones_complement(0b11111111, 8) == 0  # Negative zero
        assert xtce_codec.decode_ones_complement(0b11111110, 8) == -1
        assert xtce_codec.decode_ones_complement(0b10000000, 8) == -127  # Max negative

        # 16-bit negative numbers
        assert xtce_codec.decode_ones_complement(0b1000000000000000, 16) == -32767

    def test_edge_cases(self):
        """Test edge cases and different bit widths."""
        # 1-bit
        assert xtce_codec.decode_ones_complement(0, 1) == 0
        assert xtce_codec.decode_ones_complement(1, 1) == -0  # Negative zero

        # 4-bit
        assert xtce_codec.decode_ones_complement(0b0111, 4) == 7  # Max positive
        assert xtce_codec.decode_ones_complement(0b1000, 4) == -7  # Max negative


class TestTwosComplement:
    """Test two's complement decoder."""

    def test_positive_numbers(self):
        """Test positive numbers (MSB = 0)."""
        # 8-bit positive numbers
        assert xtce_codec.decode_twos_complement(0b01111111, 8) == 127
        assert xtce_codec.decode_twos_complement(0b00000001, 8) == 1
        assert xtce_codec.decode_twos_complement(0b00000000, 8) == 0

        # 16-bit positive numbers
        assert xtce_codec.decode_twos_complement(0b0111111111111111, 16) == 32767

    def test_negative_numbers(self):
        """Test negative numbers (MSB = 1)."""
        # 8-bit negative numbers
        assert xtce_codec.decode_twos_complement(0b11111111, 8) == -1
        assert xtce_codec.decode_twos_complement(0b11111110, 8) == -2
        assert xtce_codec.decode_twos_complement(0b10000000, 8) == -128  # Most negative

        # 16-bit negative numbers
        assert xtce_codec.decode_twos_complement(0b1000000000000000, 16) == -32768

    def test_edge_cases(self):
        """Test edge cases and different bit widths."""
        # 1-bit (can only represent 0 and -1)
        assert xtce_codec.decode_twos_complement(0, 1) == 0
        assert xtce_codec.decode_twos_complement(1, 1) == -1

        # 4-bit
        assert xtce_codec.decode_twos_complement(0b0111, 4) == 7  # Max positive
        assert xtce_codec.decode_twos_complement(0b1000, 4) == -8  # Most negative


class TestSignMagnitude:
    """Test sign-magnitude decoder."""

    def test_positive_numbers(self):
        """Test positive numbers (sign bit = 0)."""
        # 8-bit positive numbers
        assert xtce_codec.decode_sign_magnitude(0b01111111, 8) == 127
        assert xtce_codec.decode_sign_magnitude(0b00000001, 8) == 1
        assert xtce_codec.decode_sign_magnitude(0b00000000, 8) == 0  # Positive zero

    def test_negative_numbers(self):
        """Test negative numbers (sign bit = 1)."""
        # 8-bit negative numbers
        assert xtce_codec.decode_sign_magnitude(0b11111111, 8) == -127
        assert xtce_codec.decode_sign_magnitude(0b10000001, 8) == -1
        assert xtce_codec.decode_sign_magnitude(0b10000000, 8) == 0  # Negative zero

    def test_edge_cases(self):
        """Test edge cases and different bit widths."""
        # 1-bit (can only represent 0 and -0)
        assert xtce_codec.decode_sign_magnitude(0, 1) == 0
        assert xtce_codec.decode_sign_magnitude(1, 1) == 0  # Negative zero

        # 4-bit
        assert xtce_codec.decode_sign_magnitude(0b0111, 4) == 7  # Max positive
        assert xtce_codec.decode_sign_magnitude(0b1111, 4) == -7  # Max negative


class TestBCD:
    """Test Unpacked Binary Coded Decimal decoder (each digit uses full byte)."""

    def test_valid_unpacked_bcd(self):
        """Test valid unpacked BCD values."""
        # Single digits (each digit in its own byte)
        assert xtce_codec.decode_unpacked_bcd(0x00, 8) == 0  # 0
        assert xtce_codec.decode_unpacked_bcd(0x05, 8) == 5  # 5
        assert xtce_codec.decode_unpacked_bcd(0x09, 8) == 9  # 9

        # Multiple digits (each digit in separate bytes, processed LSB first)
        assert (
            xtce_codec.decode_unpacked_bcd(0x0102, 16) == 12
        )  # 0x02|0x01 -> 2 + 1*10 = 12
        assert (
            xtce_codec.decode_unpacked_bcd(0x030201, 24) == 321
        )  # 0x01|0x02|0x03 -> 1 + 2*10 + 3*100 = 321

    def test_invalid_unpacked_bcd(self):
        """Test invalid unpacked BCD values (digits > 9 in any byte)."""
        # Single invalid digit
        with pytest.raises(ValueError, match="Invalid BCD digit: 10"):
            xtce_codec.decode_unpacked_bcd(0x0A, 8)  # A = 10, invalid

        with pytest.raises(ValueError, match="Invalid BCD digit: 15"):
            xtce_codec.decode_unpacked_bcd(0x0F, 8)  # F = 15, invalid

        # Valid first byte, invalid second byte
        with pytest.raises(ValueError, match="Invalid BCD digit: 10"):
            xtce_codec.decode_unpacked_bcd(0x0A05, 16)  # Second byte (0x0A) is invalid

        # Invalid first byte (processed first due to LSB order)
        with pytest.raises(ValueError, match="Invalid BCD digit: 12"):
            xtce_codec.decode_unpacked_bcd(0x050C, 16)  # First byte (0x0C) is invalid


class TestPackedBCD:
    """Test Packed Binary Coded Decimal decoder (two digits per byte)."""

    def test_valid_packed_bcd(self):
        """Test valid packed BCD values."""
        # Single byte (two digits packed, processed LSB first)
        assert (
            xtce_codec.decode_packed_bcd(0x12, 8) == 12
        )  # 0x12 -> nibbles 2,1 -> 2 + 1*10 = 12
        assert (
            xtce_codec.decode_packed_bcd(0x99, 8) == 99
        )  # 0x99 -> nibbles 9,9 -> 9 + 9*10 = 99
        assert (
            xtce_codec.decode_packed_bcd(0x56, 8) == 56
        )  # 0x56 -> nibbles 6,5 -> 6 + 5*10 = 56

        # Multiple bytes (processed nibble by nibble, LSB first)
        assert (
            xtce_codec.decode_packed_bcd(0x1234, 16) == 1234
        )  # nibbles 4,3,2,1 -> 4 + 3*10 + 2*100 + 1*1000

        # Single digit (4 bits)
        assert xtce_codec.decode_packed_bcd(0x5, 4) == 5  # Just one nibble

    def test_invalid_packed_bcd(self):
        """Test invalid packed BCD values."""
        # Single nibble invalid (> 9)
        with pytest.raises(ValueError, match="Invalid BCD digit: 10"):
            xtce_codec.decode_packed_bcd(0xA, 4)  # A = 10, invalid nibble

        with pytest.raises(ValueError, match="Invalid BCD digit: 15"):
            xtce_codec.decode_packed_bcd(0xF, 4)  # F = 15, invalid nibble

        # First nibble invalid (processed first due to LSB order)
        with pytest.raises(ValueError, match="Invalid BCD digit: 11"):
            xtce_codec.decode_packed_bcd(0x5B, 8)  # LSB nibble B = 11, invalid

        # Second nibble invalid
        with pytest.raises(ValueError, match="Invalid BCD digit: 12"):
            xtce_codec.decode_packed_bcd(
                0xC5, 8
            )  # MSB nibble C = 12, invalid (processed second)

        # Both nibbles invalid - should fail on first (LSB) nibble
        with pytest.raises(ValueError, match="Invalid BCD digit: 11"):
            xtce_codec.decode_packed_bcd(
                0xAB, 8
            )  # Both A=10 and B=11 invalid, fails on B first


class TestSignedPackedBCD:
    """Test signed packed BCD with sign codes."""

    def test_positive_signs(self):
        """Test all valid positive sign codes for packed BCD."""
        # Standard positive signs
        assert xtce_codec.decode_packed_bcd(0x123C, 16, signed=True) == 123
        assert xtce_codec.decode_packed_bcd(0x123F, 16, signed=True) == 123
        assert xtce_codec.decode_packed_bcd(0x123A, 16, signed=True) == 123
        assert xtce_codec.decode_packed_bcd(0x123E, 16, signed=True) == 123

    def test_negative_signs(self):
        """Test all valid negative sign codes for packed BCD."""
        # Standard negative signs
        assert xtce_codec.decode_packed_bcd(0x456D, 16, signed=True) == -456
        assert xtce_codec.decode_packed_bcd(0x456B, 16, signed=True) == -456

    def test_large_signed_numbers(self):
        """Test larger signed packed BCD numbers."""
        # Large positive number
        assert xtce_codec.decode_packed_bcd(0x9876543C, 32, signed=True) == 9876543

        # Large negative number
        assert xtce_codec.decode_packed_bcd(0x9876543D, 32, signed=True) == -9876543

    def test_zero_with_signs(self):
        """Test zero with different sign codes."""
        assert xtce_codec.decode_packed_bcd(0x000C, 16, signed=True) == 0
        assert (
            xtce_codec.decode_packed_bcd(0x000D, 16, signed=True) == 0
        )  # -0 becomes 0

    def test_invalid_sign_nibbles(self):
        """Test error handling for invalid sign nibbles."""
        # Invalid sign nibbles (0-9 are not allowed as signs)
        with pytest.raises(
            ValueError, match="Invalid BCD sign nibble: expected 0xA-0xF"
        ):
            xtce_codec.decode_packed_bcd(0x1234, 16, signed=True)  # 0x4 invalid

        with pytest.raises(
            ValueError, match="Invalid BCD sign nibble: expected 0xA-0xF"
        ):
            xtce_codec.decode_packed_bcd(0x1230, 16, signed=True)  # 0x0 invalid

    def test_backward_compatibility(self):
        """Test that unsigned mode still works (backward compatibility)."""
        # These should work the same as before
        assert xtce_codec.decode_packed_bcd(0x123456, 24) == 123456
        assert xtce_codec.decode_packed_bcd(0x123456, 24, signed=False) == 123456


class TestSignedUnpackedBCD:
    """Test signed unpacked BCD with zoned decimal format."""

    def test_positive_signs(self):
        """Test valid positive sign codes in zoned decimal format."""
        # Proper zoned decimal: ASCII digits + EBCDIC positive signed digits
        assert (
            xtce_codec.decode_unpacked_bcd(0x37C8, 16, signed=True) == 78
        )  # ASCII '7' + positive '8'
        assert (
            xtce_codec.decode_unpacked_bcd(0x31C2, 16, signed=True) == 12
        )  # ASCII '1' + positive '2'

    def test_negative_signs(self):
        """Test valid negative sign codes in zoned decimal format."""
        # Proper zoned decimal: ASCII digits + EBCDIC negative signed digits
        assert (
            xtce_codec.decode_unpacked_bcd(0x39D0, 16, signed=True) == -90
        )  # ASCII '9' + negative '0'
        assert (
            xtce_codec.decode_unpacked_bcd(0x35D6, 16, signed=True) == -56
        )  # ASCII '5' + negative '6'

    def test_multi_byte_zoned(self):
        """Test multi-byte zoned decimal numbers."""
        # 3-byte number: proper zoned decimal format
        assert (
            xtce_codec.decode_unpacked_bcd(0x3132C3, 24, signed=True) == 123
        )  # ASCII '1','2' + positive '3'

        # 3-byte number: negative zoned decimal
        assert (
            xtce_codec.decode_unpacked_bcd(0x3435D6, 24, signed=True) == -456
        )  # ASCII '4','5' + negative '6'

    def test_zero_with_signs(self):
        """Test zero with different zoned sign codes."""
        assert (
            xtce_codec.decode_unpacked_bcd(0x30C0, 16, signed=True) == 0
        )  # ASCII '0' + positive '0'
        assert (
            xtce_codec.decode_unpacked_bcd(0x30D0, 16, signed=True) == 0
        )  # ASCII '0' + negative '0' -> 0

    def test_invalid_sign_nibbles(self):
        """Test error handling for invalid zoned decimal bytes."""
        # Invalid zoned decimal signed bytes
        with pytest.raises(ValueError, match="Invalid zoned decimal signed byte: 0x48"):
            xtce_codec.decode_unpacked_bcd(0x3748, 16, signed=True)  # 0x48 invalid

        with pytest.raises(ValueError, match="Invalid zoned decimal signed byte: 0xA8"):
            xtce_codec.decode_unpacked_bcd(0x37A8, 16, signed=True)  # 0xA8 invalid

        with pytest.raises(ValueError, match="Invalid zoned decimal signed byte: 0x08"):
            xtce_codec.decode_unpacked_bcd(0x3708, 16, signed=True)  # 0x08 invalid

    def test_invalid_bcd_digits(self):
        """Test error handling for invalid zoned decimal bytes."""
        # Invalid zoned decimal signed byte
        with pytest.raises(ValueError, match="Invalid zoned decimal signed byte: 0xFA"):
            xtce_codec.decode_unpacked_bcd(0x37FA, 16, signed=True)  # 0xFA invalid

        # Invalid zoned decimal non-sign byte
        with pytest.raises(ValueError, match="Invalid zoned decimal byte: 0x0C"):
            xtce_codec.decode_unpacked_bcd(0x0CC7, 16, signed=True)  # 0x0C invalid

    def test_backward_compatibility(self):
        """Test that unsigned mode still works (backward compatibility)."""
        # These should work the same as before
        assert xtce_codec.decode_unpacked_bcd(0x01020304, 32) == 1234
        assert xtce_codec.decode_unpacked_bcd(0x01020304, 32, signed=False) == 1234


class TestBCDEdgeCases:
    """Test edge cases and comprehensive scenarios for BCD functions."""

    def test_single_digit_signed(self):
        """Test single digit signed BCD numbers."""
        # Single digit packed BCD
        assert xtce_codec.decode_packed_bcd(0x5C, 8, signed=True) == 5  # +5
        assert xtce_codec.decode_packed_bcd(0x5D, 8, signed=True) == -5  # -5

        # Single digit zoned BCD (proper format)
        assert xtce_codec.decode_unpacked_bcd(0xC5, 8, signed=True) == 5  # positive '5'
        assert (
            xtce_codec.decode_unpacked_bcd(0xD5, 8, signed=True) == -5
        )  # negative '5'

    def test_all_packed_sign_combinations(self):
        """Test all valid packed BCD sign code combinations."""
        # BCD representation of 987: 0x987 (each nibble is a BCD digit)
        bcd_digits = 0x987

        # All positive signs should give +987
        for sign in [0xC, 0xF, 0xA, 0xE]:
            packed_value = (bcd_digits << 4) | sign  # Shift BCD digits left, add sign
            result = xtce_codec.decode_packed_bcd(packed_value, 16, signed=True)
            assert result == 987, f"Sign 0x{sign:X} should be positive, got {result}"

        # All negative signs should give -987
        for sign in [0xD, 0xB]:
            packed_value = (bcd_digits << 4) | sign  # Shift BCD digits left, add sign
            result = xtce_codec.decode_packed_bcd(packed_value, 16, signed=True)
            assert result == -987, f"Sign 0x{sign:X} should be negative, got {result}"

    def test_all_zoned_sign_combinations(self):
        """Test all valid zoned decimal sign code combinations."""
        # Test positive zoned decimal signs (0xC0-0xC9)
        for digit in range(10):
            positive_byte = 0xC0 + digit
            result = xtce_codec.decode_unpacked_bcd(positive_byte, 8, signed=True)
            assert result == digit, (
                f"0x{positive_byte:02X} should be +{digit}, got {result}"
            )

        # Test negative zoned decimal signs (0xD0-0xD9)
        for digit in range(10):
            negative_byte = 0xD0 + digit
            result = xtce_codec.decode_unpacked_bcd(negative_byte, 8, signed=True)
            assert result == -digit, (
                f"0x{negative_byte:02X} should be -{digit}, got {result}"
            )

    def test_maximum_values(self):
        """Test maximum representable values in different bit widths."""
        # 4-bit packed BCD (single digit)
        assert xtce_codec.decode_packed_bcd(0x9, 4) == 9

        # 8-bit packed BCD (1 digit + sign)
        assert xtce_codec.decode_packed_bcd(0x9C, 8, signed=True) == 9
        assert xtce_codec.decode_packed_bcd(0x9D, 8, signed=True) == -9

        # 8-bit zoned BCD (1 digit with sign)
        assert xtce_codec.decode_unpacked_bcd(0xC9, 8, signed=True) == 9
        assert xtce_codec.decode_unpacked_bcd(0xD9, 8, signed=True) == -9
