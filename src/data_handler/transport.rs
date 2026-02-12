use crate::server::http_transport::HTTPTransport;
use crate::tcp_handler::TCPTransport;
use async_trait::async_trait;

#[async_trait]
pub trait Transport: Clone + Send + Sync + 'static {
    async fn send_command(
        &mut self,
        command: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send>>;
    fn is_connected(&self) -> bool;
    async fn ensure_connection(&mut self) -> Result<(), Box<dyn std::error::Error + Send>>;
    async fn disconnect(&mut self) -> Option<String> {
        None
    }
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn transport_type(&self) -> TransportType;
    async fn rerun(
        &mut self,
        args: crate::cli_tool::RunArgs,
    ) -> Result<(), Box<dyn std::error::Error + Send>>;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TransportType {
    Http,
    Tcp,
    Ws,
}

#[derive(Clone)]
pub enum TransportImpl {
    Http(HTTPTransport),
    Tcp(TCPTransport),
}

#[async_trait]
impl Transport for TransportImpl {
    async fn send_command(
        &mut self,
        command: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send>> {
        match self {
            TransportImpl::Http(t) => t.send_command(command).await,
            TransportImpl::Tcp(t) => t.send_command(command).await,
        }
    }

    fn is_connected(&self) -> bool {
        match self {
            TransportImpl::Http(t) => t.is_connected(),
            TransportImpl::Tcp(t) => t.is_connected(),
        }
    }

    async fn ensure_connection(&mut self) -> Result<(), Box<dyn std::error::Error + Send>> {
        match self {
            TransportImpl::Http(t) => t.ensure_connection().await,
            TransportImpl::Tcp(t) => t.ensure_connection().await,
        }
    }

    async fn disconnect(&mut self) -> Option<String> {
        match self {
            TransportImpl::Http(t) => t.disconnect().await,
            TransportImpl::Tcp(t) => t.disconnect().await,
        }
    }

    fn transport_type(&self) -> TransportType {
        match self {
            TransportImpl::Http(t) => t.transport_type(),
            TransportImpl::Tcp(t) => t.transport_type(),
        }
    }

    async fn rerun(
        &mut self,
        args: crate::cli_tool::RunArgs,
    ) -> Result<(), Box<dyn std::error::Error + Send>> {
        match self {
            TransportImpl::Http(t) => t.rerun(args).await,
            TransportImpl::Tcp(t) => t.rerun(args).await,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        match self {
            TransportImpl::Http(t) => t.as_any(),
            TransportImpl::Tcp(t) => t.as_any(),
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        match self {
            TransportImpl::Http(t) => t.as_any_mut(),
            TransportImpl::Tcp(t) => t.as_any_mut(),
        }
    }
}
