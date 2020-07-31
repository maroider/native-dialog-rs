use std::borrow::Cow;

#[cfg(feature = "async")]
use std::{task::Waker, thread};

use crate::{Dialog, MessageAlert, MessageConfirm, MessageType, Result};

#[cfg(feature = "async")]
use crate::AsyncDialog;

impl Dialog for MessageAlert<'_> {
    type Output = ();

    fn show(self) -> Result<Self::Output> {
        super::process_init();

        message_box(MessageBoxParams {
            title: self.title.into(),
            text: self.text.into(),
            typ: self.typ,
            ask: false,
        })?;
        Ok(())
    }

    #[cfg(feature = "async")]
    fn create_async(self) -> AsyncDialog<Result<Self::Output>> {
        super::process_init();

        message_box_async(
            MessageBoxParams {
                title: self.title.into(),
                text: self.text.into(),
                typ: self.typ,
                ask: false,
            },
            |res| res.map(|_| ()),
        )
    }
}

impl Dialog for MessageConfirm<'_> {
    type Output = bool;

    fn show(self) -> Result<Self::Output> {
        super::process_init();

        message_box(MessageBoxParams {
            title: self.title.into(),
            text: self.text.into(),
            typ: self.typ,
            ask: true,
        })
    }

    #[cfg(feature = "async")]
    fn create_async(self) -> AsyncDialog<Result<Self::Output>> {
        super::process_init();

        message_box_async(
            MessageBoxParams {
                title: self.title.into(),
                text: self.text.into(),
                typ: self.typ,
                ask: true,
            },
            |res| res,
        )
    }
}

struct MessageBoxParams<'a> {
    title: Cow<'a, str>,
    text: Cow<'a, str>,
    typ: MessageType,
    ask: bool,
}

fn message_box(params: MessageBoxParams) -> Result<bool> {
    use std::ffi::OsStr;
    use std::iter::once;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use winapi::um::winuser::{
        MessageBoxW, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MB_YESNO,
    };

    let text: Vec<u16> = OsStr::new(params.text.as_ref())
        .encode_wide()
        .chain(once(0))
        .collect();

    let caption: Vec<u16> = OsStr::new(params.title.as_ref())
        .encode_wide()
        .chain(once(0))
        .collect();

    let u_type = match params.typ {
        MessageType::Info => MB_ICONINFORMATION,
        MessageType::Warning => MB_ICONWARNING,
        MessageType::Error => MB_ICONERROR,
    } | if params.ask { MB_YESNO } else { MB_OK };

    let ret = super::with_visual_styles(|| unsafe {
        MessageBoxW(null_mut(), text.as_ptr(), caption.as_ptr(), u_type)
    });

    match ret {
        0 => Err(std::io::Error::last_os_error())?,
        x => Ok(x == IDYES),
    }
}

#[cfg(feature = "async")]
fn message_box_async<'a, F, T>(
    params: MessageBoxParams<'a>,
    map_result: F,
) -> AsyncDialog<Result<T>>
where
    F: FnOnce(Result<bool>) -> Result<T> + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    let params = MessageBoxParams {
        title: params.title.into_owned().into(),
        text: params.text.into_owned().into(),
        typ: params.typ,
        ask: params.ask,
    };

    let (sender, receiver) = crossbeam_channel::bounded(1);

    let spawn = move |waker: Option<Waker>| {
        thread::spawn(move || {
            let res = message_box(params);
            waker.map(|waker| waker.wake());
            // Discard the result since there isn't anything meaningful to do if there's an error.
            let _ = sender.send(map_result(res));
        });
    };

    AsyncDialog::new(spawn, receiver)
}
