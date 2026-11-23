//! Remote-image transport for GPUI.
//!
//! Orbit is local-first; the image traffic is author avatars and the images in
//! Markdown the user opens (issue bodies, agent replies), both requested by
//! `img("https://…")`. GPUI's image loader needs an [`HttpClient`], and its
//! default is a null client that renders nothing — so this module supplies a
//! small blocking client whose requests run on a worker thread, keeping the
//! GPUI executor free. TLS uses rustls with the platform trust store and
//! Mozilla's webpki roots as fallback.

use anyhow::{anyhow, Result};
use futures::channel::oneshot;
use futures::future::BoxFuture;
use futures::FutureExt;
use gpui::http_client::http::HeaderValue;
use gpui::http_client::{AsyncBody, HttpClient, Inner, Request, Response, Url};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::{io::Read, sync::Arc, time::Duration};

const USER_AGENT: &str = concat!("Orbit/", env!("CARGO_PKG_VERSION"));
const AVATAR_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const AVATAR_TOTAL_TIMEOUT: Duration = Duration::from_secs(15);
const AVATAR_REDIRECT_LIMIT: usize = 5;
const AVATAR_RESPONSE_BODY_LIMIT: u64 = 8 * 1024 * 1024;

/// What the resolver hands hyper on failure.
type ResolveError = Box<dyn std::error::Error + Send + Sync>;

/// The app's HTTP client, or `None` if the TLS stack can't start — in which
/// case remote images fall back to their monograms.
pub fn avatar_client() -> Option<Arc<dyn HttpClient>> {
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .connect_timeout(AVATAR_CONNECT_TIMEOUT)
        .timeout(AVATAR_TOTAL_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(AVATAR_REDIRECT_LIMIT))
        // Every hop of a redirect is resolved again, so the policy below
        // covers a chain that leads somewhere private.
        .dns_resolver(Arc::new(PublicResolver))
        .build()
        .ok()?;
    Some(Arc::new(AvatarClient { client }))
}

struct AvatarClient {
    client: reqwest::blocking::Client,
}

impl HttpClient for AvatarClient {
    fn type_name(&self) -> &'static str {
        "AvatarClient"
    }

    fn user_agent(&self) -> Option<&HeaderValue> {
        None
    }

    fn proxy(&self) -> Option<&Url> {
        None
    }

    fn send(&self, request: Request<AsyncBody>) -> BoxFuture<'static, Result<Response<AsyncBody>>> {
        let (parts, body) = request.into_parts();
        // Only GPUI's image GETs reach this client; reject streaming bodies
        // rather than pretend to support them.
        let payload = match body.0 {
            Inner::Empty => Vec::new(),
            Inner::Bytes(cursor) => cursor.into_inner().to_vec(),
            Inner::AsyncReader(_) => {
                return async { Err(anyhow!("streaming request bodies are not supported")) }
                    .boxed();
            }
        };
        let method = parts.method.as_str().to_owned();
        let uri = parts.uri.to_string();
        let url = match avatar_url(&uri) {
            Ok(url) => url,
            Err(err) => return async move { Err(err) }.boxed(),
        };

        let client = self.client.clone();
        let (tx, rx) = oneshot::channel();
        // Blocking I/O off the GPUI executor; one short-lived worker per
        // request. GPUI caches decoded images per URI, so this runs rarely.
        std::thread::spawn(move || {
            let result = client
                .request(
                    reqwest::Method::from_bytes(method.as_bytes()).unwrap_or(reqwest::Method::GET),
                    url,
                )
                .header("User-Agent", USER_AGENT)
                .body(payload)
                .send()
                .map_err(|err| anyhow!("{err}"))
                .and_then(into_gpui_response);
            let _ = tx.send(result);
        });

        async move {
            rx.await
                .map_err(|_| anyhow!("http worker dropped the request"))?
        }
        .boxed()
    }
}

fn avatar_url(uri: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(uri).map_err(|err| anyhow!("invalid avatar URL: {err}"))?;
    anyhow::ensure!(url.scheme() == "https", "avatar URL must use HTTPS");
    anyhow::ensure!(url.host_str().is_some(), "avatar URL must include a host");
    // A literal address never reaches the resolver, so it is refused here.
    match url.host() {
        Some(url::Host::Ipv4(ip)) => {
            anyhow::ensure!(is_public_ip(ip.into()), "image URL host is not public")
        }
        Some(url::Host::Ipv6(ip)) => {
            anyhow::ensure!(is_public_ip(ip.into()), "image URL host is not public")
        }
        Some(url::Host::Domain(_)) | None => {}
    }
    Ok(url)
}

/// DNS for remote images: public, globally routable addresses only.
///
/// A Markdown image in an agent reply or an issue body is a URL somebody else
/// wrote, so it must not be able to dial the user's own machine, their LAN, or
/// a metadata endpoint. The check sits in resolution rather than in the URL so
/// it also covers a redirect, and covers every name that resolves privately
/// whether or not it looks local.
struct PublicResolver;

impl reqwest::dns::Resolve for PublicResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_owned();
        // Resolved eagerly: the client's requests already run on their own
        // worker thread, so the blocking lookup never reaches the UI thread.
        Box::pin(std::future::ready(resolve_public(&host)))
    }
}

fn resolve_public(host: &str) -> std::result::Result<reqwest::dns::Addrs, ResolveError> {
    let addrs: Vec<SocketAddr> = (host, 0u16)
        .to_socket_addrs()
        .map_err(|err| -> ResolveError { Box::new(err) })?
        .filter(|addr| is_public_ip(addr.ip()))
        .collect();
    if addrs.is_empty() {
        return Err(format!("{host} resolves only to private or reserved addresses").into());
    }
    Ok(Box::new(addrs.into_iter()))
}

/// Whether an address is globally routable: the only kind a remote image may
/// reach. Loopback, private, carrier-grade NAT, link-local, documentation,
/// multicast, and reserved space all answer no.
fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.is_unspecified()
                || v4.is_multicast()
                // 0.0.0.0/8 "this network"
                || octets[0] == 0
                // 100.64.0.0/10 carrier-grade NAT
                || (octets[0] == 100 && (64..128).contains(&octets[1]))
                // 198.18.0.0/15 benchmarking
                || (octets[0] == 198 && (18..20).contains(&octets[1]))
                // 240.0.0.0/4 reserved
                || octets[0] >= 240)
        }
        // 2000::/3 global unicast; loopback, unspecified, unique-local
        // (fc00::/7) and link-local (fe80::/10) fall outside it.
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_public_ip(v4.into()),
            None => {
                let segments = v6.segments();
                (segments[0] & 0xe000) == 0x2000
                    && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
            }
        },
    }
}

/// Read at most one byte past the cap so an oversized body is rejected before
/// any of it reaches GPUI, without buffering an unbounded response.
fn read_body_capped(reader: impl Read, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|err| anyhow!("failed to read avatar response: {err}"))?;
    anyhow::ensure!(
        bytes.len() <= usize::try_from(limit).unwrap_or(usize::MAX),
        "avatar response body exceeds the {limit}-byte limit"
    );
    Ok(bytes)
}

/// Convert a reqwest response into the shape GPUI's image loader reads.
fn into_gpui_response(response: reqwest::blocking::Response) -> Result<Response<AsyncBody>> {
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let bytes = read_body_capped(response, AVATAR_RESPONSE_BODY_LIMIT)?;
    let mut builder = Response::builder().status(status);
    for (name, value) in headers.iter() {
        builder = builder.header(name.as_str(), value.as_bytes());
    }
    builder.body(AsyncBody::from(bytes)).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_urls_require_absolute_https() {
        assert_eq!(
            avatar_url("https://avatars.githubusercontent.com/u/1?v=4")
                .expect("GitHub avatar URLs are valid")
                .as_str(),
            "https://avatars.githubusercontent.com/u/1?v=4"
        );
        for url in [
            "http://avatars.githubusercontent.com/u/1",
            "file:///tmp/avatar.png",
            "data:image/png;base64,AAAA",
            "not a URL",
            "https://",
            // A literal address never reaches the resolver.
            "https://127.0.0.1:8443/avatar.png",
            "https://192.168.1.1/avatar.png",
            "https://[::1]/avatar.png",
        ] {
            assert!(avatar_url(url).is_err(), "unsafe avatar URL: {url}");
        }
    }

    /// A Markdown image is a URL somebody else wrote, so the resolver refuses
    /// anything that is not globally routable — the LAN and a metadata
    /// endpoint included.
    #[test]
    fn image_dns_resolves_only_public_addresses() {
        for allowed in [
            "93.184.216.34",
            "140.82.121.3",
            "2606:4700::1111",
            "::ffff:93.184.216.34",
        ] {
            assert!(is_public_ip(allowed.parse().unwrap()), "{allowed}");
        }
        for refused in [
            "127.0.0.1",
            "0.0.0.0",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "198.18.0.1",
            "224.0.0.1",
            "255.255.255.255",
            "240.0.0.1",
            "192.0.2.1",
            "::1",
            "::",
            "fe80::1",
            "fd00:ec2::254",
            "2001:db8::1",
            "::ffff:127.0.0.1",
            "::ffff:192.168.1.1",
        ] {
            assert!(!is_public_ip(refused.parse().unwrap()), "{refused}");
        }
        // A name that only resolves privately is refused outright.
        assert!(resolve_public("localhost").is_err());
    }

    #[test]
    fn response_body_cap_accepts_the_limit_and_rejects_the_next_byte() {
        let at_limit = vec![7; 16];
        assert_eq!(
            read_body_capped(std::io::Cursor::new(at_limit.clone()), 16)
                .expect("a body at the cap is accepted"),
            at_limit
        );

        let mut over_limit = std::io::Cursor::new(vec![7; 32]);
        let error = read_body_capped(&mut over_limit, 16)
            .expect_err("the first byte past the cap rejects the body");
        assert!(error.to_string().contains("16-byte limit"));
        assert_eq!(
            over_limit.position(),
            17,
            "the reader stops after the cap plus one probe byte"
        );
    }
}
