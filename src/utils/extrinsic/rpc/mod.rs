/*
   Copyright 2019 Supercomputing Systems AG

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.

*/

pub use client::XtStatus;
use client::*;
use futures::channel::mpsc::Sender as ThreadOut;

#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use ws::connect;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

pub mod client;

#[cfg(not(target_arch = "wasm32"))]
pub fn start_rpc_client_thread(
    url: String,
    jsonreq: String,
    result_in: ThreadOut<String>,
    on_message_fn: OnMessageFn,
) {
    match thread::Builder::new()
        .name("client".to_owned())
        .spawn(move || {
            match connect(url, |out| RpcClient {
                out,
                request: jsonreq.clone(),
                result: result_in.clone(),
                on_message_fn,
            }) {
                Ok(_) => (),
                Err(err) => {
                    error!("could not spawn rpc client; {}", &err);
                }
            }
        }) {
        Ok(_) => (),
        Err(err) => {
            error!("could not spawn rpc client; {}", &err);
        }
    };
}

#[cfg(target_arch = "wasm32")]
pub fn start_rpc_client_thread(
    url: String,
    jsonreq: String,
    result_in: ThreadOut<String>,
    on_message_fn: OnMessageFn,
) {
    let ws = match WebSocket::new(&url) {
        Ok(result) => result,
        Err(err) => {
            error!(
                "create new websocket; {}",
                &err.as_string().unwrap_or("".to_string())
            );
            return;
        }
    };
    let ws_c = ws.clone();
    debug!("open websocket");
    let on_message = {
        Closure::wrap(Box::new(move |evt: MessageEvent| {
            let msg = match evt.data().as_string() {
                Some(value) => value,
                None => {
                    error!("Can't convert received data to a string");
                    return;
                }
            };
            let _res_e = (on_message_fn)(&msg, &ws_c, result_in.clone());
        }) as Box<dyn FnMut(MessageEvent)>)
    };

    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();
    let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
        debug!("error event: {:?}", e);
    }) as Box<dyn FnMut(ErrorEvent)>);
    ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
    onerror_callback.forget();
    let cloned_ws = ws.clone();
    let onopen_callback = Closure::wrap(Box::new(move |_| {
        debug!("sending message: {:?}", jsonreq);
        match cloned_ws.send_with_str(&jsonreq) {
            Ok(_) => debug!("message successfully sent"),
            Err(err) => debug!("error sending message: {:?}", err),
        }
    }) as Box<dyn FnMut(JsValue)>);
    ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();
}
