use crate::error::RedisError;
use std::{
  fmt,
  fmt::{Debug, Formatter},
  net::IpAddr,
  sync::Arc,
};

#[cfg(feature = "enable-native-tls")]
use crate::error::RedisErrorKind;
#[cfg(feature = "enable-native-tls")]
use std::convert::{TryFrom, TryInto};
#[cfg(feature = "enable-native-tls")]
pub use tokio_native_tls::native_tls;
#[cfg(feature = "enable-native-tls")]
use tokio_native_tls::native_tls::{
  TlsConnector as NativeTlsConnector,
  TlsConnectorBuilder as NativeTlsConnectorBuilder,
};
#[cfg(feature = "enable-native-tls")]
use tokio_native_tls::TlsConnector as TokioNativeTlsConnector;
#[cfg(feature = "enable-rustls")]
pub use tokio_rustls::rustls;
#[cfg(feature = "enable-rustls")]
use tokio_rustls::rustls::{Certificate, ClientConfig as RustlsClientConfig, RootCertStore};
#[cfg(feature = "enable-rustls")]
use tokio_rustls::TlsConnector as RustlsConnector;

/// A trait used for mapping IP addresses to hostnames when processing the `CLUSTER SLOTS` response.
#[cfg_attr(docsrs, doc(cfg(any(feature = "enable-native-tls", feature = "enable-rustls"))))]
pub trait HostMapping: Send + Sync + Debug {
  /// Map the provided IP address to a hostname that should be used during the TLS handshake.
  ///
  /// The `default_host` argument represents the hostname of the node that returned the `CLUSTER SLOTS` response.
  ///
  /// If `None` is returned the client will use the IP address as the server name during the TLS handshake.
  fn map(&self, ip: &IpAddr, default_host: &str) -> Option<String>;
}

/// An optional enum used to describe how the client should modify or map IP addresses and hostnames in a clustered
/// deployment.
///
/// This is only necessary to use with a clustered deployment. Centralized or sentinel deployments should use `None`.
///
/// More information can be found [here](https://github.com/mna/redisc/issues/13) and [here](https://github.com/lettuce-io/lettuce-core/issues/1454#issuecomment-707537384).
#[cfg_attr(docsrs, doc(cfg(any(feature = "enable-native-tls", feature = "enable-rustls"))))]
#[derive(Clone, Debug)]
pub enum TlsHostMapping {
  /// Do not modify or replace hostnames or IP addresses in the `CLUSTER SLOTS` response.
  ///
  /// Default
  None,
  /// Replace any IP addresses in the `CLUSTER SLOTS` response with the hostname of the node that returned
  /// the `CLUSTER SLOTS` response.
  ///
  /// If the `CLUSTER SLOTS` response contains hostnames alongside IP addresses (via the `metadata` block) then
  /// those hostnames will be used instead. However, this is a relatively new Redis feature and it's likely some
  /// configurations will not expose this information.
  DefaultHost,
  /// Provide a custom mapping from IP address to hostname to be used in a manner similar to a reverse DNS lookup.
  Custom(Arc<dyn HostMapping>),
}

impl TlsHostMapping {
  pub(crate) fn map(&self, value: &IpAddr, default_host: &str) -> Option<String> {
    match self {
      TlsHostMapping::None => None,
      TlsHostMapping::DefaultHost => Some(default_host.to_owned()),
      TlsHostMapping::Custom(ref inner) => inner.map(value, &default_host),
    }
  }
}

impl PartialEq for TlsHostMapping {
  fn eq(&self, other: &Self) -> bool {
    match self {
      TlsHostMapping::None => match other {
        TlsHostMapping::None => true,
        _ => false,
      },
      TlsHostMapping::DefaultHost => match other {
        TlsHostMapping::DefaultHost => true,
        _ => false,
      },
      TlsHostMapping::Custom(_) => match other {
        TlsHostMapping::Custom(_) => true,
        _ => false,
      },
    }
  }
}

impl Eq for TlsHostMapping {}

/// TLS configuration for a client.
///
/// Note: the `hostnames` field is only necessary to use with certain clustered deployments.
///
/// ```rust no_compile no_run
/// let config = TlsConfig {
///   // or use `TlsConnector::default_rustls()`
///   connector: TlsConnector::default_native_tls(),
///   hostnames: TlsHostMapping::None
/// };
///
/// // or use the shorthand
/// let config: TlsConfig = TlsConnector::default_native_tls()?.into();
/// let config: TlsConfig = TlsConnector::default_rustls()?.into();
/// ```
#[cfg_attr(docsrs, doc(cfg(any(feature = "enable-native-tls", feature = "enable-rustls"))))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TlsConfig {
  /// The TLS connector from either `native-tls` or `rustls`.
  pub connector: TlsConnector,
  /// The hostname modification or mapping policy to use when discovering and connecting to cluster nodes.
  pub hostnames: TlsHostMapping,
}

impl<C: Into<TlsConnector>> From<C> for TlsConfig {
  fn from(connector: C) -> Self {
    TlsConfig {
      connector: connector.into(),
      hostnames: TlsHostMapping::None,
    }
  }
}

/// An enum for interacting with various TLS libraries and interfaces.
#[cfg_attr(docsrs, doc(cfg(any(feature = "enable-native-tls", feature = "enable-rustls"))))]
#[derive(Clone)]
pub enum TlsConnector {
  #[cfg(feature = "enable-native-tls")]
  #[cfg_attr(docsrs, doc(cfg(feature = "enable-native-tls")))]
  Native(TokioNativeTlsConnector),
  #[cfg(feature = "enable-rustls")]
  #[cfg_attr(docsrs, doc(cfg(feature = "enable-rustls")))]
  Rustls(RustlsConnector),
}

impl PartialEq for TlsConnector {
  fn eq(&self, _: &Self) -> bool {
    true
  }
}

impl Eq for TlsConnector {}

impl Debug for TlsConnector {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.debug_struct("TlsConnector")
      .field("kind", match self {
        #[cfg(feature = "enable-native-tls")]
        TlsConnector::Native(_) => &"Native",
        #[cfg(feature = "enable-rustls")]
        TlsConnector::Rustls(_) => &"Rustls",
      })
      .finish()
  }
}

#[cfg_attr(docsrs, doc(cfg(any(feature = "enable-native-tls", feature = "enable-rustls"))))]
impl TlsConnector {
  /// Create a default TLS connector from the `native-tls` module.
  #[cfg(feature = "enable-native-tls")]
  #[cfg_attr(docsrs, doc(cfg(feature = "enable-native-tls")))]
  pub fn default_native_tls() -> Result<Self, RedisError> {
    NativeTlsConnector::builder().try_into()
  }

  /// Create a default TLS connector with the `rustls` module with safe defaults and system certs via [rustls-native-certs](https://github.com/rustls/rustls-native-certs).
  #[cfg(feature = "enable-rustls")]
  #[cfg_attr(docsrs, doc(cfg(feature = "enable-rustls")))]
  pub fn default_rustls() -> Result<Self, RedisError> {
    let system_certs = rustls_native_certs::load_native_certs()?;
    let mut cert_store = RootCertStore::empty();
    for system_cert in system_certs.into_iter() {
      let _ = cert_store.add(&Certificate(system_cert.0))?;
    }

    Ok(
      RustlsClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(cert_store)
        .with_no_client_auth()
        .into(),
    )
  }
}

#[cfg(feature = "enable-native-tls")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable-native-tls")))]
impl TryFrom<NativeTlsConnectorBuilder> for TlsConnector {
  type Error = RedisError;

  fn try_from(builder: NativeTlsConnectorBuilder) -> Result<Self, Self::Error> {
    let connector = builder
      .build()
      .map(|t| TokioNativeTlsConnector::from(t))
      .map_err(|e| RedisError::new(RedisErrorKind::Tls, format!("{:?}", e)))?;
    Ok(TlsConnector::Native(connector))
  }
}

#[cfg(feature = "enable-native-tls")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable-native-tls")))]
impl From<NativeTlsConnector> for TlsConnector {
  fn from(connector: NativeTlsConnector) -> Self {
    TlsConnector::Native(TokioNativeTlsConnector::from(connector))
  }
}

#[cfg(feature = "enable-native-tls")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable-native-tls")))]
impl From<TokioNativeTlsConnector> for TlsConnector {
  fn from(connector: TokioNativeTlsConnector) -> Self {
    TlsConnector::Native(connector)
  }
}

#[cfg(feature = "enable-rustls")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable-rustls")))]
impl From<RustlsClientConfig> for TlsConnector {
  fn from(config: RustlsClientConfig) -> Self {
    TlsConnector::Rustls(RustlsConnector::from(Arc::new(config)))
  }
}

#[cfg(feature = "enable-rustls")]
#[cfg_attr(docsrs, doc(cfg(feature = "enable-rustls")))]
impl From<RustlsConnector> for TlsConnector {
  fn from(connector: RustlsConnector) -> Self {
    TlsConnector::Rustls(connector)
  }
}
