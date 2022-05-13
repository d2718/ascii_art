/*!
A command-line filter utility for turning image files into ASCII art.

```text
user@system:/path$ img2ascii -h
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

By default this will read image data from stdin and write the rendered
text to stdout.

```text
$ img2ascii <rust-social-sm.jpg
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@  @W@@*  @N@@!` W@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@@@  @@@]    M`    !@    @@@N `@@@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@W@@@@`                            Z@@@@N@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@`  `~             W@N@             e` ``@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@N                  @@WN   `              @@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@^ ` ``          @M@@@@`  `   N@N@@NZ              `@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@N           WM@@@@@@@@@@  `NN@@@@@@@@@W"`          @@@@@@@@@@@@@@@@
@@@@@@@@@@NN@M@M`       $@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@N        `@@M@@@@@@@@@@@@@
@@@@@@@@@@N`          @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@Mj         `WN@@@@@@@@@@
@@@@@@@@@@@@                                         @@@@@@@N``       @@@@@@@@@@@@
@@@@@@@@NNMr`                                           @@@@@N6`      L@W@@@@@@@@@
@@@@@@@@                                                  @@@@@          `@@@@@@@@
@@@@@@@@@                                                 !@@@r         W@@@@@@@@@
@@@@@@@@N.   y@@@* `@@@            @@@@@@@@@@W            `@@y  W@@@    }W@N@@@@@@
@@@@@@       `N@@    N@            @@@@@@@@@@@N           N@N   @@@@       `N@@@@@
@@@@@@@u         ``@N@@            @@@@@@@@W@`           @@@@W            @N@@@@@@
@@@@@@@@W     `@@M@@@@@                               `}@N@@@@@@@@+`    `M@@@@@@@@
@@@@@W         @@@@@@@@                              `@@@@@@@@@@@MM       ``N@@@@@
@@@@@W         @@@@@@@@                                 @@@@@@@@@MM       ``N@@@@@
@@@@@@@@N      M@@@@@@@            @@@@@@@M@`            N@@@@@N        `M@@@@@@@@
@@@@@@Ny       M@@@@@@@            @@@@@@@@@@`           T@@@@@@          @N@@@@@@
@@@@@@         `N@@@@@@            @@@@@@@@@@N            `@@N`            `N@@@@@
@@@@@@NMN.                               ~@@@@`                         TW@W@@@@@@
@@@@@@@@@                                ~@@@@@                         W@@@@@@@@@
@@@@@@@@`                                ~@@@@@N                         `@@@@@@@@
@@@@@@@@NNN*`                            ~@@@@@@@`           `        L@WN@@@@@@@@
@@@@@@@@@@@@       ` @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@N         @@@@@@@@@@@@
@@@@@@@@@@N`                `@@@@@@@@@@@@@@@@@@@@@@@M                `WN@@@@@@@@@@
@@@@@@@@@@@@NN@@      `W@@N `N@@@@@@@@@@@@@@@@@@@@@@` `@@@        @@NN@@@@@@@@@@@@
@@@@@@@@@@@@@@@N       @@@@   N@@@@@@@@@@@@@@@@@@@@N  W@@@"       @@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@" `              wN@MNN@@@@@@N@N@N,               `@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@N                 `   ``                 @@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@`  `~                              w` ``@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@N@@@@`                            Z@@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@@@  @@@]    N`    !N    @@MN `@@@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@  @W@@!  @N@@*` W@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@
```

`img2ascii` dynamically loads `fontconfig` at runtime, and uses it to make a
best guess about which of the fonts installed on your system that you want to
use based on the font name you provide:

```text
$ img2ascii -s rust-social-sm.jpg -d rust-social-sm.txt -f "Anonymous Pro" -p 16
````
*/
use std::fmt::{Debug, Display, Formatter};
use std::io::{BufReader, Cursor, Read, Seek, Write};

use ascii_art::{FontData, Image};
use clap::Parser;

/**
This is a hack to simplify error propagation and reporting.

All possible errors get cast to `ErrorShim`s, and then printed
semi-nicely for the user. If, like, an error can ever be
semi-nice.
*/
struct ErrorShim(String);

impl<D> From<D> for ErrorShim
where
    D: Display,
{
    fn from(d: D) -> Self {
        let s = format!("{}", &d);
        Self(s)
    }
}

impl Debug for ErrorShim {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

/**
Struct used by clap to generate the CLI parsing code, and also
to hold the arguments passed from the command line.
*/
#[derive(Parser, Debug)]
#[clap(
    name = "img2ascii",
    version,
    author,
    about = "Command-line utility to turn image files into ASCII art.")]
struct Args {
    /// image path [default: read from stdin]
    #[clap(short, long)]
    source: Option<String>,

    /// output path [default: write to stdout]
    #[clap(short, long)]
    dest: Option<String>,

    /// font to use
    #[clap(short, long, default_value = "mono")]
    font: String,

    /// font size in pixels
    #[clap(short, long, default_value = "12.0")]
    pixels: f32,
}

/**
Marker trait indicating that the type implements both `Read` and `Seek`.

This is necessary because `dyn T + U` isn't possible unless `U` is an
auto trait, and we desire a `Box<dyn T>` where `T` is both
`Read` and `Seek`.
*/
trait Reread: Read + Seek {}
impl Reread for std::fs::File {}
impl<T: AsRef<[u8]>> Reread for std::io::Cursor<T> {}

/**
Struct returned by the `configure()` function (below). Holds pointers
to the image input stream, the text output stream, and the font information
to turn the data in the former into data in the latter.
*/
struct Cfg {
    /// image input stream (file or contents of stdin)
    source: Box<dyn Reread>,
    /// text output stream (file or stdout)
    dest: Box<dyn Write>,
    /// data from specified (or default) font
    font: FontData,
}

/**
Arrange the font data, and the input and output streams according to the
arguments supplied by the user; return a `Cfg` struct with these things.
*/
fn configure() -> Result<Cfg, ErrorShim> {
    use fontconfig::{Fontconfig, Pattern};
    use std::ffi::CString;
    use std::fs::File;

    let args = Args::parse();

    let source: Box<dyn Reread> = match args.source {
        Some(path) => {
            let f = File::open(path)?;
            Box::new(f)
        }
        None => {
            // Huff in the entirety of stdin and return a `Cursor` over that
            // buffer, because stdin can't be `Seek`.
            let mut v: Vec<u8> = Vec::new();
            std::io::stdin().lock().read_to_end(&mut v)?;
            Box::new(Cursor::new(v))
        }
    };

    let dest: Box<dyn Write> = match args.dest {
        Some(path) => {
            let f = File::create(path)?;
            Box::new(f)
        }
        None => Box::new(std::io::stdout()),
    };

    let fc = match Fontconfig::new() {
        Some(fc) => fc,
        None => {
            let estr = format!("Unable to initialize fontconfig.");
            return Err(ErrorShim(estr));
        }
    };
    let mut pattern = Pattern::new(&fc);
    let family = CString::new("family")?;
    let family_name = CString::new(args.font.clone().into_bytes())?;
    pattern.add_string(&family, &family_name);
    let pattern = pattern.font_match();

    let font_path = match pattern.filename() {
        Some(p) => p,
        None => {
            let estr = format!(
                "Unable to find matching font file for font \"{}\".",
                &args.font
            );
            return Err(ErrorShim(estr));
        }
    };

    let mut font_bytes: Vec<u8> = Vec::new();
    let mut f = File::open(&font_path)?;
    f.read_to_end(&mut font_bytes)?;

    let chars = ascii_art::printable_ascii();
    let font = match FontData::from_font_bytes(&font_bytes, args.pixels, &chars) {
        Err(e) => {
            let estr = format!("Error reading font file: {:?}", &e);
            return Err(ErrorShim(estr));
        }
        Ok(Ok(fd)) => fd,
        Ok(Err((fd, _))) => fd,
    };

    Ok(Cfg { source, dest, font })
}

fn main() -> Result<(), ErrorShim> {
    let cfg = configure()?;

    let img_reader = BufReader::new(cfg.source);
    let image = Image::auto(img_reader)?;

    ascii_art::write(&image, &cfg.font, cfg.dest)?;
    Ok(())
}
