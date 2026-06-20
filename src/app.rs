use std::ffi::OsStr;

use gpui::*;
use gpui_component::{
    button::*,
    input::{Input, InputState},
    label::Label,
    scroll::ScrollableElement,
    select::{Select, SelectState},
    *,
};
use tap::Pipe;

use crate::things;
use crate::things::{ConversionState, ImageConverter};

pub struct ConvertView {
    converter: ImageConverter,
    add_image_button_disabled: bool,
    convert_button_disabled: bool,
    end_format_select: Entity<SelectState<Vec<&'static str>>>,
}

impl ConvertView {
    pub fn new<T>(window: &mut Window, cx: &mut Context<T>) -> Self {
        Self {
            converter: ImageConverter::new(),
            add_image_button_disabled: false,
            convert_button_disabled: false,
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
}

impl Render for ConvertView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .size_full()
            .flex()
            .v_flex()
            .flex_col_reverse()
            .overflow_hidden()
            .child(
                div()
                    .pt_2()
                    .border_t_1()
                    .border_color(cx.theme().border)
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
                                            .add_filter("Images", &["png", "jpeg", "gif", "webp", "pdf"])
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
                                    .disabled(self.convert_button_disabled)
                                    .primary()
                                    .label("Convert")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let fmt = match this
                                            .end_format_select
                                            .read(cx)
                                            .selected_value()
                                        {
                                            Some(&"PNG") => image::ImageFormat::Png,

                                            Some(&"JPEG") => image::ImageFormat::Jpeg,

                                            Some(&"GIF") => image::ImageFormat::Gif,

                                            Some(&"WEBP") => image::ImageFormat::WebP,

                                            Some(&"TIFF") => image::ImageFormat::Tiff,

                                            Some(&"AVIF") => image::ImageFormat::Avif,
                                            _ => return,
                                        };
                                        cx.spawn(async move |this, cx| {
                                            let _ = this.update(cx, |this, cx| {
                                                this.convert_button_disabled = true;
                                                cx.notify();
                                            });

                                            let a = this.read_with(cx, |app, _cx| {
                                                app.converter.images.clone()
                                            });
                                            if a.is_err() {
                                                return;
                                            }
                                            let a = a.unwrap();

                                            for (i, (image, _)) in a.into_iter().enumerate() {
                                                let _ = this.update(cx, |this, cx| {
                                                    this.converter.images.get_mut(i).and_then(
                                                        |im: &mut (_, ConversionState)| {
                                                            im.1 = ConversionState::Processing;
                                                            Some(im)
                                                        },
                                                    );
                                                    cx.notify();
                                                });
                                                let res = cx
                                                    .background_spawn(async move {
                                                        image.convert(&fmt, None)
                                                    })
                                                    .await;
                                                let _ = this.update(cx, |this, _cx| {
                                                    this.converter.images.get_mut(i).and_then(
                                                        |im| {
                                                            im.1 = match res {
                                                                Ok(_) => ConversionState::Success,
                                                                Err(_) => ConversionState::Fail,
                                                            };
                                                            Some(im)
                                                        },
                                                    );
                                                });
                                            }
                                            let _ = this.update(cx, |this, cx| {
                                                this.convert_button_disabled = false;
                                                cx.notify();
                                            });
                                        })
                                        .detach();
                                    })),
                            ),
                    ),
            )
            .child(div().size_full().overflow_y_hidden().pipe(|d| {
                let n_images = self.converter.images.len();
                if n_images > 0 {
                    d.child(
                        div()
                            .mb_4()
                            .v_flex()
                            .overflow_y_scrollbar()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded_md()
                            .children(self.converter.images.iter().enumerate().map(
                                |(i, (image, _conversion_state))| {
                                    let key = image
                                        .path
                                        .clone()
                                        .into_os_string()
                                        .into_string()
                                        .unwrap_or(String::new());
                                    let hovering =
                                        window.use_keyed_state(key.clone(), cx, |_, _| false);
                                    let hovering_clone = hovering.clone();

                                    div()
                                        .id(key)
                                        .on_hover(move |hover, _, cx| hovering.write(cx, *hover))
                                        .h_flex()
                                        .p_2()
                                        .child(
                                            Label::new(
                                                image
                                                    .path
                                                    .file_name()
                                                    .and_then(|f| f.to_str())
                                                    .unwrap_or(""),
                                            )
                                            .mr_auto(),
                                        )
                                        .pipe(|d| {
                                            if hovering_clone.read(cx).clone() {
                                                d.child(
                                                    Button::new("close")
                                                        .ghost()
                                                        .icon(IconName::Close)
                                                        .size_5()
                                                        .on_click(cx.listener(
                                                            move |this, _, _, cx| {
                                                                this.converter.images.remove(i);
                                                                cx.notify();
                                                            },
                                                        )),
                                                )
                                            } else {
                                                d
                                            }
                                        })
                                        .pipe(|d| {
                                            if i % 2 == 0 {
                                                d
                                            } else {
                                                if i == n_images - 1 {
                                                    d.bg(cx.theme().muted.alpha(0.25))
                                                        .rounded_b(cx.theme().radius - px(1.))
                                                } else {
                                                    d.bg(cx.theme().muted.alpha(0.25))
                                                }
                                            }
                                        })
                                },
                            )),
                    )
                } else {
                    d
                }
            }))
    }
}
