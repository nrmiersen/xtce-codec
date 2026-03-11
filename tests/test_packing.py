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
                    "encoding": "us_ascii",
                    "bits": 16,
                    "byte_order": "little",
                    "reverse_bits": False,
                },
            ],
            values={
                "parameter1": "ab",
            },
        )

        print(result)


if __name__ == "__main__":
    test = TestPacking()
    # test.test_packing_strings()
    test.test_packing_recipe()
