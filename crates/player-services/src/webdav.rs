use crate::{Url, radio::RESPONSE_LIMIT};

#[derive(Debug, Clone, PartialEq)]
pub struct WebDavEntry {
    pub name: String,
    pub href: Url,
    pub is_collection: bool,
    pub size_bytes: Option<u64>,
}

pub struct WebDavCredentials {
    username: String,
    password: String,
}

impl Drop for WebDavCredentials {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.username.zeroize();
        self.password.zeroize();
    }
}

impl WebDavCredentials {
    pub fn new(username: &str, password: &str) -> Result<Self, String> {
        if username.is_empty()
            || username.len() > 256
            || username.contains(':')
            || username.chars().any(char::is_control)
            || password.is_empty()
            || password.len() > 1024
            || password.chars().any(char::is_control)
        {
            return Err("Enter a username and password without control characters".into());
        }
        Ok(Self {
            username: username.to_owned(),
            password: password.to_owned(),
        })
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Debug, Clone)]
pub struct WebDavDirectory {
    base: Url,
}

fn propstat_succeeded(propstat: roxmltree::Node<'_, '_>) -> bool {
    let status = propstat
        .children()
        .find(|node| node.is_element() && node.tag_name().name() == "status")
        .and_then(|node| node.text())
        .map(str::trim);
    status.is_none_or(|status| {
        status
            .split_whitespace()
            .nth(1)
            .and_then(|code| code.parse::<u16>().ok())
            .is_some_and(|code| (200..300).contains(&code))
    })
}

impl WebDavDirectory {
    pub fn new(text: &str) -> Result<Self, String> {
        let base = crate::public_http_url(text)?;
        if base.query().is_some() || base.fragment().is_some() || !base.path().ends_with('/') {
            return Err("Enter a WebDAV collection URL ending in /".into());
        }
        Ok(Self { base })
    }

    pub fn browse(
        &self,
        client: &impl crate::HttpClient,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<Vec<WebDavEntry>, String> {
        self.browse_with_credentials(client, cancel, None)
    }

    pub fn browse_with_credentials(
        &self,
        client: &impl crate::HttpClient,
        cancel: &std::sync::atomic::AtomicBool,
        credentials: Option<&WebDavCredentials>,
    ) -> Result<Vec<WebDavEntry>, String> {
        use std::sync::atomic::Ordering;

        if cancel.load(Ordering::Acquire) {
            return Err("Browse cancelled".into());
        }
        if credentials.is_some() && !secure_credentials_transport(&self.base) {
            return Err("Use HTTPS before sending WebDAV credentials".into());
        }
        const PROPFIND: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
            <d:propfind xmlns:d="DAV:"><d:prop><d:resourcetype/>
            <d:getcontentlength/></d:prop></d:propfind>"#;
        let response = client.propfind_authenticated(&self.base, PROPFIND, credentials, cancel)?;
        if cancel.load(Ordering::Acquire) {
            return Err("Browse cancelled".into());
        }
        parse_webdav_listing(&response, &self.base)
    }

    pub fn contains_item(&self, href: &Url) -> bool {
        direct_child_name(&self.base, href).is_some()
    }
}

fn direct_child_name(base: &Url, href: &Url) -> Option<String> {
    if href.origin() != base.origin()
        || !href.username().is_empty()
        || href.password().is_some()
        || href.query().is_some()
        || href.fragment().is_some()
    {
        return None;
    }
    let suffix = href.path().strip_prefix(base.path())?;
    let trimmed = suffix.strip_suffix('/').unwrap_or(suffix);
    if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains('\\') {
        return None;
    }
    let decoded = percent_encoding::percent_decode_str(trimmed).decode_utf8_lossy();
    if decoded.is_empty() || decoded == "." || decoded == ".." || decoded.contains(['/', '\\']) {
        return None;
    }
    Some(decoded.into_owned())
}

fn secure_credentials_transport(url: &Url) -> bool {
    url.scheme() == "https"
        || match url.host() {
            Some(url::Host::Domain("localhost")) => true,
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        }
}

/// Parse a bounded WebDAV multistatus response, retaining only direct children of `base`.
pub fn parse_webdav_listing(body: &[u8], base: &Url) -> Result<Vec<WebDavEntry>, String> {
    if body.len() > RESPONSE_LIMIT {
        return Err("WebDAV response is too large".into());
    }
    if !matches!(base.scheme(), "http" | "https")
        || base.host_str().is_none()
        || !base.username().is_empty()
        || base.password().is_some()
        || base.query().is_some()
        || base.fragment().is_some()
        || !base.path().ends_with('/')
    {
        return Err("Enter a WebDAV collection URL ending in /".into());
    }
    let xml = std::str::from_utf8(body).map_err(|_| "WebDAV response is not valid UTF-8")?;
    let document = roxmltree::Document::parse(xml).map_err(|_| "WebDAV returned invalid XML")?;
    let mut entries = Vec::new();
    for response in document
        .descendants()
        .filter(|node| node.is_element() && node.tag_name().name() == "response")
    {
        let Some(href_text) = response
            .children()
            .find(|node| node.is_element() && node.tag_name().name() == "href")
            .and_then(|node| node.text())
            .map(str::trim)
        else {
            continue;
        };
        if href_text.is_empty() || href_text.len() > 4096 {
            continue;
        }
        let propstats: Vec<_> = response
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "propstat")
            .collect();
        let properties = propstats
            .iter()
            .copied()
            .find(|propstat| propstat_succeeded(*propstat));
        if !propstats.is_empty() && properties.is_none() {
            continue;
        }
        let properties = properties.unwrap_or(response);
        let Ok(href) = base.join(href_text) else {
            continue;
        };
        let Some(decoded) = direct_child_name(base, &href) else {
            continue;
        };
        let name: String = decoded
            .chars()
            .map(|character| {
                if character.is_control() {
                    ' '
                } else {
                    character
                }
            })
            .take(512)
            .collect();
        let is_collection = properties
            .descendants()
            .any(|node| node.is_element() && node.tag_name().name() == "collection");
        let size_bytes = properties
            .descendants()
            .find(|node| node.is_element() && node.tag_name().name() == "getcontentlength")
            .and_then(|node| node.text())
            .and_then(|size| size.trim().parse().ok());
        entries.push(WebDavEntry {
            name,
            href,
            is_collection,
            size_bytes,
        });
        if entries.len() >= 1000 {
            break;
        }
    }
    entries.sort_by(|left, right| {
        right
            .is_collection
            .cmp(&left.is_collection)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::{WebDavCredentials, WebDavDirectory, parse_webdav_listing};
    use crate::{HttpClient, Url};
    use std::sync::atomic::AtomicBool;

    #[test]
    fn browser_lists_only_direct_children_and_ignores_the_collection_itself() {
        let base = Url::parse("https://dav.example/media/").unwrap();
        let response = br#"<?xml version="1.0" encoding="utf-8"?>
            <d:multistatus xmlns:d="DAV:">
              <d:response><d:href>/media/</d:href><d:propstat><d:prop>
                <d:resourcetype><d:collection/></d:resourcetype>
              </d:prop></d:propstat></d:response>
              <d:response><d:href>/media/Concert%20Video.mkv</d:href><d:propstat><d:prop>
                <d:getcontentlength>12345</d:getcontentlength>
                <d:resourcetype/>
              </d:prop></d:propstat></d:response>
              <d:response><d:href>/media/Albums/</d:href><d:propstat><d:prop>
                <d:resourcetype><d:collection/></d:resourcetype>
              </d:prop></d:propstat></d:response>
            </d:multistatus>"#;

        let entries = parse_webdav_listing(response, &base).unwrap();

        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_collection, "collections sort before files");
        assert_eq!(entries[0].name, "Albums");
        assert_eq!(entries[1].name, "Concert Video.mkv");
        assert_eq!(
            entries[1].href.as_str(),
            "https://dav.example/media/Concert%20Video.mkv"
        );
        assert!(!entries[1].is_collection);
        assert_eq!(entries[1].size_bytes, Some(12_345));
    }

    #[test]
    fn browser_rejects_external_parent_nested_and_encoded_separator_links() {
        let base = Url::parse("https://dav.example/media/").unwrap();
        let response = br#"<d:multistatus xmlns:d="DAV:">
          <d:response><d:href>https://evil.example/media/steal.mkv</d:href></d:response>
          <d:response><d:href>/private/secret.mkv</d:href></d:response>
          <d:response><d:href>../secret.mkv</d:href></d:response>
          <d:response><d:href>folder/nested.mkv</d:href></d:response>
          <d:response><d:href>%2Fprivate</d:href></d:response>
          <d:response><d:href>https://user:pass@dav.example/media/private.mkv</d:href></d:response>
          <d:response><d:href>unavailable.mkv</d:href><d:propstat>
            <d:prop><d:resourcetype/></d:prop><d:status>HTTP/1.1 404 Not Found</d:status>
          </d:propstat></d:response>
          <d:response><d:href>valid.mkv</d:href></d:response>
        </d:multistatus>"#;

        let entries = parse_webdav_listing(response, &base).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "valid.mkv");
    }

    #[test]
    fn browse_sends_a_depth_one_request_and_returns_visible_items() {
        struct Fixture;
        impl HttpClient for Fixture {
            fn get(&self, _: &Url, _: &AtomicBool) -> Result<Vec<u8>, String> {
                panic!("WebDAV browsing must use PROPFIND")
            }

            fn propfind(&self, url: &Url, body: &[u8], _: &AtomicBool) -> Result<Vec<u8>, String> {
                assert_eq!(url.as_str(), "https://dav.example/media/");
                assert!(std::str::from_utf8(body).unwrap().contains("<d:propfind"));
                Ok(br#"<d:multistatus xmlns:d="DAV:"><d:response>
                  <d:href>/media/Movie.mkv</d:href><d:propstat><d:prop><d:resourcetype/>
                  </d:prop></d:propstat></d:response></d:multistatus>"#
                    .to_vec())
            }
        }

        let directory = WebDavDirectory::new("https://dav.example/media/").unwrap();
        let entries = directory.browse(&Fixture, &AtomicBool::new(false)).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Movie.mkv");
    }

    #[test]
    fn directory_requires_a_clean_collection_url_and_cancellation_avoids_network() {
        for invalid in [
            "file:///private/",
            "https://user:secret@dav.example/media/",
            "https://dav.example/media/?token=secret",
            "https://dav.example/media#fragment/",
            "https://dav.example/media",
        ] {
            assert!(WebDavDirectory::new(invalid).is_err(), "accepted {invalid}");
        }
        assert!(WebDavDirectory::new("https://dav.example/media/").is_ok());

        struct NoRequests;
        impl HttpClient for NoRequests {
            fn get(&self, _: &Url, _: &AtomicBool) -> Result<Vec<u8>, String> {
                panic!("cancelled browse must not issue a request")
            }
        }
        let directory = WebDavDirectory::new("https://dav.example/media/").unwrap();
        let error = directory
            .browse(&NoRequests, &AtomicBool::new(true))
            .unwrap_err();
        assert_eq!(error, "Browse cancelled");
    }

    #[test]
    fn playback_selection_accepts_only_safe_direct_children() {
        let directory = WebDavDirectory::new("https://dav.example/media/").unwrap();
        assert!(
            directory.contains_item(&Url::parse("https://dav.example/media/movie.mkv").unwrap())
        );
        for text in [
            "https://dav.example/media2/movie.mkv",
            "https://dav.example/media/sub/movie.mkv",
            "https://dav.example/media/%2e%2e%2fsecret.mkv",
            "https://user:secret@dav.example/media/movie.mkv",
            "https://other.example/media/movie.mkv",
            "https://dav.example/media/movie.mkv?token=secret",
        ] {
            assert!(
                !directory.contains_item(&Url::parse(text).unwrap()),
                "accepted {text}"
            );
        }
    }

    #[test]
    fn parser_rejects_invalid_xml_and_oversized_responses() {
        let base = Url::parse("https://dav.example/media/").unwrap();
        assert!(parse_webdav_listing(b"<unclosed>", &base).is_err());
        assert!(parse_webdav_listing(&vec![b' '; 2 * 1024 * 1024 + 1], &base).is_err());
    }

    #[test]
    fn authenticated_browse_passes_validated_credentials_only_to_the_http_client() {
        struct AuthFixture;
        impl HttpClient for AuthFixture {
            fn get(&self, _: &Url, _: &AtomicBool) -> Result<Vec<u8>, String> {
                panic!("WebDAV browsing must use PROPFIND")
            }

            fn propfind_authenticated(
                &self,
                _: &Url,
                _: &[u8],
                credentials: Option<&WebDavCredentials>,
                _: &AtomicBool,
            ) -> Result<Vec<u8>, String> {
                let credentials = credentials.unwrap();
                assert_eq!(credentials.username(), "media-user");
                assert_eq!(credentials.password(), "secret with spaces");
                Ok(br#"<d:multistatus xmlns:d="DAV:"></d:multistatus>"#.to_vec())
            }
        }
        let directory = WebDavDirectory::new("https://dav.example/media/").unwrap();
        let credentials = WebDavCredentials::new("media-user", "secret with spaces").unwrap();

        assert!(
            directory
                .browse_with_credentials(&AuthFixture, &AtomicBool::new(false), Some(&credentials))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn credentials_are_rejected_for_remote_plain_http_before_network_access() {
        struct NoRequests;
        impl HttpClient for NoRequests {
            fn get(&self, _: &Url, _: &AtomicBool) -> Result<Vec<u8>, String> {
                panic!("credentials must be rejected before network access")
            }

            fn propfind_authenticated(
                &self,
                _: &Url,
                _: &[u8],
                _: Option<&WebDavCredentials>,
                _: &AtomicBool,
            ) -> Result<Vec<u8>, String> {
                panic!("credentials must be rejected before network access")
            }
        }
        let directory = WebDavDirectory::new("http://dav.example/media/").unwrap();
        let credentials = WebDavCredentials::new("user", "secret").unwrap();
        let error = directory
            .browse_with_credentials(&NoRequests, &AtomicBool::new(false), Some(&credentials))
            .unwrap_err();
        assert!(error.contains("Use HTTPS"));
    }

    #[test]
    fn credentials_reject_basic_auth_delimiters_and_control_characters() {
        assert!(WebDavCredentials::new("", "secret").is_err());
        assert!(WebDavCredentials::new("user:other", "secret").is_err());
        assert!(WebDavCredentials::new("user\nforged", "secret").is_err());
        assert!(WebDavCredentials::new("user", "secret\nforged").is_err());
        assert!(WebDavCredentials::new("user", "").is_err());
    }
}
