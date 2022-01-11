// Copyright 2021 Xayn AG
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    fs::File,
    io::Write,
    sync::{
        mpsc::{channel, Sender},
        Mutex,
    },
    thread,
};

use once_cell::sync::Lazy;

use dart_api_dl::{
    cobject::{CObject, CObjectMut, CObjectValuesRef},
    initialize_dart_api_dl,
    ports::{
        DartPortId,
        NativeMessageHandler,
        NativeRecvPort,
        PortCreationFailed,
        PostingMessageFailed,
        SendPort,
    },
    DartRuntime,
    InitData,
    InitializationFailed,
};
use thiserror::Error;

static LOGGER: Lazy<Mutex<File>> = Lazy::new(|| Mutex::new(File::create("/tmp/yolo.txt").unwrap()));

fn log(msg: impl Into<String>) {
    let mut l = LOGGER.lock().unwrap();
    let _ = l.write_all(msg.into().as_bytes());
    let _ = l.write_all(b"\n");
    let _ = l.flush();
}

/// Initializes the rust library.
///
/// # Safety
///
/// See `initialize_dart_api_dl` from the
/// `dart-api-dl` crate.
#[no_mangle]
pub unsafe extern "C" fn initialize(init_data: InitData) -> bool {
    log("pre-init");
    initialize_dart_api_dl(init_data).is_ok()
}

#[no_mangle]
pub extern "C" fn setup_cmd_handler(
    // We can't use Dart_Port_DL as cbindgen doesn't know about bindgen.
    respond_to: i64,
) -> bool {
    setup_cmd_handler_inner(respond_to).is_ok()
}

fn setup_cmd_handler_inner(respond_to: DartPortId) -> Result<(), SetupError> {
    log("setup-0");
    let rt = DartRuntime::instance()?;
    log("setup-1");
    let send_port = rt
        .send_port_from_raw(respond_to)
        .ok_or(SetupError::MalformedMessage)?;
    log("setup-2");
    let adder_send_port = rt.native_recv_port::<CmdHandler>()?.leak();
    log("setup-3");
    let mut cobj = CObject::send_port(adder_send_port);
    log("setup-4");
    send_port.post_cobject_mut(cobj.as_mut())?;
    log("setup-5");
    Ok(())
}

static ADDER_THREAD: Lazy<Mutex<Sender<(i64, i64, SendPort)>>> = Lazy::new(|| {
    let (sender, receiver) = channel::<(_, _, SendPort)>();
    thread::spawn(move || {
        while let Ok((a, b, send_port)) = receiver.recv() {
            let res = a + b;
            send_port.post_integer(res as i64).ok();
            if res == 0 {
                break;
            }
        }
    });
    Mutex::new(sender)
});

#[derive(Debug, Error)]
#[error("setup failed")]
enum SetupError {
    InitFailed(#[from] InitializationFailed),
    PortCreatingFailed(#[from] PortCreationFailed),
    PortPostMessageFailed(#[from] PostingMessageFailed),
    MalformedMessage,
}

struct CmdHandler;

impl CmdHandler {
    fn handle_cmd(
        rt: DartRuntime,
        respond_to: SendPort,
        slice: &[CObjectMut<'_>],
    ) -> Result<(), String> {
        let cmd = slice
            .get(0)
            .ok_or("no cmd argument")?
            .as_string(rt)
            .ok_or("1st cmd is not a string")?;

        match cmd {
            "add" => {
                let a = slice
                    .get(1)
                    .ok_or("missing 1st number")?
                    .as_int(rt)
                    .ok_or("first argument not a number")?;
                let b = slice
                    .get(2)
                    .ok_or("missing 2nd number")?
                    .as_int(rt)
                    .ok_or("second argument not a number")?;
                let chan = ADDER_THREAD.lock().unwrap().clone();
                chan.send((a, b, respond_to))
                    .map_err(|_| "Adder was shutdown".to_owned())?;
            }
            "hy" => {
                let msg = CObject::string("hy hy ho").map_err(|v| v.to_string())?;
                respond_to.post_cobject(msg).map_err(|v| v.to_string())?;
            }
            "send etd" => {
                let msg = CObject::external_typed_data(vec![1u8, 12, 33]);
                respond_to.post_cobject(msg).map_err(|v| v.to_string())?;
            }
            "panic" => {
                panic!("IT IS A PANIC");
            }
            _ => {
                return Err("Unknown Command".to_owned());
            }
        }
        Ok(())
    }
}

impl NativeMessageHandler for CmdHandler {
    const CONCURRENT_HANDLING: bool = true;
    const NAME: &'static str = "adder";

    fn handle_message(rt: DartRuntime, _ourself: &NativeRecvPort, msg: CObjectMut<'_>) {
        log(format!("handle-msg-0: {:?}", msg));
        if let Ok(CObjectValuesRef::Array(slice)) = msg.value_ref(rt) {
            if let Some(respond_to) = slice.get(0).and_then(|o| o.as_send_port(rt)).flatten() {
                if let Err(err) = Self::handle_cmd(rt, respond_to, &slice[1..]) {
                    if let Ok(mut err) = CObject::string(format!("Error: {}", err)) {
                        if respond_to.post_cobject_mut(err.as_mut()).is_err() {
                            log(format!("Failed to post error: {:?}", err.as_mut()));
                        }
                    }
                }
            }
        }
    }

    fn handle_panic(
        rt: DartRuntime,
        _ourself: &NativeRecvPort,
        data: CObjectMut<'_>,
        mut panic: CObject,
    ) {
        let value_ref = match data.value_ref(rt) {
            Ok(r) => r,
            Err(_) => return,
        };

        let slice = match value_ref {
            CObjectValuesRef::Array(slice) => slice,
            _ => return,
        };

        let send_port = match slice.get(0).and_then(|v| v.as_send_port(rt)) {
            Some(Some(s)) => s,
            _ => return,
        };

        if let Err(_err) = send_port.post_cobject_mut(panic.as_mut()) {
            //TODO
        }
    }
}
