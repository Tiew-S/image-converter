use gpui::*;
use gpui_component::*;

use crate::convert::ConvertView;
mod convert;
mod things;

pub struct App {
    convert_view_entity: Entity<convert::ConvertView>,
}

impl App {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            convert_view_entity: cx.new(|cx| ConvertView::new(window, cx)),
        }
    }
}

impl Render for App {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .v_flex()
            .child(TitleBar::new().child("Image Converter").text_sm())
            .child(self.convert_view_entity.clone())
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
                        origin: gpui::Point::new(px(200.), px(100.))
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
