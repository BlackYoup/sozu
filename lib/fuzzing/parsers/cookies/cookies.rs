extern crate afl;
extern crate sozu_lib;

use sozu_lib::parser::cookies::parse_request_cookies;

fn main() {
  afl::read_stdio_bytes(|bytes| {
    parse_request_cookies(&bytes);
  });
}
