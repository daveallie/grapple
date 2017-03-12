use crypto::digest::Digest;
use crypto::md5::Md5;
use reqwest::Client;
use reqwest::header::Headers;
use serialize::base64::{ToBase64, Standard, Config, Newline};
use url::Url;
use uuid::Uuid;

pub struct AuthenticationRequest {
    method: Option<String>,
    username: String,
    password: Option<String>,
    url: String,
}

impl AuthenticationRequest {
    pub fn new(url: String,
               username: String,
               password: Option<String>,
               method: Option<String>)
               -> AuthenticationRequest {
        AuthenticationRequest {
            method: method,
            url: url,
            username: username,
            password: password,
        }
    }

    pub fn authenticate(&self) -> Result<Option<Headers>, &'static str> {
        let basic_auth = "Basic".to_string();
        let digest_auth = "Digest".to_string();

        match self.requires_authentication() {
            Ok(Some(header_value)) => {
                if let Some((auth_type, rest)) = self.authentication_type(&header_value) {
                    if auth_type == basic_auth {
                        self.do_basic_auth()
                    } else if auth_type == digest_auth {
                        self.do_digest_auth(rest)
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

    fn authentication_type(&self, header: &String) -> Option<(String, String)> {
        header.find(" ").map(|slice_at| {
            (header[0..slice_at].to_string(), header[slice_at + 1..header.len()].to_string())
        })
    }

    fn do_basic_auth(&self) -> Result<Option<Headers>, &'static str> {
        let mut data = self.username.clone();
        data.push(':');
        if let Some(ref pass) = self.password {
            data.push_str(&pass[..]);
        }
        let header = &data.as_bytes().to_base64(Config {
            char_set: Standard,
            newline: Newline::CRLF,
            pad: true,
            line_length: None,
        });
        let mut headers = Headers::new();
        headers.set_raw("Authorization",
                        vec![format!("Basic {}", header[..].to_string()).into_bytes().to_vec()]);
        Ok(Some(headers))
    }

    fn do_digest_auth(&self, header_value: String) -> Result<Option<Headers>, &'static str> {
        if self.method.is_none() {
            return Err("Method required for digest authentication.");
        }

        let uri = self.get_request_path();
        let nc = "00000001".to_string();
        let trimmed = header_value.trim().to_string();
        let collected = trimmed.split(",").map(|s| s.trim()).collect();
        let cnonce = Uuid::new_v4().to_string();

        let password = self.password.clone().unwrap_or("".to_string());
        let method = self.method.clone().unwrap();
        let realm = self.find_string_value(&collected, "realm".to_string())
            .unwrap_or("".to_string());
        let nonce = self.find_string_value(&collected, "nonce".to_string())
            .unwrap_or("".to_string());
        let qop = self.find_string_value(&collected, "qop".to_string()).unwrap_or("".to_string());
        let opaque = self.find_string_value(&collected, "opaque".to_string())
            .unwrap_or("".to_string());

        let ha1 =
            self.input_to_md5_str(format!("{}:{}:{}", self.username.clone(), realm, password));
        let ha2 = self.input_to_md5_str(format!("{}:{}", method, uri));
        let response =
            self.input_to_md5_str(format!("{}:{}:{}:{}:{}:{}", ha1, nonce, nc, cnonce, qop, ha2));
        let auth_header = format!("Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", \
                                   uri=\"{}\", qop={}, nc={}, cnonce=\"{}\", response=\"{}\", \
                                   opaque=\"{}\"",
                                  self.username,
                                  realm,
                                  nonce,
                                  uri,
                                  qop,
                                  nc,
                                  cnonce,
                                  response,
                                  opaque);

        let mut headers = Headers::new();
        headers.set_raw("Authorization", vec![auth_header.into_bytes().to_vec()]);
        Ok(Some(headers))
    }

    fn get_request_path(&self) -> String {
        let parsed_url = Url::parse(&self.url).unwrap();
        let path = parsed_url.path();
        match parsed_url.query() {
            Some(qs) => format!("{}?{}", path, qs),
            None => format!("{}", path),
        }
    }

    fn input_to_md5_str(&self, input: String) -> String {
        let mut digest = Md5::new();
        let input_bytes = input.into_bytes();
        digest.input(&input_bytes);
        let result_str = digest.result_str();
        digest.reset();
        result_str
    }

    fn find_string_value(&self, parts: &Vec<&str>, field: String) -> Option<String> {
        for p in parts {
            if p.starts_with(&field) {
                let formatted = format!("{}=", field);
                return Some(self.unquote(p.replace(&formatted, "")));
            }
        }
        None
    }

    fn unquote(&self, input: String) -> String {
        let mod1 = input.trim_left_matches("\"").to_string();
        mod1.trim_right_matches("\"").to_string()
    }

    fn requires_authentication(&self) -> Result<Option<String>, &'static str> {
        let client = Client::new().unwrap();
        match client.head(&self.url).send() {
            Ok(ref mut res) => {
                match res.headers().get_raw("WWW-Authenticate") {
                    Some(raw) => {
                        // debug!("WWW-Authenticate is: {}",
                        //    String::from_utf8(raw.get(0).unwrap().clone()).unwrap());
                        Ok(Some(String::from_utf8(raw.get(0).unwrap().clone()).unwrap()))
                    }
                    None => return Ok(None),
                }
            }
            Err(e) => {
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
