use std::rc::Rc;

use yew::prelude::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dialog {
    enabled: bool,
    state: Option<DialogState>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DialogState {
    DeleteSingle {
        name: String,
        ondelete: Callback<()>,
    },
}

pub enum DialogAction {
    Close,
    Disable,
    Request(DialogState),
}

impl Reducible for Dialog {
    type Action = DialogAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            DialogAction::Close => Rc::new(Default::default()),
            DialogAction::Disable => Rc::new(Self {
                enabled: false,
                ..(&*self).clone()
            }),
            DialogAction::Request(state) => Rc::new(Self {
                enabled: true,
                state: Some(state),
            }),
        }
    }
}

pub fn build_dialog(dialog: &UseReducerHandle<Dialog>) -> Option<Html> {
    let oncancel = if dialog.enabled {
        let dialog = dialog.clone();
        Some(move |_| dialog.dispatch(DialogAction::Close))
    } else {
        None
    };

    match dialog.state.as_ref()? {
        DialogState::DeleteSingle { name, ondelete } => {
            let ondelete = if dialog.enabled {
                let ondelete = ondelete.clone();
                Some(move |_| ondelete.emit(()))
            } else {
                None
            };

            let btn_status = if dialog.enabled { "" } else { "btn-disabled" };

            Some(html! {
                <dialog class="modal" open=true >
                    <div class="modal-box">
                        <form method="dialog">
                            <button
                                class={ format!("btn btn-sm btn-circle btn-ghost {btn_status} absolute right-2 top-2") }
                                onclick={ oncancel.clone() }
                            >
                                { "✕" }
                            </button>
                        </form>
                        <h3 class="text-lg font-bold">{ "Warning" }</h3>
                        <p class="py-4">{ format!("Do you want to delete {name}?") }</p>
                        <div class="flex flex-row-reverse join">
                            <button
                                class={ format!("btn btn-error {btn_status}") }
                                onclick={ ondelete }
                            >
                                { "Delete" }
                            </button>
                            <button
                                class={ format!("btn btn-ghost {btn_status}") }
                                onclick={ oncancel }
                            >
                                { "Cancel" }
                            </button>
                        </div>
                    </div>
                </dialog>
            })
        }
    }
}
