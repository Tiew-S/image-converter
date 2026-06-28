use std::{collections::HashMap, path::PathBuf};

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{
    button::*,
    label::Label,
    menu::{ContextMenuExt, PopupMenuItem},
    scroll::ScrollableElement,
    select::{Select, SelectState},
    spinner::Spinner,
    *,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::things::{self, ImageConverter};

#[derive(PartialEq, Debug, Clone)]
pub enum ConversionState {
    Untouched,
    Processing,
    Success,
    Fail(String),
}

pub struct ConvertView {
    converter: ImageConverter,
    conversion_in_progress: bool,
    conversion_states: HashMap<std::path::PathBuf, ConversionState>,
    add_image_button_disabled: bool,
    end_format_select: Entity<SelectState<Vec<&'static str>>>,
}

impl ConvertView {
    pub fn new<T>(window: &mut Window, cx: &mut Context<T>) -> Self {
        Self {
            converter: ImageConverter::new(),
            conversion_states: HashMap::new(),
            conversion_in_progress: false,
            add_image_button_disabled: false,
            end_format_select: cx.new(|cx| {
                SelectState::new(
                    vec!["PNG", "JPEG", "GIF", "WEBP", "TIFF", "AVIF"],
                    Some(IndexPath::default()),
                    window,
                    cx,
                )
            }),
        }
    }

    fn controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .child({
                Button::new("add_image")
                    .ghost()
                    .label("Add images")
                    .disabled(self.add_image_button_disabled)
                    .on_click(cx.listener(move |_, _, _, cx| {
                        cx.spawn(async |app, cx| {
                            app.update(cx, |app, cx| {
                                app.add_image_button_disabled = true;
                                cx.notify();
                                let paths = rfd::FileDialog::new()
                                    .add_filter(
                                        "Images",
                                        &["png", "jpg", "jpeg", "gif", "webp", "svg", "pdf"],
                                    )
                                    .pick_files();

                                if let Some(paths) = paths {
                                    paths.iter().for_each(|path| {
                                        let _ = app.converter.add_image_from_path(path);
                                    })
                                }
                                app.add_image_button_disabled = false;
                                cx.notify();
                            })
                            .ok();
                        })
                        .detach();
                    }))
            })
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .child(Select::new(&self.end_format_select))
                    .child(
                        Button::new("convert")
                            .disabled(self.conversion_in_progress)
                            .primary()
                            .label("Convert")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let fmt = match this.end_format_select.read(cx).selected_value() {
                                    Some(&"PNG") => image::ImageFormat::Png,

                                    Some(&"JPEG") => image::ImageFormat::Jpeg,

                                    Some(&"GIF") => image::ImageFormat::Gif,

                                    Some(&"WEBP") => image::ImageFormat::WebP,

                                    Some(&"TIFF") => image::ImageFormat::Tiff,

                                    Some(&"AVIF") => image::ImageFormat::Avif,
                                    _ => return,
                                };

                                cx.spawn(async move |this, cx| {
                                    this.update(cx, |this, cx| {
                                        this.conversion_in_progress = true;
                                        this.converter.options =
                                            confy::load::<things::Settings>("ImageConverter", None)
                                                .unwrap_or_default()
                                                .image_convertion_options;
                                        cx.notify();
                                    })
                                    .ok();
                                    let Ok(conv) = this.read_with(cx, |c, _| c.converter.clone())
                                    else {
                                        return;
                                    };
                                    let (send, mut recv) =
                                        tokio::sync::mpsc::channel::<
                                            Option<(PathBuf, ConversionState)>,
                                        >(100);

                                    cx.background_spawn(async move {
                                        conv.get_images().par_iter().for_each(|image| {
                                            send.blocking_send(Some((
                                                image.path.clone(),
                                                ConversionState::Processing,
                                            )))
                                            .ok();
                                            let res = image.convert(&fmt, &conv.options);
                                            match res {
                                                Ok(_) => send.blocking_send(Some((
                                                    image.path.clone(),
                                                    ConversionState::Success,
                                                ))),
                                                Err(e) => {
                                                    dbg!(&e);
                                                    send.blocking_send(Some((
                                                        image.path.clone(),
                                                        ConversionState::Fail(e.to_string()),
                                                    )))
                                                }
                                            }
                                            .ok();
                                        });
                                        dbg!("Finished");
                                        send.blocking_send(None).ok();
                                    })
                                    .detach();

                                    while let Some(msg) = recv.recv().await {
                                        match msg {
                                            Some(msg) => {
                                                //dbg!(&msg);
                                                this.update(cx, |this, cx| {
                                                    this.conversion_states.insert(msg.0, msg.1);
                                                    cx.notify();
                                                })
                                                .ok();
                                            }
                                            None => break,
                                        }
                                    }

                                    this.update(cx, |this, cx| {
                                        this.conversion_in_progress = false;
                                        cx.notify();
                                    })
                                    .ok();
                                })
                                .detach();
                            })),
                    ),
            )
    }

    fn images_view(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .flex_grow_1()
            .min_h_full()
            .on_drop(
                cx.listener(move |this, external_paths: &ExternalPaths, _window, cx| {
                    for path in external_paths.paths() {
                        let _ = this.converter.add_image_from_path(path);
                    }
                    cx.notify();
                }),
            )
            .when(!self.converter.get_images().is_empty(), |d| {
                d.child(
                    Button::new("clear-images")
                        .label("Clear")
                        .ghost()
                        .xsmall()
                        .m_1()
                        .mb_0()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.converter.clear_images();
                            cx.notify();
                        })),
                )
            })
            .children(
                self.converter
                    .get_images()
                    .iter()
                    .enumerate()
                    .map(|(i, image)| {
                        let key = image
                            .path
                            .clone()
                            .into_os_string()
                            .into_string()
                            .unwrap_or(String::new());
                        let hovering = window.use_keyed_state(key.clone(), cx, |_, _| false);
                        let hovering_clone = hovering.clone();
                        let conversion_state = self
                            .conversion_states
                            .get(&image.path)
                            .unwrap_or(&ConversionState::Untouched);
                        let path = image.path.clone();
                        let path2 = image.path.clone();

                        div()
                            .id(key.clone())
                            .on_hover(move |hover, _, cx| hovering.write(cx, *hover))
                            .h_flex()
                            .gap_2()
                            .p_2()
                            .child(
                                div()
                                    .id(key)
                                    .gap_1()
                                    .h_flex()
                                    .flex_grow_1()
                                    .text_ellipsis()
                                    .child(Label::new(
                                        image
                                            .path
                                            .file_name()
                                            .and_then(|f| f.to_str())
                                            .unwrap_or(""),
                                    ))
                                    .map(|d| match conversion_state {
                                        ConversionState::Untouched => d,
                                        ConversionState::Processing => d.child(Spinner::new()),
                                        ConversionState::Success => d.child(
                                            Icon::new(IconName::Check)
                                                .text_color(cx.theme().success),
                                        ),
                                        ConversionState::Fail(e) => d
                                            .child(
                                                Icon::new(IconName::Close)
                                                    .text_color(cx.theme().danger),
                                            )
                                            .child(
                                                Label::new(e)
                                                    .ml_1()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .italic(),
                                            ),
                                    })
                                    .on_double_click(move |_, _, _| {
                                        let _ = open::that(path.clone());
                                    }),
                            )
                            .map(|d| {
                                let path = image.path.clone();

                                if hovering_clone.read(cx).clone() {
                                    d.child(
                                        Button::new("close")
                                            .absolute()
                                            .right_2()
                                            .ghost()
                                            .icon(IconName::Close)
                                            .size_6()
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                cx.stop_propagation();
                                                this.converter.remove_image(i);
                                                this.conversion_states.remove(&path);
                                                cx.notify();
                                            })),
                                    )
                                } else {
                                    d.child(
                                        div()
                                            .id("no-clickey")
                                            .absolute()
                                            .right_2()
                                            .size_6()
                                            .on_any_mouse_down(|_, _, cx| cx.stop_propagation()),
                                    )
                                }
                            })
                            .map(|d| {
                                if i % 2 == 0 {
                                    d
                                } else {
                                    if i == self.converter.get_images().len() - 1 {
                                        d.bg(cx.theme().muted.alpha(0.25))
                                            .rounded_b(cx.theme().radius - px(1.))
                                    } else {
                                        d.bg(cx.theme().muted.alpha(0.25))
                                    }
                                }
                            })
                            .context_menu(move |menu, _window, _cx| {
                                let path = path2.clone();
                                let path2 = path2.clone();
                                let path3 = path2.clone();

                                menu.item(
                                    PopupMenuItem::element(move |_window, cx| {
                                        Label::new(path.to_str().unwrap_or_default())
                                            .w_48()
                                            .text_color(cx.theme().muted_foreground)
                                    })
                                    .disabled(true),
                                )
                                .item(PopupMenuItem::new("Open").on_click(move |_, _, _| {
                                    let _ = open::that(path2.clone());
                                }))
                                .item(
                                    PopupMenuItem::new("Open folder").on_click(move |_, _, _| {
                                        if let Some(p) = path3.clone().parent() {
                                            let _ = open::that(p);
                                        }
                                    }),
                                )
                            })
                    }),
            )
    }
}

impl Render for ConvertView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .gap_4()
            .flex()
            .size_full()
            .flex_col()
            .overflow_hidden()
            .child(
                div().flex_1().min_h_0().overflow_hidden().child(
                    div()
                        .h_full()
                        .overflow_y_scrollbar()
                        .child(self.images_view(window, cx)),
                ),
            )
            .child(self.controls(cx))
    }
}
