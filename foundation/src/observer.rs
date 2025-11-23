use crate::{bits::_NSConcreteStackBlock, class, msg_send, sel};
use std::{ffi::CString, os::raw::c_void, sync::mpsc::Sender};

pub struct WorkspaceObserver {
    observers: Vec<*mut c_void>,
    _contexts: Vec<*mut (Sender<WorkspaceEvent>, WorkspaceEvent)>,
}

#[derive(Clone, Debug)]
pub enum WorkspaceEvent {
    AppLaunched,
    AppTerminated,
    AppActivated,
}

impl WorkspaceObserver {
    pub fn new(sender: Sender<WorkspaceEvent>) -> Self {
        let mut observers = Vec::new();
        let mut contexts = Vec::new();

        unsafe {
            let workspace_class = class("NSWorkspace");
            let workspace = msg_send!(workspace_class, sel("sharedWorkspace"));
            let notification_center = msg_send!(workspace, sel("notificationCenter"));

            let (obs, ctx) = Self::register_notification(
                notification_center,
                "NSWorkspaceDidLaunchApplicationNotification",
                sender.clone(),
                WorkspaceEvent::AppLaunched,
            );
            observers.push(obs);
            contexts.push(ctx);

            let (obs, ctx) = Self::register_notification(
                notification_center,
                "NSWorkspaceDidTerminateApplicationNotification",
                sender.clone(),
                WorkspaceEvent::AppTerminated,
            );
            observers.push(obs);
            contexts.push(ctx);

            let (obs, ctx) = Self::register_notification(
                notification_center,
                "NSWorkspaceDidActivateApplicationNotification",
                sender,
                WorkspaceEvent::AppActivated,
            );
            observers.push(obs);
            contexts.push(ctx);

            Self {
                observers,
                _contexts: contexts,
            }
        }
    }

    unsafe fn register_notification(
        notification_center: *mut c_void,
        notification_name: &str,
        sender: Sender<WorkspaceEvent>,
        event: WorkspaceEvent,
    ) -> (*mut c_void, *mut (Sender<WorkspaceEvent>, WorkspaceEvent)) {
        type AddObserverFunc = unsafe extern "C" fn(
            *mut c_void,
            *mut c_void,
            *mut c_void,
            *mut c_void,
            *mut c_void,
            *mut c_void,
        ) -> *mut c_void;

        unsafe {
            let ns_string_class = class("NSString");
            let name_cstr = CString::new(notification_name).unwrap();
            let name = msg_send!(
                ns_string_class,
                sel("stringWithUTF8String:"),
                name_cstr.as_ptr() as *mut c_void
            );

            let context = Box::into_raw(Box::new((sender, event)));
            let block = create_block(context);

            let add_observer: AddObserverFunc =
                std::mem::transmute(crate::bits::objc_msgSend as *const ());
            let observer = add_observer(
                notification_center,
                sel("addObserverForName:object:queue:usingBlock:"),
                name,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                block as *mut c_void,
            );

            (observer, context)
        }
    }
}

impl Drop for WorkspaceObserver {
    fn drop(&mut self) {
        unsafe {
            let workspace_class = class("NSWorkspace");
            let workspace = msg_send!(workspace_class, sel("sharedWorkspace"));
            let notification_center = msg_send!(workspace, sel("notificationCenter"));

            for &observer in &self.observers {
                msg_send!(notification_center, sel("removeObserver:"), observer);
            }

            for &context in &self._contexts {
                let _ = Box::from_raw(context);
            }
        }
    }
}

#[repr(C)]
struct BlockDescriptor {
    reserved: usize,
    size: usize,
}

#[repr(C)]
struct Block {
    isa: *const c_void,
    flags: i32,
    reserved: i32,
    invoke: extern "C" fn(*mut Block, *mut c_void),
    descriptor: *const BlockDescriptor,
    context: *mut c_void,
}

extern "C" fn block_callback(block: *mut Block, _notification: *mut c_void) {
    unsafe {
        let context = (*block).context as *mut (Sender<WorkspaceEvent>, WorkspaceEvent);
        let (sender, event) = &*context;
        let event_copy = event.clone();
        let _ = sender.send(event_copy);
    }
}

static BLOCK_DESCRIPTOR: BlockDescriptor = BlockDescriptor {
    reserved: 0,
    size: size_of::<Block>(),
};

unsafe fn create_block(context: *mut (Sender<WorkspaceEvent>, WorkspaceEvent)) -> *mut Block {
    let block = unsafe {
        Box::new(Block {
            isa: _NSConcreteStackBlock,
            flags: 0x50000000,
            reserved: 0,
            invoke: block_callback,
            descriptor: &BLOCK_DESCRIPTOR,
            context: context as *mut c_void,
        })
    };

    Box::into_raw(block)
}
