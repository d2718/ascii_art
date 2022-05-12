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
ascii_art
Command-line utility to turn image files into ASCII art.

USAGE:
    img2ascii [OPTIONS]

OPTIONS:
    -d, --dest <DEST>        output path [default: write to stdout]
    -f, --font <FONT>        font to use [default: mono]
    -h, --help               Print help information
    -p, --pixels <PIXELS>    font size in pixels [default: 12.0]
    -s, --source <SOURCE>    image path [default: read from stdin]
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