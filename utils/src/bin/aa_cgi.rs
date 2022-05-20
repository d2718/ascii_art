/*!
A CGI script for a web service that uses the `ascii_art` library to turn
uploaded images into "art".

Responds to two types of requests:

  * a GET or POST with the `aa-action` header value of `list`
  * a POST with the following headers
      + `aa-action: render`
      + `content-type: multipart/form-data; boundary=...` (&c on the boundary)
    and a multipart body with the following three parts:
      + `name="font"` (the font family name to use)
      + `name="size"` (the pixel size of the font to use)
      + `name="file"` (the image file to ASCII-ize)
    If a part with `name="invert"` exists and is true, then the image will be
    rendered for black text on a white background.

(It also respons to an OPTIONS request, but I'm not sure if that's necessary.)

An `aa-action: list` request will return a JSONized
`HashMap<String, Vec<u16>>` mapping the font names available to the sizes of
each font that the CGI program's font libraray can render.

An `aa-action: render` request will use the supplied information and return
a textual result: the rendered image.

You can see this program in action at
[`https://d2718.net/ascii_art/`](https://d2718.net/ascii_art/)

*/
use std::collections::HashMap;
use std::io::{BufReader, Cursor};

use dumb_cgi::{Request, EmptyResponse, FullResponse, Body};
use ascii_art::{FontData, Image};

/// Location of font data library.
const LIB_PATH: &str = "/home/dan/svc/ascii_art/fonts.json";

/**
Load, deserialize, and return the font data library.
*/
fn load_library() -> Result<HashMap<String, HashMap<u16, FontData>>, String> {
    let f = match std::fs::File::open(LIB_PATH) {
        Ok(f) => f,
        Err(e) => {
            return Err(format!("unable to open font lib: {}", &e));
        },
    };
    
    let lib: HashMap<String, HashMap<u16, FontData>>;
    lib = match serde_json::from_reader(&f) {
        Ok(x) => x,
        Err(e) => {
            return Err(format!("error deserializing font lib: {}", &e));
        },
    };
    
    Ok(lib)
}

/**
Given the value of the "content-disposition" header of a multipart/form-data
body part, return the form element name (if present).
*/
fn field_name_from_content_disposition<'a>(val: &'a str) -> Option<&'a str> {
    let start = match val.find("name=\"") {
        Some(n) => n + "name=\"".len(),
        None => { return None; }
    };
    let end = match val[start..].find('"') {
        Some(n) => start + n,
        None => { return None; }
    };
    
    Some(&val[start..end])
}

/**
Return the "human-readable" status message for a given HTTP response code.
*/
fn status_message(code: u16) -> &'static str {
    match code {
        200 => "OK",
        204 => "No Data",
        400 => "Bad Request",
        500 => "Internal Server Error",
        _ => "Unknown Status Type",
    }
}

/**
Respond with the given error code and explanation message.
*/
fn error_response(code: u16, message: &str) -> ! {

    log::debug!(
        "error response: {} ({}): {}",
        code,
        status_message(code),
        message
    );
    
    EmptyResponse::new(code)
        .with_content_type("text/plain")
        .with_body(message)
        .respond()
        .unwrap();
    
    std::process::exit(0);
}

/**
Respond to a preflight OPTIONS request.

This may not be strictly necessary, but I don't totally understand all
the nuances of modern HTTP.
*/
fn options_response() -> ! {
    EmptyResponse::new(204)
        .with_header("Access-Control-Allow-Methods", "OPTIONS")
        .with_header("Access-Control-Allow-Headers", "aa-action")
        .respond()
        .unwrap();
    
    std::process::exit(0);
}

/**
Respond with data specifying the fonts and sizes about which the CGI program
has data.

The data sent should be a JSONized map that maps font names to vectors of
pixel sizes available in the specified font.

```json
{
    "Deja Vu Sans Mono": [8, 9, 10, 12, 16, 18, 24, 36],
    "Droid Sans Mono": [8, 9, 10, 12, 16, 18, 24, 36],
    "Inconsolata": [8, 9, 10, 12, 16, 18, 24, 36],
    e
    t
    c
    .
    .
    .``
}
````
*/
fn list_response() -> ! {
    let lib = match load_library() {
        Ok(lib) => lib,
        Err(s) => {
            let estr = format!("Unable to load font library: {}", &s);
            error_response(500, &estr);
        }
    };
    
    let mut list_map: HashMap<String, Vec<u16>>;
    list_map = HashMap::with_capacity(lib.len());
    
    for (font_name, size_map) in lib.iter() {
        let font_name = String::from(font_name);
        let mut sizes: Vec<u16> = Vec::with_capacity(size_map.len());
        for (k, _) in size_map.iter() {
            sizes.push(*k);
        }
        list_map.insert(font_name, sizes);
    }
    
    let response_data: String = match serde_json::to_string_pretty(&list_map) {
        Ok(data) => data,
        Err(e) => {
            let estr = format!("Unable to serialze font list: {}", &e);
            error_response(500, &estr);
        },
    };

    let r = EmptyResponse::new(200)
        .with_content_type("text/json")
        .with_body(response_data);
    
    log::debug!(
        "Sending list response. Body:\n{}",
        // This is okay because ascii_art::write() should only write UTF-8
        std::str::from_utf8(r.get_body()).unwrap()
    );
    
    r.respond().unwrap();

    std::process::exit(0);
}

fn render_from_server_font(
    font_name: &str,
    size: u16,
    image: &Image,
    invert: bool
) -> Result<FullResponse, String> {
    let fonts = load_library()?;
        
    let family = fonts.get(font_name).ok_or(
        format!("No font data matching \"{}\".", font_name)
    )?;
    
    let font = family.get(&size).ok_or(
        format!("No data for font \"{}\" at size \"{}\".", font_name, size)
    )?;
    
    let write_f = if invert {
        ascii_art::write_inverted
    } else {
        ascii_art::write
    };
    
    let mut r = EmptyResponse::new(200)
        .with_content_type("text/plain");
    
    write_f(image, font, &mut r).or_else(|e| {
        Err(format!("Error writing text image: {}", &e))
    })?;
    
    Ok(r)
}

fn render_from_user_font(
    font_bytes: &[u8],
    size: u16,
    image: &Image,
    invert: bool
) -> Result<FullResponse, String> {
    let chars = ascii_art::printable_ascii();
    let size = size as f32;
    let font = FontData::from_font_bytes(font_bytes, size, &chars)
        .or_else(|e| Err(e.to_string()) )?.unwrap();
    
    let write_f = if invert {
        ascii_art::write_inverted
    } else {
        ascii_art::write
    };
    
    let mut r = EmptyResponse::new(200)
        .with_content_type("text/plain");
    
    write_f(image, &font, &mut r).or_else(|e| {
        Err(format!("Error writing text image: {}", &e))
    })?;
    
    Ok(r)
}

/**
Respond to a request to render an image,
*/
fn render_response(req: &dumb_cgi::Request) -> ! {
    let mut font_name: Option<String> = None;
    let mut font_file: Option<&[u8]>  = None;
    let mut user_supplied_font: bool = false;
    let mut size: Option<u16>    = None;
    let mut data: Option<&[u8]>  = None;
    let mut invert: bool         = false;
    
    let body_parts = match req.body() {
        Body::Multipart(v) => v,
        _ => { error_response(400, "Request is not multipart/form-data."); },
    };
    
    for part in body_parts.iter() {
        
        log::debug!("  Part:");
        for (k, v) in part.headers.iter() {
            log::debug!("    \"{}\": \"{}\"", &k, &v);
        }
        log::debug!("    {} byte value.", part.body.len());
        
        if let Some(val) = part.headers.get("content-disposition") {
            match field_name_from_content_disposition(val) {
                Some("font") => {
                    let font = String::from_utf8_lossy(&part.body)
                                        .trim().to_string();
                    font_name = Some(font);
                },
                Some("font_file") => {
                    font_file = Some(&part.body);
                }
                Some("font_source") => if part.body == "user".as_bytes() {
                    user_supplied_font = true;
                },
                Some("size") => {
                    match std::str::from_utf8(&part.body) {
                        Ok(s) => match s.trim().parse::<u16>() {
                            Ok(n) => {
                                size = Some(n)
                            },
                            Err(_) => {
                                error_response(400, "Unparseable \"size\" value.");
                            }
                        },
                        Err(_) => {
                            error_response(400, "\"size\" value not valid UTF-8.");
                        },
                    }
                },
                Some("file") => {
                    data = Some(&part.body); 
                },
                Some("invert") => {
                     if "true" == String::from_utf8_lossy(&part.body).to_string() {
                        invert = true;
                    }
                },
                _ => { /* Don't do anything. Why is this field being sent? */ },
            }
        }
    }
    
    
    //let font = font.unwrap_or_else(|| error_response(400, "Missing \"font\" value."));
    let size = size.unwrap_or_else(|| error_response(400, "Missing \"size\" value."));
    let data = data.unwrap_or_else(|| error_response(400, "Missing \"file\" value."));
    
    let mut image_reader = BufReader::new(Cursor::new(data));
    let image = match Image::auto(&mut image_reader) {
        Ok(img) => img,
        Err(e) => {
            let estr = format!("Error reading image data: {}", &e);
            error_response(400, &estr);
        }
    };
    
    let response = if user_supplied_font {
        let font_file = font_file.unwrap_or_else(||
            error_response(400, "Missing \"font_file\" falue.")
        );
        
        match render_from_user_font(&font_file, size, &image, invert) {
            Ok(r) => r,
            Err(ref s) => { error_response(500, s); },
        }
    } else {
        let font_name = font_name.unwrap_or_else(||
            error_response(400, "Missing \"font\" value.")
        );
        
        match render_from_server_font(&font_name, size, &image, invert) {
            Ok(r) => r,
            Err(s) => { error_response(500, &s); },   
        }
    };
    
    // let fonts = match load_library() {
    //     Ok(map) => map,
    //     Err(e) => { error_response(500, &e); },
    // };
    
    // let font_family = match fonts.get(&font) {
    //     Some(map) => map,
    //     None => {
    //         let estr = format!("No font data matching \"{}\".", &font);
    //         error_response(400, &estr);
    //     },
    // };
    
    // let font_data = match font_family.get(&size) {
    //     Some(fd) => fd,
    //     None => {
    //         let estr = format!("No data for font \"{}\" at size \"{}\".",
    //                             &font, &size);
    //         error_response(400, &estr);
    //     },
    // };
    
    
    // let mut r = EmptyResponse::new(200)
    //     .with_content_type("text/plain");
    
    // if invert {
    //     if let Err(e) = ascii_art::write_inverted(
    //         &image, &font_data, &mut r
    //     ) {
    //         let estr = format!("Error writing text image: {}", &e);
    //         error_response(500, &estr);
    //     }
    // } else {
    //     if let Err(e) = ascii_art::write(
    //         &image, &font_data, &mut r
    //     ) {
    //         let estr = format!("Error writing text image: {}", &e);
    //         error_response(500, &estr);
    //     }
    // }
    
    log::debug!(
        "render response: {}: {} bytes of body.",
        response.get_status(), response.get_body().len()
    );
    
    response.respond().unwrap();
    
    std::process::exit(0);
}

fn main() {
    use simplelog::{WriteLogger, LevelFilter, Config};
    WriteLogger::init(
        LevelFilter::max(),
        Config::default(),
        std::fs::OpenOptions::new().write(true)
            .open("/home/dan/aa_cgi.log").unwrap()
    ).unwrap();
    
    let req = match Request::new() {
        Ok(req) => req,
        Err(e) => {
            e.to_response().respond().unwrap();
            std::process::exit(0);
        },
    };
    
    log::debug!("rec'd request: {} {}",
        req.var("REQUEST_METHOD").unwrap_or("[ no METHOD ]"),
        req.var("REQUEST_URI").unwrap_or("[ no URI ]")
    );
    
    if let Some("OPTIONS") = req.var("METHOD") {
        options_response();
    }
    
    match req.header("aa-action") {
        None => error_response(400, "Missing \"aa-action\" header."),
        Some(action) => {
            let action = action.to_lowercase();
            if action == "list" {
                list_response();
            } else if action == "render" {
                render_response(&req);
            } else {
                error_response(400, "aa-action header must be one of \"list\", \"render\".");
            }
        }
    }
}