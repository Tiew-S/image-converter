use gpui::*;
use gpui_component::{
    form::{field, v_form}, input::{Input, InputState}, label::Label, switch::Switch, *,
};

use crate::things::Settings;

pub struct SettingsView {
    settings: Settings,
    svg_dpi_input: Entity<InputState>,
    pdf_dpi_input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

impl SettingsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let settings: Settings = confy::load("ImageConverter", None).unwrap_or_default();
        let svg_dpi_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(settings.image_convertion_options.svg_render_dpi.to_string())
                .pattern(regex::Regex::new(r"^\d+$").unwrap())
        });
        let pdf_dpi_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(settings.image_convertion_options.pdf_render_dpi.to_string())
                .pattern(regex::Regex::new(r"^\d+$").unwrap())
        });
        let _subscriptions = vec![
            cx.subscribe_in(&svg_dpi_input, window, {
                let input_state = svg_dpi_input.clone();
                move |this: &mut Self, _, ev: &gpui_component::input::InputEvent, _window, cx| {
                    match ev {
                        gpui_component::input::InputEvent::Change => {
                            let value = input_state.read(cx).value();
                            if let Ok(value) = value.parse() {
                                this.settings.image_convertion_options.svg_render_dpi = value;
                                let _ = this.save_settings();
                            }
                        }
                        _ => {}
                    }
                }
            }),
            cx.subscribe_in(&pdf_dpi_input, window, {
                let input_state = pdf_dpi_input.clone();
                move |this: &mut Self, _, ev: &gpui_component::input::InputEvent, _window, cx| {
                    match ev {
                        gpui_component::input::InputEvent::Change => {
                            let value = input_state.read(cx).value();
                            if let Ok(value) = value.parse() {
                                this.settings.image_convertion_options.pdf_render_dpi = value;
                                let _ = this.save_settings();
                            }
                        }
                        _ => {}
                    }
                }
            }),
        ];
        Self {
            settings: settings.clone(),
            svg_dpi_input,
            pdf_dpi_input,
            _subscriptions,
        }
    }

    fn save_settings(&self) -> Result<()> {
        confy::store("ImageConverter", None, &self.settings)?;
        Ok(())
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_4()
            .gap_4()
            .v_flex()
            .child(Label::new("Settings").font_semibold().text_lg())
            .child(
                v_form()
                    .p_2()
                    .border_1()
                    .rounded_md()
                    .border_color(cx.theme().border)
                    .gap_2()
                    .child(
                        field().label("SVG DPI").child(Input::new(&self.svg_dpi_input))
                    )
                    .child(
                        field().label("PDF DPI").child(Input::new(&self.pdf_dpi_input))
                    )
                    .child(
                        field().label("Separate PDF pages").child(Switch::new("pdf-sep-pages").checked(self.settings.image_convertion_options.pdf_render_to_separate_pages).on_click(cx.listener(|this, checked, _, cx| {
                            this.settings.image_convertion_options.pdf_render_to_separate_pages = *checked;
                            let _ = this.save_settings();
                            cx.notify();
                        })))
                    )
            )
    }
}
