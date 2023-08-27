use base64;
use md5;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, WWW_AUTHENTICATE};
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Mutex;
use url::Url;
use uuid::Uuid;

lazy_static! {
    static ref NONCES: Mutex<HashMap<String, usize>> = Mutex::new(HashMap::new());
}

pub struct AuthenticationRequest {
    method: Option<String>,
    username: String,
    password: Option<String>,
    url: String,
}

impl AuthenticationRequest {
    pub fn new(
        url: String,
        username: String,
        password: Option<String>,
        method: Option<String>,
    ) -> AuthenticationRequest {
        AuthenticationRequest {
            method,
            url,
            username,
            password,
        }
    }

    pub fn authenticate(&self) -> Result<Option<HeaderMap>, &'static str> {
        let basic_auth = "Basic".to_string();
        let digest_auth = "Digest".to_string();

        match self.requires_authentication() {
            Ok(Some(header_value)) => {
                if let Some((auth_type, rest)) = self.authentication_type(&header_value) {
                    if auth_type == basic_auth {
                        self.do_basic_auth()
                    } else if auth_type == digest_auth {
                        self.do_digest_auth(&rest)
                    } else {
                        Err("Authentication type is not supported yet.")
                    }
                } else {
                    Err("Incorrect WWW-Authenticate header.")
                }
            }
            Ok(None) => Ok(None),
            Err(message) => Err(message),
        }
    }

    fn authentication_type(&self, header: &str) -> Option<(String, String)> {
        header.find(' ').map(|slice_at| {
            (
                header[0..slice_at].to_string(),
                header[slice_at + 1..header.len()].to_string(),
            )
        })
    }

    fn do_basic_auth(&self) -> Result<Option<HeaderMap>, &'static str> {
        let mut data = self.username.clone();
        data.push(':');
        if let Some(ref pass) = self.password {
            data.push_str(&pass[..]);
        }
        let header = format!("Basic {}", base64::encode(&data));
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(header.as_str()).unwrap());
        Ok(Some(headers))
    }

    fn do_digest_auth(&self, header_value: &str) -> Result<Option<HeaderMap>, &'static str> {
        if self.method.is_none() {
            return Err("Method required for digest authentication.");
        }

        let uri = self.get_request_path();
        let trimmed = header_value.trim().to_string();
        let collected = trimmed.split(',').map(|s| s.trim()).collect();
        let cnonce = Uuid::new_v4().to_string();

        let password = self.password.clone().unwrap_or_else(|| "".to_string());
        let method = self.method.clone().unwrap();
        let realm = self
            .find_string_value(&collected, "realm")
            .unwrap_or_else(|| "".to_string());
        let nonce = self
            .find_string_value(&collected, "nonce")
            .unwrap_or_else(|| "".to_string());
        let qop = self
            .find_string_value(&collected, "qop")
            .unwrap_or_else(|| "".to_string());

        let mut nonces = NONCES
            .lock()
            .expect("Failed to acquire NONCES lock, lock poisoned!");
        let nc = nonces.entry(nonce.clone()).or_insert(1);

        let ha1 = md5::compute(format!("{}:{}:{}", self.username.clone(), realm, password));
        let ha2 = md5::compute(format!("{}:{}", method, uri));
        let response = md5::compute(format!(
            "{:x}:{}:{:0>8}:{}:{}:{:x}",
            ha1, nonce, nc, cnonce, qop, ha2
        ));
        let auth_header = format!(
            "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", \
             uri=\"{}\", qop={}, nc={:0>8}, cnonce=\"{}\", \
             response=\"{:x}\"",
            self.username, realm, nonce, uri, qop, nc, cnonce, response
        );

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(auth_header.as_str()).unwrap());
        Ok(Some(headers))
    }

    fn get_request_path(&self) -> String {
        let parsed_url = Url::parse(&self.url).unwrap();
        let path = parsed_url.path();
        match parsed_url.query() {
            Some(qs) => format!("{}?{}", path, qs),
            None => path.to_string(),
        }
    }

    fn find_string_value(&self, parts: &Vec<&str>, field: &str) -> Option<String> {
        for p in parts {
            if p.starts_with(&field) {
                let formatted = format!("{}=", field);
                return Some(self.unquote(&p.replace(&formatted, "")));
            }
        }
        None
    }

    fn unquote(&self, input: &str) -> String {
        let mod1 = input.trim_start_matches('\"').to_string();
        mod1.trim_start_matches('\"').to_string()
    }

    fn requires_authentication(&self) -> Result<Option<String>, &'static str> {
        let client = Client::new();
        match client.head(&self.url).send() {
            Ok(ref mut res) => {
                match res.headers().get(WWW_AUTHENTICATE) {
                    Some(raw) => {
                        // debug!("WWW-Authenticate is: {}",
                        //    String::from_utf8(raw.get(0).unwrap().clone()).unwrap());
                        Ok(Some(
                            raw.to_str().unwrap().to_string()
                        ))
                    }
                    None => Ok(None),
                }
            }
            Err(_) => {
                // debug!("Error while issuing HEAD request. Reason: {}",
                //    e.to_string());
                Err("Could not issue a HTTP request.")
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_wrong_url_error() {
        let url = "http://httpbin.orgmeh/digest-auth/auth/user/passwd".to_string();
        let username = "user".to_string();
        let password = Some("passwd".to_string());
        let method = None;
        let ar = AuthenticationRequest::new(url, username, password, method);
        let result = ar.authenticate();
        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_digest_auth_no_method() {
        let url = "http://httpbin.org/digest-auth/auth/user/passwd".to_string();
        let username = "user".to_string();
        let password = Some("passwd".to_string());
        let method = None;
        let ar = AuthenticationRequest::new(url, username, password, method);
        let result = ar.authenticate();
        assert_eq!(result.is_err(), true);
    }

    #[test]
    fn test_basic_auth_works() {
        let url = "http://httpbin.org/basic-auth/user/passwd".to_string();
        let username = "user".to_string();
        let password = Some("passwd".to_string());
        let method = None;
        let ar = AuthenticationRequest::new(url, username, password, method);
        let result = ar.authenticate();
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_digest_auth_works() {
        let url = "http://httpbin.org/digest-auth/auth/user/passwd".to_string();
        let username = "user".to_string();
        let password = Some("passwd".to_string());
        let method = Some("POST".to_string());
        let ar = AuthenticationRequest::new(url, username, password, method);
        let result = ar.authenticate();
        assert_eq!(result.is_ok(), true);
    }
}
