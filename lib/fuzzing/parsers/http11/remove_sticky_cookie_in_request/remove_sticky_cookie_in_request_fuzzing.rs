extern crate afl;
extern crate sozu_lib;

use sozu_lib::parser::http11::Header;

fn main() {
  afl::read_stdio_string(|value| {
    let raw_header = format!("Cookie: {}\r\n", value);
    let header = Header {
      name: "Cookie".as_bytes(),
      value: &value.as_bytes()
    };

    header.remove_sticky_cookie_in_request(raw_header.as_bytes(), raw_header.len());
  })
}
