# `ascii_art/utils

A couple of programs that use the `ascii_art` libraray but aren't necessarily
intended to be used by anyone else.

## `src/bin/aa_cgi.rs`

Together with `test/index.html` and `test/script.js`, this is a CGI endpoint
for applying the `ascii_art` crate with a web service. You can see it in
action here:

[`https://d2718.net/ascii_art/`](https://d2718.net/ascii_art/)

## `src/bin/librarify.rs`

This program is for generating a library file of font data for use by
`aa_cgi.rs`. It takes an input like `test/font_list.txt` and produces the
kind of file that `aa_cgi.rs` looks to load.

