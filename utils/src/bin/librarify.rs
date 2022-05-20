/*!
Generate a library file of `FontData`.

Usage: `librarify <filename.json>`

This program will read a series of font names and sizes from the standard
input, then write a JSON file of a serialized
`HashMap<String, HashMap<u16, FontData>>`
that contains a library of font information suitable for transferring to
a system that may not have the given fonts installed.

The input format is one font per line, with the font name followed by a
comma, then the list of pixel sizes for that font to be rendered in:

```text
Inconsolata, 8 9 10 12 16 18 24
Liberation Mono, 8 9 10 12 16 18 24
Terminus, 8 9 10 12 24

...etc.
```

The produced JSON data will have the following format:

```json
{
    "Inconsolata": {
        8: FontData { ... },
        9: FontData { ... },
        ...etc.
    },
    "Liberation Mono" {
        8: FontData { ... },
        9: FontData { ... },
        ...etc.
    },

    ...etc.
}
```

The program will also produce, on the standard output, a list of font
names generated. Fontconfig's matchy algorithm is weird and might not
always produce the match you want (and you might also ask for a font
that isn't installed on your system), so this serves as an easy way
to check whether you've gotten the fonts you want.
*/
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::File;
use std::io::{BufRead, Write};

use ascii_art::*;
use fontconfig::{Fontconfig, Pattern};

const DEFAULT_OUTFILE: &str = "fonts.json";

/**
This is a hack to simplify error reporting. It allows any error
type that implements `Display` to be bubbled up and printed nicely
for the user by just returning it from main().
*/
struct ErrorShim(String);

impl std::fmt::Debug for ErrorShim {
    /// Implementing `Debug` allows the `ErrorShim` to be printed nicely
    /// for the user by just returning it as a `Result` `Err()` variant
    /// from the `main()` function.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl<D> From<D> for ErrorShim
where
    D: std::fmt::Display,
{
    fn from(d: D) -> Self {
        ErrorShim(format!("{}", &d))
    }
}

/**
Parse a line of input, and return a (font name, vector of desired sizes)
tuple (or an explanatory error).
*/
fn parse_input_line(line: &str) -> Result<(String, Vec<u16>), String> {
    let line_split: Vec<&str> = line.split(',').collect();
    let (name, size_string) = match line_split[..] {
        [name, size_string] => (name.trim(), size_string),
        _ => {
            return Err("improper input format".to_string());
        }
    };

    if name.is_empty() {
        return Err("no valid font name".to_string());
    }

    let sizes: Vec<u16> = size_string
        .split(char::is_whitespace)
        .map(|s| s.parse::<u16>())
        .filter(|res| res.is_ok())
        .map(|res| res.unwrap())
        .collect();

    if sizes.is_empty() {
        return Err("no valid font sizes".to_string());
    }

    Ok((String::from(name), sizes))
}

/**
Given a user-supplied font name, return a tuple of
(actual font name, font file path) for fontconfig's best guess at a match
(or an explanatory string if unsuccessful).
*/
fn get_fc_font_info(fc: &Fontconfig, name: &str) -> Result<(String, String), &'static str> {
    let cname = CString::new(name).unwrap();
    let family = CString::new("family").unwrap();
    let mut orig_pattern = Pattern::new(fc);
    orig_pattern.add_string(&family, &cname);
    let pattern = orig_pattern.font_match();

    let actual_name = match pattern.name() {
        None => {
            return Err("no matching font name");
        }
        Some(s) => String::from(s),
    };
    let font_path = match pattern.filename() {
        None => {
            return Err("no matching font path");
        }
        Some(s) => String::from(s),
    };

    Ok((actual_name, font_path))
}

/**
Given a font file path, a slice of pixel sizes, and a set of characters to
use to make generate the `FontData`, return a `HashMap<u16, FontData>` with
the sizes as keys. The second element of the returned tuple is a list of
error messgaes produced (if any).
*/
fn make_sized_data_for_font(
    fname: &str,
    sizes: &[u16],
    chars: &[char],
) -> (HashMap<u16, FontData>, Vec<String>) {
    let mut map: HashMap<u16, FontData> = HashMap::new();
    let font_bytes = match std::fs::read(fname) {
        Ok(v) => v,
        Err(e) => {
            return (
                map,
                vec![format!("Unable to open file \"{}\": {}.", fname, &e)],
            );
        }
    };
    let mut errs: Vec<String> = Vec::new();

    for siz in sizes.iter() {
        match FontData::from_font_bytes(&font_bytes, *siz as f32, chars) {
            Err(e) => {
                let estr = format!("\"{}\" at size {}: {}", fname, *siz, &e);
                errs.push(estr);
            }
            Ok(res) => {
                let fd = match res {
                    Ok(fd) => fd,
                    Err((fd, bads)) => {
                        let estr =
                            format!("\"{}\" at size {}: no coverage of {:?}", fname, *siz, &bads);
                        errs.push(estr);
                        fd
                    }
                };
                map.insert(*siz, fd);
            }
        }
    }

    (map, errs)
}

fn main() -> Result<(), ErrorShim> {
    let fc = Fontconfig::new().expect("Unable to initialize fontconfig.");

    let outfile = match std::env::args().nth(1) {
        None => {
            println!(
                "No filename specified, using default \"{}\".",
                &DEFAULT_OUTFILE
            );
            String::from(DEFAULT_OUTFILE)
        }
        Some(fname) => fname,
    };

    // Will hold the font names specified by the user and the actual font
    // names from the fontconfig match results.
    let mut font_name_pairs: Vec<(String, String)> = Vec::new();
    // Perhaps in the future I'll add an option to use a different set
    // of characters.
    let chars = printable_ascii();
    // Holds all the important data we're generating; will ultimately
    // get serialized.
    let mut main_map: HashMap<String, HashMap<u16, FontData>> = HashMap::new();

    for (line_n, line) in std::io::stdin().lock().lines().enumerate() {
        // If there is an error in an input line, just go ahead and die.
        let line = line.unwrap();

        // Ignore empty lines.
        if line.is_empty() {
            continue;
        }

        let (name_str, sizes) = match parse_input_line(&line) {
            Err(e) => {
                eprintln!("Error in input line {}: {}", &line_n, &e);
                continue;
            }
            Ok((name_str, sizes)) => (name_str, sizes),
        };

        let (actual_name, fname) = match get_fc_font_info(&fc, &name_str) {
            Err(e) => {
                eprintln!(
                    "Error from input line {} (font \"{}\"): {}",
                    line_n, &name_str, &e
                );
                continue;
            }
            Ok((name, filename)) => (name, filename),
        };

        let (map, mut errs) = make_sized_data_for_font(&fname, &sizes, &chars);
        if map.is_empty() {
            for err in errs.drain(..) {
                eprintln!("Error from input line {}: {}", &line_n, &err);
            }
            eprintln!(
                "Error from input line {}: {} (from \"{}\") produced no useable data.",
                &line_n, &actual_name, &name_str
            );
            continue;
        }

        for err in errs.drain(..) {
            eprintln!("Error from input line {}: {}", &line_n, &err);
        }
        main_map.insert(actual_name.clone(), map);
        font_name_pairs.push((name_str, actual_name));
    }

    if main_map.is_empty() {
        println!("No useable data generated; no output file written.");
    } else {
        println!();
        for (user_name, fc_name) in &font_name_pairs {
            println!("{} <= \"{}\"", fc_name, user_name);
        }

        let mut f = File::create(&outfile)?;
        serde_json::to_writer(&mut f, &main_map)?;
        f.flush()?;
    }

    Ok(())
}
