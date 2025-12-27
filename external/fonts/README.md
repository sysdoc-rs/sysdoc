# Liberation Sans Fonts

This directory contains Liberation Sans TrueType fonts used by sysdoc for PDF generation.

## Font Files

- `LiberationSans-Regular.ttf` - Regular weight
- `LiberationSans-Bold.ttf` - Bold weight
- `LiberationSans-Italic.ttf` - Italic style
- `LiberationSans-BoldItalic.ttf` - Bold italic style

## License

These fonts are licensed under the **SIL Open Font License, Version 1.1**.

See `LICENSE` file in this directory for the full license text.

## Source

Liberation Fonts version 2.1.5
- Project: https://github.com/liberationfonts/liberation-fonts
- Release: https://github.com/liberationfonts/liberation-fonts/releases/tag/2.1.5

## Usage

The fonts are embedded directly into the sysdoc binary using Rust's `include_bytes!` macro. This allows PDF generation without requiring external font files at runtime.

## Copyright

Copyright (c) 2007 Red Hat, Inc.
Copyright (c) 2012, 2014 Google, Inc.

## Reserved Font Names

The following font names are reserved:
- Liberation
- Arimo
- Tinos
- Cousine

## Redistribution

These fonts may be freely redistributed under the terms of the SIL Open Font License. The license permits:

- ✅ Bundling with software applications
- ✅ Embedding in documents and applications
- ✅ Commercial and non-commercial use
- ✅ Modification and derivative works

See LICENSE file for complete terms and conditions.
