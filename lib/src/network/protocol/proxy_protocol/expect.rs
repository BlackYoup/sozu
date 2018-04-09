use std::net::IpAddr;
use std::io::{Write, ErrorKind};
use std::io::Read;

use mio::*;
use mio::tcp::TcpStream;
use mio::unix::UnixReady;
use nom::IResult::*;
use nom::Offset;
use network::protocol::proxy_protocol::header;
use network::{Protocol, ClientResult};
use network::Readiness;
use network::protocol::ProtocolResult;
use network::socket::{SocketHandler, SocketResult};
use network::buffer_queue::BufferQueue;
use network::SessionMetrics;
use network::protocol::pipe::Pipe;
use parser::proxy_protocol::parse_v2_header;
use pool::Checkout;
use super::header::ProxyAddr;

pub struct ExpectProxyProtocol<Front:SocketHandler> {
  pub frontend:       Front,
  pub frontend_token: Token,
  pub front_buf:      Checkout<BufferQueue>,
  pub readiness:      Readiness,
  pub addresses:      Option<ProxyAddr>,
}

impl <Front:SocketHandler + Read>ExpectProxyProtocol<Front> {
  pub fn new(frontend: Front, frontend_token: Token, front_buf: Checkout<BufferQueue>) -> Self {
    println!("expect starting, connection from {:?}", frontend.socket_ref().peer_addr());
    ExpectProxyProtocol {
      frontend,
      frontend_token,
      front_buf,
      readiness: Readiness {
        front_interest:  UnixReady::from(Ready::readable()) | UnixReady::hup() | UnixReady::error(),
        back_interest:   UnixReady::hup() | UnixReady::error(),
        front_readiness: UnixReady::from(Ready::empty()),
        back_readiness:  UnixReady::from(Ready::empty()),
      },
      addresses: None,
    }
  }

  pub fn readable(&mut self, metrics: &mut SessionMetrics) -> (ProtocolResult, ClientResult) {
    let (sz, res) = self.frontend.socket_read(self.front_buf.buffer.space());
    info!("FRONT proxy protocol [{:?}]: read {} bytes and res={:?}", self.frontend_token, sz, res);

    if sz > 0 {
      self.front_buf.buffer.fill(sz);
      self.front_buf.sliced_input(sz);

      count!("bytes_in", sz as i64);
      metrics.bin += sz;

      if res == SocketResult::Error {
        error!("[{:?}] front socket error, closing the connection", self.frontend_token);
        metrics.service_stop();
        incr_ereq!();
        self.readiness.reset();
        return (ProtocolResult::Continue, ClientResult::CloseClient);
      }

      if res == SocketResult::WouldBlock {
        self.readiness.front_readiness.remove(Ready::readable());
      }

      let read_sz = match parse_v2_header(self.front_buf.unparsed_data()) {
        Done(rest, header) => {
          self.addresses = Some(header.addr);
          self.front_buf.next_output_data().offset(rest)
        },
        Incomplete(_) => {
          return (ProtocolResult::Continue, ClientResult::Continue)
        },
        Error(e) => {
          return (ProtocolResult::Continue, ClientResult::CloseClient)
        }
      };

      self.front_buf.consume_parsed_data(read_sz);
      self.front_buf.delete_output(read_sz);
      info!("read {} bytes of proxy protocol, {} remaining", read_sz, self.front_buf.available_input_data());
      return (ProtocolResult::Upgrade, ClientResult::Continue)
    }

    return (ProtocolResult::Continue, ClientResult::Continue);
  }

  pub fn front_socket(&self) -> &TcpStream {
    self.frontend.socket_ref()
  }

  pub fn back_socket(&self) -> Option<&TcpStream> {
    unimplemented!()
  }

  pub fn set_back_socket(&mut self, socket: TcpStream) {
    unimplemented!()
  }

  pub fn front_token(&self) -> Option<Token> {
    Some(self.frontend_token)
  }

  pub fn set_front_token(&mut self, token: Token) {
    self.frontend_token = token;
  }

  pub fn back_token(&self) -> Option<Token> {
    unimplemented!()
  }

  pub fn set_back_token(&mut self, token: Token) {
    unimplemented!()
  }

  pub fn readiness(&mut self) -> &mut Readiness {
    &mut self.readiness
  }
}
