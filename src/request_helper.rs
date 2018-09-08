use auth_helper::AuthenticationRequest;
use reqwest::header::{ByteRangeSpec, Headers, Range};
use reqwest::{Client, Method, Response, Url};
use std::str::FromStr;

pub fn head_request(uri: Url) -> Response {
    authed_request(uri, "HEAD".to_string())
}

pub fn get_range_request(uri: Url, range: (u64, u64)) -> Response {
    let (from, to) = range;
    let mut headers = Headers::new();
    headers.set(Range::Bytes(vec![ByteRangeSpec::FromTo(from, to)]));
    authed_request_with_headers(uri, "GET".to_string(), headers)
}

pub fn authed_request(uri: Url, method: String) -> Response {
    authed_request_with_headers(uri, method, Headers::new())
}

pub fn authed_request_with_headers(uri: Url, method: String, headers: Headers) -> Response {
    let da = AuthenticationRequest::new(
        uri.as_str().to_string(),
        uri.username().to_string(),
        uri.password().map(|password| password.to_string()),
        Some(method.clone()),
    );

    let req_method = Method::from_str(&method).expect("Invalid method!");

    let client = Client::new();
    let mut req_builder = client.request(req_method, uri);
    match da.authenticate() {
        Ok(Some(auth_headers)) => req_builder.headers(headers).headers(auth_headers),
        Ok(None) => req_builder.headers(headers),
        Err(e) => panic!(e), // this is genuine error, authentication was not attempted
    };
    let res = req_builder.send().unwrap();

    if !res.status().is_success() {
        panic!("Didn't get a 2xx response. Status: {:?}", res.status());
    }

    res
}

pub fn get_last_url_segment(uri: Url) -> String {
    uri.as_str()
        .split("?")
        .next()
        .unwrap()
        .split("/")
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or("file")
        .to_string()
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn basic_get_last_url_segment() {
        let url = Url::parse("http://origin.com/some/path/to/a/file.txt").unwrap();
        assert_eq!(get_last_url_segment(url), "file.txt".to_string());
    }

    #[test]
    fn query_string_get_last_url_segment() {
        let url = Url::parse("http://origin.com/some/path/to/a/file.txt?a=b&b=c").unwrap();
        assert_eq!(get_last_url_segment(url), "file.txt".to_string());
    }
}
