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
    cobject::{CObjectRef, ExternCObject, OwnedCObject},
    initialize_dart_api_dl,
    port::{DartPortId, NativeMessageHandler, NativeRecvPort, PortPostMessageFailed, SendPort},
    DartRuntime, InitData, InitializationFailed,
};
use thiserror::Error;

static LOGGER: Lazy<Mutex<File>> = Lazy::new(|| Mutex::new(File::create("/tmp/yolo.txt").unwrap()));

fn log(msg: impl Into<String>) {
    let mut l = LOGGER.lock().unwrap();
    let _ = l.write_all(msg.into().as_bytes());
    let _ = l.write_all(b"\n");
    let _ = l.flush();
}

#[no_mangle]
pub unsafe extern "C" fn initialize(init_data: InitData) -> bool {
    log("pre-init");
    initialize_dart_api_dl(init_data).is_ok()
}

#[no_mangle]
pub unsafe extern "C" fn setup_cmd_handler(
    // We can't use Dart_Port_DL as cbindgen doesn't know about bindgen.
    respond_to: i64,
) -> bool {
    setup_cmd_handler_inner(respond_to).is_ok()
}

unsafe fn setup_cmd_handler_inner(respond_to: DartPortId) -> Result<(), SetupError> {
    log("setup-0");
    let rt = DartRuntime::instance()?;
    log("setup-1");
    let send_port = rt
        .send_port_from_raw(respond_to)
        .ok_or(SetupError::PortCreatingFailed)?;
    log("setup-2");
    let adder_send_port = rt
        .native_recv_port::<CmdHandler>()
        .ok_or(SetupError::PortCreatingFailed)?
        .leak();
    log("setup-3");
    let mut cobj = OwnedCObject::send_port(adder_send_port);
    log("setup-4");
    send_port.post_cobject_mut(&mut cobj)?;
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
    PortCreatingFailed,
    PortPostMessageFailed(#[from] PortPostMessageFailed),
}

struct CmdHandler;

impl CmdHandler {
    fn handle_cmd(
        rt: DartRuntime,
        respond_to: SendPort,
        slice: &[&ExternCObject],
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
                    .ok_or("first number not a number")?;
                let b = slice
                    .get(2)
                    .ok_or("missing 2nt number")?
                    .as_int(rt)
                    .ok_or("second number not a number")?;
                let chan = ADDER_THREAD.lock().unwrap().clone();
                chan.send((a, b, respond_to))
                    .map_err(|_| "Adder was shutdown".to_owned())?;
            }
            "hy" => {
                let msg = OwnedCObject::string("hy hy ho").map_err(|v| v.to_string())?;
                respond_to.post_cobject(msg).map_err(|v| v.to_string())?;
            }
            "send etd" => {
                let msg = OwnedCObject::external_typed_data(vec![1u8, 12, 33]);
                respond_to.post_cobject(msg).map_err(|v| v.to_string())?;
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

    fn handle_message(rt: DartRuntime, _ourself: &NativeRecvPort, msg: &mut ExternCObject) {
        log(format!("handle-msg-0: {:?}", msg));
        if let Ok(CObjectRef::Array(slice)) = msg.value_ref(rt) {
            if let Some(respond_to) = slice.get(0).and_then(|o| o.as_send_port(rt)).flatten() {
                if let Err(err) = Self::handle_cmd(rt, respond_to, &slice[1..]) {
                    if let Ok(mut err) = OwnedCObject::string(format!("Error: {}", err)) {
                        if let Err(_) = respond_to.post_cobject_mut(&mut err) {
                            log(format!("Failed to post error: {:?}", err));
                        }
                    }
                }
            }
        }
    }
}
