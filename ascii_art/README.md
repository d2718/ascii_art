# Rust crate `ascii_art`

a library for rendering images as text

This kind of thing has historically been called "ASCII art", and we'll go
ahead and keep using that term, but this crate works with any arbitrary
Unicode characters as long as the font involved has glyphs for them.

In `Cargo.toml`:

```toml
ascii_art = { git = "https://github.com/d2718/ascii_art/ascii_art" }
```

Then rendering an image as text requires three main steps:

  1. Choose a set of characters (or code points or whatever; the things in
     Rust that map to `char`s) and a font file to load and analyze at a
     specific pixel size to generate a mapping from pixel intensity to
     character/code point/glyph/whatever.

```rust
let chars_to_use = ascii_art::printable_ascii();

let font_bytes = std::fs::read("test/LiberationMono-Regular.ttf").unwrap();
let font = ascii_art::FontData::from_font_bytes(
    &font_bytes,
    12.0,
    &chars_to_use
).unwrap().unwrap() // It's a nested `Result`.
```

2. Load an image file and convert it into a format this crate understands.

```rust
let image_file = std::fs::File::open("test/griffin_sm.jpb").unwrap();
let mut image_file = std::io::BufReader::new(image_file);
let image = ascii_art::Image::auto(&mut image_file).unwrap();
```

3. Combine the two to make art (or at least, "art").

```rust
let mut stdout = std::io::stdout();
ascii_art::write(&image, &font, &mut stdout).unwrap();
```

# Features

`ascii_art` depends on the [`image`](https://docs.rs/image/latest/image/)
crate for decoding image files and resizing images. `image` supports quite
a few image formats, some of which are somewhat rare. `ascii_art` uses
only some of these by default, but you can choose exactly which formats
you want to support by enabling or disabling features.

The following features (and formats) are enabled by default, and must
be explicitly disabled:

  * `bmp`
  * `gif`
  * `ico`
  * `jpeg`
  * `png`
  * `pnm` (.pbm, .pgm, and .ppm "metaformat")
  * `tiff`
  * `webp`

The following features must be explicitly enabled:

  * `dds` (DirectDraw Surface container format)
  * [`farbfeld`](https://tools.suckless.org/farbfeld/) (the "suckless"
    image format, which seems pretty similar to .ppm P6 format)
  * `hdr` (Radiance HDR images)
  * `openexr` (ILM compositing graphics format)
  * `tga` (TARGA, Truevision TGA)

Finally, enabling the `rayon` feature will enable `image`'s multi-threading
JPEG codec.

# Plans

The intent is that future releases will include

  * ~~support for dark text on light background ("day mode" text)~~
    done is 0.3.0
  * convenience functions for reading/writing directly from/to
    stdin/stdout and files

and _possibly_

  * a way to auto-detect and use _all_ supported glyphs from a font
  * specification of font sizes in _points_ instead of just pixels

# License

[MIT](https://opensource.org/licenses/MIT)

# Credits

This crate would not be possible without, and its maintainer is grateful for,
the heavy lifting provided by

  * ['ab_glyph'](https://crates.io/crates/ab_glyph)
  * [`image`](https://crates.io/crates/image)
  * and, of course, [`serde`](https://serde.rs)
  