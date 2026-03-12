"""Unit tests for the unpacking function."""

import pytest

import xtce_codec


class TestUnpacking:
    """Tests for the unpacking function."""

    def test_unpacking_recipe(self):
        result = xtce_codec.unpack_parameters(
            packet_bytes=b"\x01\x02\x03\x04",
            offset_bits=0,
            recipe=[
                {
                    "name": "parameter1",
                    "encoding": "unsigned",
                    "bits": 8,
                    "byte_order": "big",
                    "reverse_bits": False,
                },
                {
                    "name": "parameter2",
                    "encoding": "unsigned",
                    "bits": 8,
                    "byte_order": "big",
                    "reverse_bits": False,
                    "offset_bits": 16,
                },
                {
                    "name": "parameter4",
                    "encoding": "unsigned",
                    "bits": 8,
                    "byte_order": "big",
                    "reverse_bits": False,
                },
            ],
        )

        print(result)

    def test_unpacking_strings(self):
        result = xtce_codec.unpack_parameters(
            packet_bytes=b"abcdefghijklmnop",
            offset_bits=0,
            recipe=[
                {
                    "name": "parameter1",
                    "encoding": "us_ascii",
                    "bits": 16,
                    "byte_order": "little",
                    "reverse_bits": False,
                },
            ],
        )

        print(result)


if __name__ == "__main__":
    test = TestUnpacking()
    test.test_unpacking_strings()
