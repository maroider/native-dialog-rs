#[cfg(feature = "async")]
use crate::AsyncDialog;
use crate::{
    r#impl::OpenDialogTarget, Dialog, Error, OpenMultipleFile, OpenSingleDir, OpenSingleFile,
    Result,
};
use std::path::{Path, PathBuf};
#[cfg(feature = "async")]
use std::{task::Waker, thread};
use wfd::{
    DialogError, DialogParams, OpenDialogResult, FOS_ALLOWMULTISELECT, FOS_FILEMUSTEXIST,
    FOS_NOREADONLYRETURN, FOS_OVERWRITEPROMPT, FOS_PATHMUSTEXIST, FOS_PICKFOLDERS,
};

impl Dialog for OpenSingleFile<'_> {
    type Output = Option<PathBuf>;

    fn show(self) -> Result<Self::Output> {
        super::process_init();

        open_dialog(OpenDialogParams {
            dir: self.dir,
            filter: self.filter,
            multiple: false,
            target: OpenDialogTarget::File,
        })
        .map(|ok| ok.map(|some| some.selected_file_path))
    }

    #[cfg(feature = "async")]
    fn create_async(self) -> AsyncDialog<Result<Self::Output>> {
        super::process_init();

        open_dialog_async(
            OpenDialogParams {
                dir: self.dir,
                filter: self.filter,
                multiple: false,
                target: OpenDialogTarget::File,
            },
            |res| res.map(|ok| ok.map(|some| some.selected_file_path)),
        )
    }
}

impl Dialog for OpenMultipleFile<'_> {
    type Output = Vec<PathBuf>;

    fn show(self) -> Result<Self::Output> {
        super::process_init();

        let result = open_dialog(OpenDialogParams {
            dir: self.dir,
            filter: self.filter,
            multiple: true,
            target: OpenDialogTarget::File,
        });

        match result {
            Ok(Some(t)) => Ok(t.selected_file_paths),
            Ok(None) => Ok(vec![]),
            Err(e) => Err(e),
        }
    }

    #[cfg(feature = "async")]
    fn create_async(self) -> AsyncDialog<Result<Self::Output>> {
        super::process_init();

        open_dialog_async(
            OpenDialogParams {
                dir: self.dir,
                filter: self.filter,
                multiple: true,
                target: OpenDialogTarget::File,
            },
            |res| match res {
                Ok(Some(t)) => Ok(t.selected_file_paths),
                Ok(None) => Ok(vec![]),
                Err(e) => Err(e),
            },
        )
    }
}

impl Dialog for OpenSingleDir<'_> {
    type Output = Option<PathBuf>;

    fn show(self) -> Result<Self::Output> {
        super::process_init();

        open_dialog(OpenDialogParams {
            dir: self.dir,
            filter: None,
            multiple: false,
            target: OpenDialogTarget::Directory,
        })
        .map(|ok| ok.map(|some| some.selected_file_path))
    }

    #[cfg(feature = "async")]
    fn create_async(self) -> AsyncDialog<Result<Self::Output>> {
        super::process_init();

        open_dialog_async(
            OpenDialogParams {
                dir: self.dir,
                filter: None,
                multiple: false,
                target: OpenDialogTarget::Directory,
            },
            |res| res.map(|ok| ok.map(|some| some.selected_file_path)),
        )
    }
}

struct OpenDialogParams<'a> {
    dir: Option<&'a Path>,
    filter: Option<&'a [&'a str]>,
    multiple: bool,
    target: OpenDialogTarget,
}

fn open_dialog(params: OpenDialogParams) -> Result<Option<OpenDialogResult>> {
    let file_types = match params.filter {
        Some(filter) => {
            let types: Vec<String> = filter.iter().map(|s| format!("*.{}", s)).collect();
            types.join(";")
        }
        None => String::new(),
    };
    let file_types = match params.filter {
        Some(_) => vec![("", file_types.as_str())],
        None => vec![],
    };

    let mut options = FOS_PATHMUSTEXIST | FOS_FILEMUSTEXIST;
    if params.multiple {
        options |= FOS_ALLOWMULTISELECT;
    }
    if params.target == OpenDialogTarget::Directory {
        options |= FOS_PICKFOLDERS;
    }

    let params = DialogParams {
        default_folder: params.dir.unwrap_or("".as_ref()),
        file_types,
        options,
        ..Default::default()
    };

    let result = wfd::open_dialog(params);

    match result {
        Ok(t) => Ok(Some(t)),
        Err(e) => match e {
            DialogError::UserCancelled => Ok(None),
            DialogError::HResultFailed { error_method, .. } => {
                Err(Error::ImplementationError(error_method))
            }
        },
    }
}

#[cfg(feature = "async")]
fn open_dialog_async<'a, F, T>(
    params: OpenDialogParams<'a>,
    map_result: F,
) -> AsyncDialog<Result<T>>
where
    F: FnOnce(Result<Option<OpenDialogResult>>) -> Result<T> + Send + Sync + 'static,
    T: Send + Sync + 'static,
{
    let dir = params.dir.map(ToOwned::to_owned);
    let filter = params
        .filter
        .as_ref()
        .map(|filter| filter.iter().map(|s| s.to_string()).collect::<Vec<_>>());
    let multiple = params.multiple;
    let target = params.target;

    let (sender, receiver) = crossbeam_channel::bounded(1);

    let spawn = move |waker: Option<Waker>| {
        thread::spawn(move || {
            let filter = filter
                .as_ref()
                .map(|filter| filter.iter().map(AsRef::as_ref).collect::<Vec<_>>());
            let filter = filter.as_ref().map(|filter| filter.as_slice());
            let res = open_dialog(OpenDialogParams {
                dir: dir.as_ref().map(AsRef::as_ref),
                filter,
                multiple,
                target,
            });
            waker.map(|waker| waker.wake());
            // Discard the result since there isn't anything meaningful to do if there's an error.
            let _ = sender.send(map_result(res));
        });
    };

    AsyncDialog::new(spawn, receiver)
}

#[allow(dead_code)]
fn save_dialog() {
    let mut _options = FOS_OVERWRITEPROMPT | FOS_PATHMUSTEXIST | FOS_NOREADONLYRETURN;
}
