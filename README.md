# xtce-codec
Handles all encoding and decoding types and structures defined in the XTCE standard.

**Alpha Software:** This package is currently in the alpha stage of development. The API is not stable and is subject to breaking changes in future releases. Please use with caution and report any bugs or issues you find.

# Supported Encoding Types

## Integer
| Type                        | Bits   |
| --------------------------- | ------ |
| Unsigned                    | 1 - 64 |
| Sign-Magnitude              | 1 - 64 |
| Ones' Complement            | 1 - 64 |
| Two's Complement            | 1 - 64 |
| Binary Coded Decimal        | 1 - 64 |
| Packed Binary Coded Decimal | 1 - 64 |

## Float
| Type          | Bits             |
| ------------- | ---------------- |
| IEEE754       | 16, 32, 64, 128* |
| IEEE754-1985  | 32, 64, 80*      |
| MIL-STD-1750A | 32, 48           |
| DEC           | 32, 64           |
| IBM           | 32, 64           |
| TI            | 32, 40           |

(* Python floats are f64, so f128 and f80 will be cast down to f64 and some precision will be lost)

## String
| Type         |
| ------------ |
| US-ASCII     |
| ISO-8859-1   |
| Windows-1252 |
| UTF-8        |
| UTF-16       |
| UTF-16LE     |
| UTF-16BE     |
| UTF-32       |
| UTF-32LE     |
| UTF-32BE     |
