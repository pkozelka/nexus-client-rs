use reqwest::header::{AUTHORIZATION, HeaderValue};

/// Computes basic auth.
///
/// Unfortunately, function [reqwest::util::basic_auth] is not public in the library, so we call it via its calling code.
/// Also, merging this PR would make it unnecessary: https://github.com/seanmonstar/reqwest/pull/1398
pub fn basic_auth<U, P>(username: U, password: Option<P>) -> HeaderValue
    where
        U: std::fmt::Display,
        P: std::fmt::Display,
{
    let fake_client = reqwest::Client::new();
    let fake_request = fake_client.get("https://example.com")
        .basic_auth(username, password)
        .build()
        .unwrap();
    fake_request.headers().get(AUTHORIZATION)
        .unwrap()
        .clone()
}
