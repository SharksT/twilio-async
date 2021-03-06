#![allow(dead_code)]

#[macro_use]
mod macros;
mod call;
mod conference;
pub mod error;
mod message;
mod recording;
pub mod twiml;

pub use crate::{call::*, conference::*, error::*, message::*, recording::*};

use async_trait::async_trait;
use hyper::{client::HttpConnector, Body, Client, Method, Request};
use hyper_tls::HttpsConnector;
use serde::Deserialize;
use std::borrow::Borrow;

pub use typed_headers::{Authorization, Credentials};
pub use url::{form_urlencoded, Url};

#[derive(Debug)]
pub struct Twilio {
    sid: String,
    auth: Authorization,
    client: Client<HttpsConnector<HttpConnector>, hyper::Body>,
}

pub type TwilioResp<T> = Result<Option<T>, TwilioErr>;

impl Twilio {
    pub fn new<S, P>(sid: S, token: P) -> TwilioResult<Twilio>
    where
        S: Into<String>,
        P: AsRef<str>,
    {
        let sid = sid.into();
        let client = Client::builder().build::<_, hyper::Body>(HttpsConnector::new());

        Ok(Twilio {
            auth: Authorization(Credentials::basic(&sid, token.as_ref())?),
            sid,
            client,
        })
    }

    pub fn send_msg<'a>(&'a self, from: &'a str, to: &'a str, body: &'a str) -> SendMsg<'a> {
        SendMsg {
            msg: Msg::new(from, to, body),
            client: &self,
        }
    }

    pub fn msg<'a>(&'a self, message_sid: &'a str) -> GetMessage<'a> {
        GetMessage {
            message_sid,
            client: &self,
        }
    }

    pub fn msgs(&self) -> Messages<'_> {
        Messages { client: &self }
    }

    pub fn call<'a>(&'a self, from: &'a str, to: &'a str, url: &'a str) -> SendCall<'a> {
        SendCall {
            call: Call::new(from, to, url),
            client: &self,
        }
    }

    pub fn conference<'a>(&'a self, sid: &'a str) -> GetConference<'a> {
        GetConference {
            conference: Conference::new(sid),
            client: &self,
        }
    }

    pub fn conferences(&self) -> Conferences<'_> {
        Conferences { client: &self }
    }

    pub fn recording<'a>(&'a self, sid: &'a str) -> GetRecording<'a> {
        GetRecording {
            recording: Recording::new(sid),
            client: &self,
        }
    }

    pub fn recordings(&self) -> Recordings<'_> {
        Recordings { client: &self }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum TwilioJson<T> {
    Success(T),
    Fail {
        code: usize,
        message: String,
        status: usize,
    },
}

#[async_trait]
pub trait Execute {
    fn request<U>(
        &self,
        method: Method,
        url: U,
        body: Option<String>,
    ) -> Result<Request<Body>, TwilioErr>
    where
        U: AsRef<str>;
    async fn execute<U, D>(
        &self,
        method: Method,
        url: U,
        body: Option<String>,
    ) -> TwilioResp<TwilioJson<D>>
    where
        U: AsRef<str> + Send,
        D: for<'de> serde::Deserialize<'de>;
}

#[async_trait]
pub trait TwilioRequest: Execute {
    type Resp: for<'de> serde::Deserialize<'de>;
    async fn run(&self) -> TwilioResp<TwilioJson<Self::Resp>>;
}

pub fn encode_pairs<I, K, V>(pairs: I) -> Option<String>
where
    K: AsRef<str>,
    V: AsRef<str>,
    I: IntoIterator,
    I::Item: Borrow<(K, V)>,
{
    let mut partial = form_urlencoded::Serializer::new(String::new());
    for pair in pairs {
        let &(ref k, ref v) = pair.borrow();
        partial.append_pair(k.as_ref(), v.as_ref());
    }
    let encoded = partial.finish();
    Some(encoded)
}

pub fn url_encode<I, K, V>(pairs: I) -> String
where
    K: AsRef<str>,
    V: AsRef<str>,
    I: IntoIterator,
    I::Item: Borrow<(K, V)>,
{
    pairs
        .into_iter()
        .map(|pair| {
            let &(ref k, ref v) = pair.borrow();
            format!("{}={}", k.as_ref(), v.as_ref())
        })
        .fold(String::new(), |mut acc, item| {
            acc.push_str(&item);
            acc.push_str("&");
            acc
        })
}
