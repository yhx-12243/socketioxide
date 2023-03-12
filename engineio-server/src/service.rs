use crate::{
    body::ResponseBody,
    engine::{EngineIo, EngineIoConfig},
    futures::ResponseFuture,
    websocket::upgrade_ws_connection,
};
use http::{Method, Request};
use http_body::Body;
use hyper::{service::Service, Response};
use std::{
    fmt::Debug,
    task::{Context, Poll},
};

#[derive(Debug, Clone)]
pub struct EngineIoService<S> {
    inner: S,
    engine: EngineIo,
}

impl<S> EngineIoService<S> {
    pub fn from_config(inner: S, config: EngineIoConfig) -> Self {
        EngineIoService {
            inner,
            engine: EngineIo::from_config(config),
        }
    }
}

// let (sender, body) = hyper::Body::channel();
            // let res = Response::builder()
            //         .status(200)
            //         .body(body)
            //         .unwrap();
            // ResponseFuture::new(res);
impl<ReqBody, ResBody, S> Service<Request<ReqBody>> for EngineIoService<S>
where
    ResBody: Body,
    ReqBody: Send + 'static + Debug,
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = Response<ResponseBody<ResBody>>;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        if req.uri().path().starts_with("/engine.io") {
            match RequestType::parse(&req) {
                RequestType::Invalid => ResponseFuture::empty_response(400),
                RequestType::HttpOpen => todo!(),
                RequestType::HttpPoll => todo!(),
                RequestType::WebsocketUpgrade => upgrade_ws_connection(req),
            }
        } else {
            ResponseFuture::new(self.inner.call(req))
        }
    }
}

enum RequestType {
    Invalid,
    HttpOpen,
    HttpPoll,
    WebsocketUpgrade,
}

impl RequestType {
    fn parse<B>(req: &Request<B>) -> Self {
        if let Some(query) = req.uri().query() {
            if !query.contains("EIO=4")
                || req.method() != Method::GET && req.method() != Method::POST
            {
                return RequestType::Invalid;
            }

            if query.contains("transport=polling") {
                if query.contains("sid=") {
                    RequestType::HttpPoll
                } else if req.method() == Method::GET {
                    RequestType::HttpOpen
                } else {
					RequestType::Invalid
				}
            } else if query.contains("transport=websocket") && req.method() == Method::GET {
                RequestType::WebsocketUpgrade
            } else {
                RequestType::Invalid
            }
        } else {
            RequestType::Invalid
        }
    }
}