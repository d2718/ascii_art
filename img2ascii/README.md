`img2ascii`

A command-line filter utility for turning image files into ASCII art.

---

The `ascii_art` crate upon which this utility depends will actually
deal with arbitrary unicode text (as long as the font being used has
the required coverage) but this utility limits itself to "printable
ASCII" (the characters returned by `ascii_art::printable_ascii()`).

---

```text
$ img2ascii -h
img2ascii 0.2.0
Dan <dx2718@gmail.com>
Command-line utility to turn image files into ASCII art.
USAGE:
    img2ascii [OPTIONS]
OPTIONS:
    -d, --dest <DEST>        output path [default: write to stdout]
    -f, --font <FONT>        font to use [default: mono]
    -h, --help               Print help information
    -i, --invert             target inverted (dark on light) text
    -p, --pixels <PIXELS>    font size in pixels [default: 12.0]
    -s, --source <SOURCE>    image path [default: read from stdin]
    -V, --version            Print version information
```

`img2ascii` dynamically loads and queries Fontconfig at run time; it will
make its best guess as to which of the fonts installed on your system you
want to use.

## Anticipated questions

### Why isn't the image in the font I requested?

`img2ascii` writes out plain text (plain _ASCII_ text, in fact). The font
information is used to select the appropriate characters (and number of
characters) to use, but if you are viewing the output in your terminal or
a text editor, unless the font of your terminal/editor is set to the
target font, it won't display in that font.

### Why is the image so huge/tiny?

`img2ascii` attempts to match the dimensions of the original image in
the target font, at the target font size. If you are viewing it in a
different font, or at a different size than the target font, it will
be the wrong size. Also, if you give it a huge image, even if you're
viewing it in the target font, the resultant output textual image
will be huge. Reduce the size of your input image (or target a larger
font) in order to reduce the size of your output image.

### Why is the aspect ratio all screwy?

Again, `img2ascii` attempts to match the dimensions of the original
image in the target font. If you are viewing it in a font with a
different aspect ratio than the target font, it will appear stretched
or squished. For example, of this writing, I use
[Iosevka](https://github.com/be5invis/Iosevka) as my terminal font;
Iosevka is a tall, narrow font, so images rendered in most other
target fonts will appear too tall and narrow.

### Why is the default light-on-dark text and dark-on-light text considered _inverted_? Isn't most text dark-on-light? Are you some kind of edgelord dark-mode chauvanist trying to force your niche preference on the rest of us?

While I do generally prefer dark text on a light background, I specificially
chose light-on-dark to be the "forward" direction because that's the way
most pixel values are treated on the computer screen. A value of 0 tends to
be "no light" (and thus minimum brightness), and a value of 255 (or 65535
or 1.0 or whatever the maximum value of the format is) tends to be "full
light" (and thus maximum brightness). Thus, glyphs will less coverage are
treated as "darker", and glyphs with more coverage are treated as "lighter".