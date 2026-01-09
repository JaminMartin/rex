use crate::server::http_transport::HTTPTransport;
use crate::server::websocket_transport::WebSocketTransport;
use crate::tcp_handler::TCPTransport;
pub trait Transport {
    fn send_command(&mut self, command: &str) -> Result<String, Box<dyn std::error::Error>>;
    fn is_connected(&self) -> bool;
    fn ensure_connection(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    fn disconnect(&mut self) -> Option<String> {
        None
    }
}
#[derive(Debug)]
pub enum TransportImpl {
    Http(HTTPTransport),
    Tcp(TCPTransport),
    Ws(WebSocketTransport),
}

impl Transport for TransportImpl {
    fn send_command(&mut self, command: &str) -> Result<String, Box<dyn std::error::Error>> {
        match self {
            TransportImpl::Http(t) => t.send_command(command),
            TransportImpl::Tcp(t) => t.send_command(command),
            TransportImpl::Ws(t) => t.send_command(command),
        }
    }

    fn is_connected(&self) -> bool {
        match self {
            TransportImpl::Http(t) => t.is_connected(),
            TransportImpl::Tcp(t) => t.is_connected(),
            TransportImpl::Ws(t) => t.is_connected(),
        }
    }

    fn ensure_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            TransportImpl::Http(t) => t.ensure_connection(),
            TransportImpl::Tcp(t) => t.ensure_connection(),
            TransportImpl::Ws(t) => t.ensure_connection(),
        }
    }

    fn disconnect(&mut self) -> Option<String> {
        match self {
            TransportImpl::Http(t) => t.disconnect(),
            TransportImpl::Tcp(t) => t.disconnect(),
            TransportImpl::Ws(t) => t.disconnect(),
        }
    }
}
