"""Unit tests for string decoder functions."""

import xtce_codec


class TestUSASCII:
    """Test US-ASCII string decoder."""

    def test_valid_ascii(self):
        """Test valid ASCII strings."""
        # Basic ASCII characters
        assert xtce_codec.decode_us_ascii(b"Hello") == "Hello"
        assert xtce_codec.decode_us_ascii(b"123") == "123"
        assert xtce_codec.decode_us_ascii(b"") == ""  # Empty string
        assert xtce_codec.decode_us_ascii(b"Hello, World!") == "Hello, World!"

        # Control characters
        assert xtce_codec.decode_us_ascii(b"\x00\x01\x1f") == "\x00\x01\x1f"

        # All valid ASCII (0-127)
        all_ascii = bytes(range(128))
        result = xtce_codec.decode_us_ascii(all_ascii)
        assert len(result) == 128

    def test_invalid_ascii(self):
        """Test invalid ASCII bytes (> 127)."""
        try:
            xtce_codec.decode_us_ascii(b"\x80")  # 128, outside ASCII range
            assert False, "Should have raised an error"
        except ValueError as e:
            assert "Invalid US-ASCII byte" in str(e)

        try:
            xtce_codec.decode_us_ascii(b"Hello\xff")  # 255, outside ASCII range
            assert False, "Should have raised an error"
        except ValueError:
            pass


class TestISO88591:
    """Test ISO-8859-1 (Latin-1) string decoder."""

    def test_valid_iso88591(self):
        """Test valid ISO-8859-1 strings."""
        # Basic ASCII (subset of ISO-8859-1)
        assert xtce_codec.decode_iso_8859_1(b"Hello") == "Hello"

        # Extended characters (128-255)
        assert xtce_codec.decode_iso_8859_1(b"\xc0\xc1\xc2") == "ÀÁÂ"  # À Á Â
        assert xtce_codec.decode_iso_8859_1(b"\xe9\xe8\xea") == "éèê"  # é è ê

        # Full range (0-255)
        all_bytes = bytes(range(256))
        result = xtce_codec.decode_iso_8859_1(all_bytes)
        assert len(result) == 256


class TestWindows1252:
    """Test Windows-1252 string decoder."""

    def test_valid_windows1252(self):
        """Test valid Windows-1252 strings."""
        # Basic ASCII
        assert xtce_codec.decode_windows_1252(b"Hello") == "Hello"

        # Windows-1252 specific characters in 128-159 range
        # These would be control characters in ISO-8859-1 but are printable in Windows-1252
        test_bytes = b"\x80\x82\x83\x84"  # € ‚ ƒ „
        result = xtce_codec.decode_windows_1252(test_bytes)
        assert len(result) == 4  # Should decode successfully


class TestUTF8:
    """Test UTF-8 string decoder."""

    def test_valid_utf8(self):
        """Test valid UTF-8 strings."""
        # Basic ASCII
        assert xtce_codec.decode_utf8(b"Hello") == "Hello"

        # Multi-byte UTF-8
        assert xtce_codec.decode_utf8("Hello, 世界!".encode("utf-8")) == "Hello, 世界!"
        assert xtce_codec.decode_utf8("Café".encode("utf-8")) == "Café"

        # Emoji
        assert xtce_codec.decode_utf8("🚀".encode("utf-8")) == "🚀"

        # Empty string
        assert xtce_codec.decode_utf8(b"") == ""

    def test_invalid_utf8(self):
        """Test invalid UTF-8 sequences."""
        try:
            xtce_codec.decode_utf8(b"\xff\xfe")  # Invalid UTF-8
            assert False, "Should have raised an error"
        except ValueError:
            pass

        try:
            xtce_codec.decode_utf8(b"\x80")  # Invalid start byte
            assert False, "Should have raised an error"
        except ValueError:
            pass


class TestUTF16:
    """Test UTF-16 string decoders."""

    def test_utf16_with_bom(self):
        """Test UTF-16 with BOM detection."""
        # UTF-16LE with BOM
        text = "Hello"
        utf16le_with_bom = b"\xff\xfe" + text.encode("utf-16le")
        assert xtce_codec.decode_utf16(utf16le_with_bom) == text

        # UTF-16BE with BOM
        utf16be_with_bom = b"\xfe\xff" + text.encode("utf-16be")
        assert xtce_codec.decode_utf16(utf16be_with_bom) == text

    def test_utf16_without_bom(self):
        """Test UTF-16 without BOM (assumes big-endian)."""
        text = "Hi"
        utf16be_no_bom = text.encode("utf-16be")
        assert xtce_codec.decode_utf16(utf16be_no_bom) == text

    def test_utf16le(self):
        """Test UTF-16LE specifically."""
        text = "Hello, 世界!"
        utf16le_bytes = text.encode("utf-16le")
        assert xtce_codec.decode_utf16le(utf16le_bytes) == text

    def test_utf16be(self):
        """Test UTF-16BE specifically."""
        text = "Hello, 世界!"
        utf16be_bytes = text.encode("utf-16be")
        assert xtce_codec.decode_utf16be(utf16be_bytes) == text

    def test_utf16_odd_bytes(self):
        """Test UTF-16 with odd number of bytes (should fail)."""
        try:
            xtce_codec.decode_utf16(b"ABC")  # 3 bytes, not even
            assert False, "Should have raised an error"
        except ValueError as e:
            assert "even number of bytes" in str(e)


class TestUTF32:
    """Test UTF-32 string decoders."""

    def test_utf32_with_bom(self):
        """Test UTF-32 with BOM detection."""
        # UTF-32LE with BOM
        text = "Hi"
        utf32le_with_bom = b"\xff\xfe\x00\x00" + text.encode("utf-32le")
        assert xtce_codec.decode_utf32(utf32le_with_bom) == text

        # UTF-32BE with BOM
        utf32be_with_bom = b"\x00\x00\xfe\xff" + text.encode("utf-32be")
        assert xtce_codec.decode_utf32(utf32be_with_bom) == text

    def test_utf32le(self):
        """Test UTF-32LE specifically."""
        text = "Hello!"
        utf32le_bytes = text.encode("utf-32le")
        assert xtce_codec.decode_utf32le(utf32le_bytes) == text

    def test_utf32be(self):
        """Test UTF-32BE specifically."""
        text = "Hello!"
        utf32be_bytes = text.encode("utf-32be")
        assert xtce_codec.decode_utf32be(utf32be_bytes) == text

    def test_utf32_invalid_length(self):
        """Test UTF-32 with length not multiple of 4."""
        try:
            xtce_codec.decode_utf32(b"ABCDE")  # 5 bytes, not multiple of 4
            assert False, "Should have raised an error"
        except ValueError as e:
            assert "multiple of 4 bytes" in str(e)
