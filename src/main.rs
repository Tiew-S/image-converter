//#![windows_subsystem = "windows"]

use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    button::{Button, ButtonVariants},
    label::Label,
    separator::Separator,
    *,
};

use crate::{convert::ConvertView, settings::SettingsView};
mod convert;
mod settings;
mod things;

pub struct App {
    settings_open: bool,
    settings_view_entity: Entity<SettingsView>,
    convert_view_entity: Entity<convert::ConvertView>,
}

impl App {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            settings_open: false,
            convert_view_entity: cx.new(|cx| ConvertView::new(window, cx)),
            settings_view_entity: cx.new(|cx| SettingsView::new(window, cx)),
        }
    }
}

impl Render for App {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dialog_layer = Root::render_dialog_layer(window, cx);
        div()
            .size_full()
            .v_flex()
            .child(
                TitleBar::new()
                    .child(Label::new("Image Converter").text_sm())
                    .child(
                        div()
                            .h_flex()
                            .child(
                                Button::new("open-settings")
                                    .icon(Icon::new(IconName::Settings))
                                    .corner_radii(Corners::all(px(0.)))
                                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                        window.prevent_default();
                                        cx.stop_propagation();
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.settings_open = !this.settings_open;
                                        cx.notify();
                                    }))
                                    .ghost()
                                    .when(self.settings_open, |d| d.bg(cx.theme().muted)),
                            )
                            .child(Separator::vertical()),
                    ),
            )
            .child(if self.settings_open {
                div()
                    .size_full()
                    .v_flex()
                    .child(
                        Button::new("go-back")
                            .icon(IconName::ArrowLeft)
                            .label("Back")
                            .ghost()
                            .m_1()
                            .mr_auto()
                            .xsmall()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.settings_open = false;
                                cx.notify()
                            })),
                    )
                    .child(self.settings_view_entity.clone())
            } else {
                div().size_full().child(self.convert_view_entity.clone())
            })
            .children(dialog_layer)
    }
}

fn main() {
    let app = gpui_platform::application().with_assets(gpui_component_assets::Assets);

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);
        let theme = Theme::global_mut(cx);

        theme.apply_config(&{
            if dark_light::detect().unwrap() == dark_light::Mode::Dark {
                theme.dark_theme.clone()
            } else {
                theme.light_theme.clone()
            }
        });
        cx.spawn(async move |cx| {
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        size: gpui::Size::new(px(400.), px(600.)),
                        origin: gpui::Point::new(px(200.), px(100.)),
                    })),
                    titlebar: Some(TitleBar::title_bar_options()),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| App::new(window, cx));
                    // This first level on the window, should be a Root.
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        })
        .detach();
    });
}
