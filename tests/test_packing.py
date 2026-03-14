"""Unit tests for the packing function."""

import pytest

import xtce_codec


class TestPacking:
    """Tests for the packing function."""

    def test_packing_recipe(self):
        result = xtce_codec.pack_parameters(
            recipe=[
                {
                    "name": "parameter1",
                    "value": 1,
                    "encoding": "unsigned",
                    "bits": 11,
                    "byte_order": "big",
                    "reverse_bits": False,
                },
                {
                    "name": "parameter1",
                    "value": 1,
                    "encoding": "unsigned",
                    "bits": 8,
                    "byte_order": "big",
                    "reverse_bits": False,
                },
                {
                    "name": "parameter2",
                    "value": 2,
                    "encoding": "unsigned",
                    "bits": 8,
                    "byte_order": "big",
                    "reverse_bits": False,
                },
                {
                    "name": "parameter4",
                    "value": 4,
                    "encoding": "unsigned",
                    "bits": 8,
                    "byte_order": "big",
                    "reverse_bits": False,
                },
            ],
        )

        print("Result bytes:", result)
        print("Result hex:", result.hex())
        print("Result binary:", " ".join(f"{b:08b}" for b in result))

    def test_packing_strings(self):
        result = xtce_codec.pack_parameters(
            recipe=[
                {
                    "name": "parameter1",
                    "value": "ab",
                    "encoding": "us_ascii",
                    "bits": 16,
                    "byte_order": "little",
                    "reverse_bits": False,
                },
            ],
        )

        print(result)

    def test_utf8_string_is_zero_padded_not_repeated(self):
        result = xtce_codec.pack_parameters(
            recipe=[
                {
                    "name": "payload",
                    "value": "TEST_STRING",
                    "encoding": "utf8",
                    "bits": 512,
                    "byte_order": "big",
                    "reverse_bits": False,
                },
            ],
        )

        assert len(result) == 64
        assert result[:11] == b"TEST_STRING"
        assert result[11:] == b"\x00" * 53

    def test_binary_encoding_masks_to_field_width(self):
        result = xtce_codec.pack_parameters(
            recipe=[
                {
                    "name": "payload",
                    "value": b"\xff\xaa",
                    "encoding": "binary",
                    "bits": 12,
                    "byte_order": "big",
                    "reverse_bits": False,
                },
            ],
        )

        assert result == b"\xfa\xa0"

    def test_binary_encoding_respects_little_endian_byte_order(self):
        result = xtce_codec.pack_parameters(
            recipe=[
                {
                    "name": "payload",
                    "value": b"\x12\x34",
                    "encoding": "binary",
                    "bits": 16,
                    "byte_order": "little",
                    "reverse_bits": False,
                },
            ],
        )

        assert result == b"\x34\x12"

    def test_binary_encoding_reverses_bits_within_width(self):
        result = xtce_codec.pack_parameters(
            recipe=[
                {
                    "name": "payload",
                    "value": b"\xb0",
                    "encoding": "binary",
                    "bits": 8,
                    "byte_order": "big",
                    "reverse_bits": True,
                },
            ],
        )

        assert result == b"\x0d"


if __name__ == "__main__":
    test = TestPacking()
    # test.test_packing_strings()
    test.test_packing_recipe()
