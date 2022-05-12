# `ascii_art`

A Rust library for rendering images as text and some utilities that use it.

This workspace consists of a collection of crates:

  * `ascii_art`: a library for rendering images to text
  * `img2ascii`: a CLI program that uses the `ascii_art` library and your
    system's [Fontconfig](https://www.freedesktop.org/wiki/Software/fontconfig/)
    installation to render images to text based on any font installed on
    your system.
  * `utils`: some other utility programs that use the library

See more specific documentation for more specific information.

# License

The original work in this crate is all licensed under
[the MIT license](https://opensource.org/licenses/MIT).

The Liberation Mono and Iosevka fonts (used as example files) are licensed under the
[SIL Open Font License](https://scripts.sil.org/cms/scripts/page.php?site_id=nrsi&id=OFL).