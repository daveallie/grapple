use auth_helper::AuthenticationRequest;
use reqwest::header::{HeaderMap, HeaderValue, RANGE};
use reqwest::{Client, Method, Response, Url};
use std::str::FromStr;
use url::form_urlencoded;

enum ByteRangeSpec {
    FromTo(u64, u64),
}

impl ToString for ByteRangeSpec {
    fn to_string(&self) -> String {
        let ByteRangeSpec::FromTo(from, to) = self;
        format!("bytes={}-{}", from, to)
    }
}

pub fn head_request(uri: Url) -> Response {
    authed_request(uri, "HEAD")
}

pub fn get_range_request(uri: Url, range: (u64, u64)) -> Response {
    let (from, to) = range;
    let range_spec = ByteRangeSpec::FromTo(from, to);
    let mut headers = HeaderMap::new();
    headers.insert(RANGE, HeaderValue::from_str(&range_spec.to_string()).unwrap());
    authed_request_with_headers(uri, "GET", headers)
}

pub fn authed_request(uri: Url, method: &str) -> Response {
    authed_request_with_headers(uri, method, HeaderMap::new())
}

pub fn authed_request_with_headers(uri: Url, method: &str, headers: HeaderMap) -> Response {
    let da = AuthenticationRequest::new(
        uri.as_str().to_string(),
        uri.username().to_string(),
        uri.password().map(|password| password.to_string()),
        Some(method.to_string()),
    );

    let req_method = Method::from_str(method).expect("Invalid method!");

    let client = Client::new();
    let mut req_builder = client.request(req_method, uri);
    req_builder = match da.authenticate() {
        Ok(Some(auth_headers)) => req_builder.headers(headers).headers(auth_headers),
        Ok(None) => req_builder.headers(headers),
        Err(e) => panic!("{}", e), // this is genuine error, authentication was not attempted
    };
    let res = req_builder.send().unwrap();

    if !res.status().is_success() {
        panic!("Didn't get a 2xx response. Status: {:?}", res.status());
    }

    res
}

pub fn get_last_url_segment_decoded(uri: &Url) -> String {
    let last_segment = uri
        .as_str()
        .split('?')
        .next()
        .unwrap()
        .split('/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or("file")
        .to_string();

    let ls_clone = last_segment.clone();

    let last_segment_as_bytes = ls_clone.as_bytes();
    let last_segment_parts = form_urlencoded::parse(last_segment_as_bytes)
        .into_owned()
        .next();

    if let Some((parsed_segment, _)) = last_segment_parts {
        if !parsed_segment.is_empty() {
            return parsed_segment;
        }
    }

    last_segment
}

pub fn override_username_password(
    uri: &mut Url,
    username: Option<String>,
    password: Option<String>,
) {
    if let Some(user) = username {
        uri.set_username(&user)
            .expect("Failed to set username from param");
    }

    if let Some(pass) = password {
        uri.set_password(Some(&pass))
            .expect("Failed to set username from param");
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn basic_get_last_url_segment_decoded() {
        let url = Url::parse("http://origin.com/some/path/to/a/file.txt").unwrap();
        assert_eq!(get_last_url_segment_decoded(&url), "file.txt".to_string());
    }

    #[test]
    fn query_string_get_last_url_segment_decoded() {
        let url = Url::parse("http://origin.com/some/path/to/a/file.txt?a=b&b=c").unwrap();
        assert_eq!(get_last_url_segment_decoded(&url), "file.txt".to_string());
    }

    #[test]
    fn decode_get_last_url_segment_decoded() {
        let url = Url::parse("http://origin.com/some/path/to/a/file%20name.txt").unwrap();
        assert_eq!(
            get_last_url_segment_decoded(&url),
            "file name.txt".to_string()
        );
    }

    #[test]
    fn override_username_password_existing_auth() {
        let mut url = Url::parse("http://user@origin.com/some/path/to/a/file.txt").unwrap();
        override_username_password(&mut url, Some("newuser".to_string()), None);
        assert_eq!(
            url.to_string(),
            "http://newuser@origin.com/some/path/to/a/file.txt".to_string()
        );

        override_username_password(&mut url, None, Some("password".to_string()));
        assert_eq!(
            url.to_string(),
            "http://newuser:password@origin.com/some/path/to/a/file.txt".to_string()
        );

        override_username_password(
            &mut url,
            Some("".to_string()),
            Some("password2".to_string()),
        );
        assert_eq!(
            url.to_string(),
            "http://:password2@origin.com/some/path/to/a/file.txt".to_string()
        );
    }

    #[test]
    fn override_username_password_blank_auth() {
        let mut url = Url::parse("http://origin.com/some/path/to/a/file.txt").unwrap();
        override_username_password(&mut url, Some("newuser".to_string()), None);
        assert_eq!(
            url.to_string(),
            "http://newuser@origin.com/some/path/to/a/file.txt".to_string()
        );

        let mut url = Url::parse("http://origin.com/some/path/to/a/file.txt").unwrap();
        override_username_password(&mut url, None, Some("password".to_string()));
        assert_eq!(
            url.to_string(),
            "http://:password@origin.com/some/path/to/a/file.txt".to_string()
        );

        let mut url = Url::parse("http://origin.com/some/path/to/a/file.txt").unwrap();
        override_username_password(
            &mut url,
            Some("user".to_string()),
            Some("password".to_string()),
        );
        assert_eq!(
            url.to_string(),
            "http://user:password@origin.com/some/path/to/a/file.txt".to_string()
        );
    }
}
