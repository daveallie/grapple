use auth_helper::AuthenticationRequest;
use reqwest::{Client, Method, Response, Url};
use reqwest::header::{ByteRangeSpec, Headers, Range};
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
    let da = AuthenticationRequest::new(uri.as_str().to_string(),
                                        uri.username().to_string(),
                                        uri.password()
                                            .map(|password| password.to_string()),
                                        Some(method.clone()));

    let req_method = Method::from_str(&method).expect("Invalid method!");

    let client = Client::new().unwrap();
    let req = client.request(req_method, uri).headers(headers);
    let req = match da.authenticate() {
        Ok(Some(auth_headers)) => req.headers(auth_headers),
        Ok(None) => req,
        Err(e) => panic!(e), // this is genuine error, authentication was not attempted
    };
    let res = req.send().unwrap();

    if !res.status().is_success() {
        panic!("Didn't get a 2xx response. Status: {:?}", res.status());
    }

    res
}

pub fn get_last_url_segment(uri: Url) -> String {
    uri.as_str().split("/").filter(|s| !s.is_empty()).last().unwrap_or("file").to_string()
}
