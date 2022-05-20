/*!
Represent images in common image formats with text.

This has been historically called "ASCII art", and we'll go ahead and keep
calling it that, but this crate works with arbitrary Unicode characters
as long as the font involved has glyphs for them.

Rendering an image as text requires three steps:

  1. Choose a set of characters (code points/glyphs/whatever you want to call
     them; we'll use these kind of interchangably here) and a font file to
     load and analyze at a specific pixel size to generate a mapping from
     pixel intensity to character/code point/glyph/whatever.

```
let chars_to_use = ascii_art::printable_ascii();

let bytes = std::fs::read("test/LiberationMono-Regular.ttf").unwrap();
let font = ascii_art::FontData::from_font_bytes(
    &bytes,
    12.0,
    &chars_to_use
).unwrap().unwrap();    // Yep, it's a nested `Result`.
# let image_file = std::fs::File::open("test/griffin_sm.jpg").unwrap();
# let mut image_file = std::io::BufReader::new(image_file);
# let image = ascii_art::Image::auto(&mut image_file).unwrap();
# let mut stdout = std::io::stdout();
# ascii_art::write(&image, &font, &mut stdout).unwrap();
```

  2. Load an image file and convert it into a format this crate can use.

```
# let chars_to_use = ascii_art::printable_ascii();
#
# let bytes = std::fs::read("test/LiberationMono-Regular.ttf").unwrap();
# let font = ascii_art::FontData::from_font_bytes(
#     &bytes,
#     12.0,
#     &chars_to_use
# ).unwrap().unwrap();    // Yep, it's a nested `Result`.
let image_file = std::fs::File::open("test/griffin_sm.jpg").unwrap();
let mut image_file = std::io::BufReader::new(image_file);
let image = ascii_art::Image::auto(&mut image_file).unwrap();
# let mut stdout = std::io::stdout();
# ascii_art::write(&image, &font, &mut stdout).unwrap();
```

  3. Combine the two to make some art.

```
# let chars_to_use = ascii_art::printable_ascii();
#
# let bytes = std::fs::read("test/LiberationMono-Regular.ttf").unwrap();
# let font = ascii_art::FontData::from_font_bytes(
#     &bytes,
#     12.0,
#     &chars_to_use
# ).unwrap().unwrap();    // Yep, it's a nested `Result`.
# let image_file = std::fs::File::open("test/griffin_sm.jpg").unwrap();
# let mut image_file = std::io::BufReader::new(image_file);
# let image = ascii_art::Image::auto(&mut image_file).unwrap();
let mut stdout = std::io::stdout();
ascii_art::write(&image, &font, &mut stdout).unwrap();
```

# Features

`ascii_art` depends on the [`image`](https://docs.rs/image/latest/image/)
crate for decoding image files and resizing images. `image` supports
quite a few image formats, several of which are somewhat unusual or
special-purpose. `ascii_art` disables these less-common formats by default,
but they can be explicitly enabled (or the more-common formats can be
explicitly disabled) as features.

The following features (and formats) are enabled by default, and must
be explicitly disabled (with the `--no-default-features` option or
`default_features = false` in your manifest):
  * `bmp`
  * `gif`
  * `ico`
  * `jpeg`
  * `png`
  * `pnm`
  * `tiff`
  * `webp`

The following features must be explicitly enabled:
  * `dds` DirectDraw Surface container format
  * `farbfeld`
  * `hdr` Radiance HDR images
  * `openexr`
  * `tga` TARGA (Truevision TGA)

Finally, enabling the `rayon` feature will enable `image`'s support for
multithreaded JPEG decoding.
*/

use std::cmp::Ordering;
use std::io::{BufRead, BufWriter, Read, Seek, Write};

use ab_glyph::{Font, FontRef, ScaleFont};
use image::{
    imageops::{resize, FilterType},
    ImageBuffer, Luma,
};
use serde_derive::{Deserialize, Serialize};

const SPACE: char = ' ';
const REPLACE: char = '�'; // unicode replacement character
const PRINTABLE_ASCII: std::ops::Range<u32> = 0x20..0x7f;

/**
Return a `Vec<char>` of the printable ASCII characters.

This includes 0x20, the space, which can be argued counts a "printable",
and is in any case a good character to use in this application.

This is a solid choice for an "ascii art" character set, as almost
any font aimed at Latin alphabets will include these glyphs.
*/
pub fn printable_ascii() -> Vec<char> {
    PRINTABLE_ASCII
        .into_iter()
        .map(|n| char::try_from(n).unwrap())
        .collect()
}

/**
Error type for errors produced by this crate.
*/
#[derive(Debug)]
pub enum Error {
    /// Indicates that the bytes of a font file can't be interpreted.
    /// (The most likely reason being that the file isn't actually a
    /// font file.)
    InvalidFontData,

    /// The font supplied doesn't cover _any_ of the glyphs for the
    /// characters supplied.
    NoUseableGlyphs,

    /// Something has gone wrong reading or writing data; the contained
    /// string should contain more details.
    IOError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Error::InvalidFontData => {
                write!(
                    f,
                    "Supplied buffer does not contain valid or recognizable font data."
                )
            }
            Error::NoUseableGlyphs => {
                write!(f, "FontData object contains no useable glyphs.")
            }
            Error::IOError(s) => {
                write!(f, "I/O error: {}", s)
            }
        }
    }
}

/*
Raw information about a glyph taken directly from `ab_glyph`.

Will later get turned into a `Char` with normalized coverage,
and the horizontal advance information will be soaked up into
the containing `FontData` struct.
*/
struct UnscaledChar {
    /* character value this glyph represents */
    chr: char,
    /*  Glyph "coverage" in total pixels used. This is a floating-point
    value because the font outline only partially covers some pixels
    (probably _most_ at normally-used screen sizes and resolutions). */
    cov: f32,
    /*  Horizontal advance of this character in pixels. The containing
    `FontData` will set its "width" in pixels to be the largest
    value of this that any of its glyphs contain (but for monospace
    fonts, these should all be the same). This is a floating-point
    value because fonts these days are all fancy and don't care
    about the rigid box structure of the pixels you're trying to
    represent them with. */
    adv: f32,
}

impl UnscaledChar {
    /*
    Get the data about the glyph for the given `chr` from the supplied
    `ab_glyph::ScaleFont`.
    */
    fn from_ab_glyph<F: Font>(chr: char, font: &dyn ScaleFont<F>) -> Option<UnscaledChar> {
        let scaled_glyph = font.scaled_glyph(chr);
        if scaled_glyph.id == font.glyph_id(REPLACE) {
            return None;
        }
        let adv = font.h_advance(scaled_glyph.id);
        if let Some(g) = font.outline_glyph(scaled_glyph) {
            let mut cov: f32 = 0.0;
            g.draw(|_, _, c| cov += c);
            Some(UnscaledChar { chr, cov, adv })
        } else {
            /*
            Evidently space characters don't have "outline glyphs"
            (presumably because there's nothing to draw), but we want
            to use them in our ASCII art anyway, so we'll just return
            an `UnscaledChar` with zero coverage.
            */
            if chr == SPACE {
                Some(UnscaledChar {
                    chr,
                    cov: 0.0f32,
                    adv,
                })
            } else {
                None
            }
        }
    }
}

/*
Represents a char/code point/glyph with its normalized (0.0 <= x <= 1.0)
coverage.
*/
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(from = "(char, f32)", into = "(char, f32)")]
struct Char {
    chr: char,
    val: f32,
}

impl Char {
    fn from_unscaled(usc: UnscaledChar, scale: f32) -> Self {
        Char {
            chr: usc.chr,
            val: usc.cov / scale,
        }
    }
}

impl PartialEq for Char {
    fn eq(&self, rhs: &Self) -> bool {
        self.val == rhs.val
    }
}

impl Eq for Char {}

impl PartialOrd for Char {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.val.partial_cmp(&other.val)
    }
}

impl Ord for Char {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl From<(char, f32)> for Char {
    fn from(tup: (char, f32)) -> Char {
        Char {
            chr: tup.0,
            val: tup.1,
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<(char, f32)> for Char {
    fn into(self) -> (char, f32) {
        (self.chr, self.val)
    }
}

/**
The `FontData` struct holds all the information about a font
(at a given size) to render an image in it: a mapping from
pixel intensity to characters, plus geometry data.
*/
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FontData {
    values: Vec<Char>,
    width: f32,
    height: f32,
    fudge_factor: f32,
}

impl FontData {
    /**
    Analyze a font at a given `size` to produce a `FontData` struct that
    maps pixel intensity to the given set of `chars`. `bytes` should
    contain a .ttf or .otf font file.

    The return value is a little wonky. If there is an actual error or
    the returned `FontData` would contain no actual characters, the outer
    `Result` will return an `Err(Error).` If the given font is successfully
    analyzed, the inner `Result` will be returned.

    ```
    # use ascii_art::{FontData, printable_ascii};

    let chars = printable_ascii();

    let not_a_font_file = std::fs::read("test/griffin_sm.jpg").unwrap();
    let result = FontData::from_font_bytes(
        &not_a_font_file,
        12.0,
        &chars
    );

    println!("{:?}", &result);
    // Err(InvalidFontData)

    let font_file = std::fs::read("test/LiberationMono-Regular.ttf")
        .unwrap();
    let result = FontData::from_font_bytes(
        &font_file,
        12.0,
        &chars
    );

    println!("{:?}", &result)
    // Ok(Ok(FontData { ... }))
    ```

    No font contains glyphs for all characters, so there is a chance that
    this function may be unable to use all of the supplied `chars` in
    the returned `FontData`'s map. If this is the case, the inner `Result`
    will be an `Err()` containing a tuple with the `FontData` struct as
    well as a `Vec` of the unused `chars`. If all the provided characters
    are used, then this will just be an `Ok(FontData)`.

    ```
    # use ascii_art::{FontData, printable_ascii};

    let mut chars = printable_ascii();
    // Now we'll add a bunch of characters for which Liberation Mono
    // doesn't have glyph coverage.
    for n in (0x1100u32..0x112fu32).into_iter() {
        let c: char = n.try_into().unwrap();
        chars.push(c);
    }

    let font_file = std::fs::read("test/LiberationMono-Regular.ttf")
        .unwrap();
    let inner_result = FontData::from_font_bytes(
        &font_file,
        12.0,
        &chars
    ).unwrap(); // We're getting the inner result.

    println!("{:?}", &inner_result);
    // Err((FontData { ... }, ['ᄀ', 'ᄁ', 'ᄂ', ... ]))
    ```
    */
    pub fn from_font_bytes(
        bytes: &[u8],
        size: f32,
        chars: &[char],
    ) -> Result<Result<FontData, (FontData, Vec<char>)>, Error> {
        let font = match FontRef::try_from_slice(bytes) {
            Ok(f) => f,
            Err(_) => {
                return Err(Error::InvalidFontData);
            }
        };
        let scaled_font = font.as_scaled(size);

        let mut reject_chars: Vec<char> = Vec::new();
        let mut charz: Vec<UnscaledChar> = Vec::with_capacity(chars.len());

        for c in chars.iter() {
            match UnscaledChar::from_ab_glyph(*c, &scaled_font) {
                None => {
                    reject_chars.push(*c);
                }
                Some(ch) => {
                    charz.push(ch);
                }
            }
        }

        if charz.is_empty() || (charz.len() == 1 && charz[0].chr == ' ') {
            return Err(Error::NoUseableGlyphs);
        }

        charz.sort_unstable_by(|a, b| a.cov.partial_cmp(&b.cov).unwrap());
        // The next couple of things seem hacky because floats aren't Ord.
        let max_cov = charz.last().unwrap().cov;
        let mut width: f32 = 0.0;
        for ch in charz.iter() {
            if ch.adv > width {
                width = ch.adv;
            }
        }
        if width == 0.0f32 {
            return Err(Error::NoUseableGlyphs);
        }

        let height = scaled_font.height() + scaled_font.line_gap();

        let values: Vec<Char> = charz
            .drain(..)
            .map(|ch| Char::from_unscaled(ch, max_cov))
            .collect();
        let fudge_factor: f32 = 1.0 / (values.len() as f32);

        let dat = FontData {
            values,
            width,
            height,
            fudge_factor,
        };

        if reject_chars.is_empty() {
            Ok(Ok(dat))
        } else {
            Ok(Err((dat, reject_chars)))
        }
    }

    /**
    Resize the `FontData`'s internal map to contain only the characters used
    by `n` equally-spaced intensities between 0.0 and 1.0.

    Any given font has clusters of glyphs that are very close together in
    coverage (for example `{ `O`, `0` }` or `{'1', '!', 'l' ,'I', '|'}`).
    When converting from an image format with a fixed number of equally-spaced
    intensity levels (like 256 levels of brightness), some characters in these
    clusters may never be used. This method trims out impossible-to-use
    characters from its receivers map, possibly reducing space usage and
    increasing look-up efficiency.
    */
    pub fn prune_for_n_intensities(&mut self, n: usize) {
        use std::collections::BTreeSet;
        let mut charz: BTreeSet<char> = BTreeSet::new();
        let nf = n as f32;
        for k in 0..n {
            let kf = (k as f32) / nf;
            charz.insert(self.pixel(kf));
        }
        self.values = self
            .values
            .drain(..)
            .filter(|c| charz.contains(&c.chr))
            .collect();
    }

    /**
    Return the character mapped to for a pixel with an intensity `val`.
    This is for rendering _light_ text on a _dark_ background (what these
    days is commonly called "night mode" text). For _dark_ text on a
    _light_ background "day mode), use the `.pixel_inv()` method.
    Intensities are assumed to be between 0.0 and 1.0; intensities outside
    that range will result in the minimum or maximum coverage character,
    respectively.
    */
    pub fn pixel(&self, val: f32) -> char {
        let val = val - self.fudge_factor;
        let dummy = Char { chr: ' ', val };

        let n = match &self.values.binary_search(&dummy) {
            Ok(n) => *n,
            Err(n) => *n,
        };

        self.values[n].chr
    }

    /**
    Return the character mapped to for a pixel with intensity `1.0 - val`.
    This is for rendering _dark_ text on a _light_ background, as opposed
    to the `.pixel()` method, which is for rendering _light_ text on a
    _dark_ background. As with `.pixel()`, intensities are assumed to be
    between 0.0 and 1.0; intensities outside that range will result in the
    minimum or maximum coverage character, respectively.
    */
    pub fn pixel_inv(&self, val: f32) -> char {
        let val = 1.0 - (val + self.fudge_factor);
        let dummy = Char { chr: ' ', val };

        let n = match &self.values.binary_search(&dummy) {
            Ok(n) => *n,
            Err(n) => *n,
        };

        self.values[n].chr
    }

    /// Return the width and height (in pixels) of a single character
    /// in this font and size.
    pub fn geometry(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    /**
    Serialize the receiver into a chunk of JSON.

    The intended use is for generating font data to be used on systems
    that don't have the given fonts installed.
    */
    pub fn serialize<W: Write>(&self, writer: W) -> Result<(), Error> {
        if let Err(e) = serde_json::to_writer(writer, self) {
            Err(Error::IOError(format!("{}", &e)))
        } else {
            Ok(())
        }
    }

    /**
    Deserialize some `FontData` that has been previously serialized
    with the `.serialize()` method.

    The intended use is for systems that don't have the target
    fonts installed.
    */
    pub fn deserialize<R: Read>(reader: R) -> Result<FontData, Error> {
        match serde_json::from_reader(reader) {
            Ok(fd) => Ok(fd),
            Err(e) => Err(Error::IOError(format!("{}", &e))),
        }
    }
}

pub use image::ImageFormat;

/**
Image data in a format useable by this crate: each pixel represented
as a normalized (0.0 <= x <= 1.0) intensity value.
*/
pub struct Image {
    buff: ImageBuffer<Luma<f32>, Vec<f32>>,
}

impl Image {
    /**
    Create a new `Image`, attempting to guess the format of the data
    in the `Read`er.
    */
    pub fn auto<R: BufRead + Seek>(r: R) -> Result<Image, Error> {
        let rdr = match image::io::Reader::new(r).with_guessed_format() {
            Err(e) => {
                let err = format!("{}", &e);
                return Err(Error::IOError(err));
            }
            Ok(x) => x,
        };
        let img = match rdr.decode() {
            Err(e) => {
                let err = format!("{}", &e);
                return Err(Error::IOError(err));
            }
            Ok(x) => x,
        };

        let img = img.to_luma32f();
        Ok(Image { buff: img })
    }

    /**
    Create a new `Image`, attempting to decode the data in `r` from the
    provided `format`.
    */
    pub fn with_format<R: BufRead + Seek>(r: R, format: ImageFormat) -> Result<Image, Error> {
        let img = match image::io::Reader::with_format(r, format).decode() {
            Err(e) => {
                let err = format!("{}", &e);
                return Err(Error::IOError(err));
            }
            Ok(x) => x,
        };

        let img = img.to_luma32f();
        Ok(Image { buff: img })
    }

    fn geometry(&self) -> (f32, f32) {
        let (w, h) = self.buff.dimensions();
        (w as f32, h as f32)
    }
}

/**
Given some `FontData`, write the `Image` as text to the `writer`.

This is for writing light text on a dark background (that is, pixel
intensity values are positively correlated with luminosity.)

This function looks at the geometry of the `font` and the geometry of the
`Image` and tries to output a rectangle of text that will match the size
of the original image. Depending on how the text is viewed, characters and
lines may have different amounts of spacing between them, resulting in
an imperfect size match.
*/
pub fn write<W: Write>(img: &Image, font: &FontData, writer: W) -> Result<(), Error> {
    let (img_wf, img_hf) = img.geometry();
    let (font_wf, font_hf) = font.geometry();
    let w = (img_wf / font_wf) as u32;
    let h = (img_hf / font_hf) as u32;
    let mut writer = BufWriter::new(writer);

    let resized = resize(&img.buff, w, h, FilterType::Nearest);
    for row in resized.rows() {
        for p in row {
            let g = font.pixel(p.0[0]);
            if let Err(e) = write!(&mut writer, "{}", g) {
                let err = format!("{}", &e);
                return Err(Error::IOError(err));
            }
        }
        if let Err(e) = writeln!(&mut writer) {
            let err = format!("{}", &e);
            return Err(Error::IOError(err));
        }
    }

    if let Err(e) = writer.flush() {
        Err(Error::IOError(format!("{}", &e)))
    } else {
        Ok(())
    }
}

/**
Given some `FontData`, write the `Image` as text to the `writer`.

This is for writing _dark_ text on a _light_ background (that is, pixel
intensity values are _negatively_ correlated with luminosity.)

This function looks at the geometry of the `font` and the geometry of the
`Image` and tries to output a rectangle of text that will match the size
of the original image. Depending on how the text is viewed, characters and
lines may have different amounts of spacing between them, resulting in
an imperfect size match.
*/
pub fn write_inverted<W: Write>(img: &Image, font: &FontData, writer: W) -> Result<(), Error> {
    let (img_wf, img_hf) = img.geometry();
    let (font_wf, font_hf) = font.geometry();
    let w = (img_wf / font_wf) as u32;
    let h = (img_hf / font_hf) as u32;
    let mut writer = BufWriter::new(writer);

    let resized = resize(&img.buff, w, h, FilterType::Nearest);
    for row in resized.rows() {
        for p in row {
            let g = font.pixel_inv(p.0[0]);
            if let Err(e) = write!(&mut writer, "{}", g) {
                let err = format!("{}", &e);
                return Err(Error::IOError(err));
            }
        }
        if let Err(e) = writeln!(&mut writer) {
            let err = format!("{}", &e);
            return Err(Error::IOError(err));
        }
    }

    if let Err(e) = writer.flush() {
        Err(Error::IOError(format!("{}", &e)))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;

    const FONT_PATH: &str = "test/LiberationMono-Regular.ttf";
    const IMAGE_PATH: &str = "test/griffin_sm.jpg";
    // a chunk of the Hangul Jamo block (not covered by Liberation Mono)
    const BAD_CHARS: std::ops::Range<u32> = 0x1100..0x112f;

    fn test_font(size: f32) -> FontData {
        let font_bytes = std::fs::read(FONT_PATH).unwrap();
        let chars = printable_ascii();
        FontData::from_font_bytes(&font_bytes, size, &chars)
            .unwrap()
            .unwrap()
    }

    fn assert_font_data_are_eq(lhs: &FontData, rhs: &FontData) -> Result<(), String> {
        if (lhs.width, lhs.height) != (rhs.width, rhs.height) {
            let e = format!(
                "geometries don't match: {} by {} != {} by {}",
                lhs.width, lhs.height, rhs.width, rhs.height
            );
            return Err(e);
        }
        if lhs.fudge_factor != rhs.fudge_factor {
            let e = format!(
                "fudge factors don't match: {} != {}",
                lhs.fudge_factor, rhs.fudge_factor
            );
            return Err(e);
        }

        for (n, (lhc, rhc)) in lhs.values.iter().zip(rhs.values.iter()).enumerate() {
            if lhc != rhc {
                let e = format!("map entries {} differ: {:?} != {:?}", n, lhc, rhc);
                return Err(e);
            }
        }

        Ok(())
    }

    #[test]
    fn make_font_data() {
        let data = test_font(12.0);
        println!("{:?}", &data);

        let font_bytes = std::fs::read(FONT_PATH).unwrap();
        // actually both good and bad chars`
        let bad_chars: Vec<char> = PRINTABLE_ASCII
            .into_iter()
            .chain(BAD_CHARS.into_iter())
            .map(|n| char::try_from(n).unwrap())
            .collect();
        match FontData::from_font_bytes(&font_bytes, 12.0f32, &bad_chars) {
            Err(e) => {
                println!("{:?}", &e)
            }
            Ok(res) => match res {
                Err((fd, bads)) => {
                    println!("rejected chars: {:?}", &bads);
                    println!("okay: {:?}", &fd);
                }
                Ok(fd) => {
                    println!("{:?}", &fd);
                }
            },
        };
    }

    #[test]
    fn get_pixels() {
        let data = test_font(12.0);
        let end = 256usize;
        let endf = end as f32;
        let charz: Vec<char> = (0usize..end)
            .into_iter()
            .map(|n| {
                let pix_val = (n as f32) / endf;
                data.pixel(pix_val)
            })
            .collect();
        println!("{:?}", &charz);
    }

    #[test]
    fn prune() {
        let mut font = test_font(12.0);
        println!("font has {} chars", font.values.len());
        font.prune_for_n_intensities(256);
        println!("font has {} chars", font.values.len());

        let mut font = test_font(12.0);
        println!("font has {} chars", font.values.len());
        font.prune_for_n_intensities(100);
        println!("font has {} chars", font.values.len());
    }

    #[test]
    fn serde() -> Result<(), String> {
        use std::io::Cursor;

        let font = test_font(12.0);
        let mut data: Vec<u8> = Vec::new();
        font.serialize(&mut data).unwrap();
        let mut cursor = Cursor::new(data);
        let deserialized = FontData::deserialize(&mut cursor).unwrap();
        assert_font_data_are_eq(&font, &deserialized).unwrap();
        Ok(())
    }

    #[test]
    fn load_image() {
        {
            let f = std::fs::File::open(IMAGE_PATH).unwrap();
            let mut f = BufReader::new(f);
            let i = Image::auto(&mut f).unwrap();
            println!("{:?}", &i.geometry());
        }
        {
            let f = std::fs::File::open(IMAGE_PATH).unwrap();
            let mut f = BufReader::new(f);
            let e = Image::with_format(&mut f, ImageFormat::Png);
            assert!(e.is_err());
            let e = unsafe { e.unwrap_err_unchecked() };
            println!("{:?}", &e)
        }
    }

    #[test]
    fn to_writer() {
        let mut v: Vec<u8> = Vec::new();

        let f = std::fs::File::open(IMAGE_PATH).unwrap();
        let mut f = BufReader::new(f);
        let img = Image::auto(&mut f).unwrap();

        let font = test_font(16.0);

        write(&img, &font, &mut v).unwrap();
        let outstring = String::from_utf8(v).unwrap();
        println!("{}", &outstring);
    }
}
