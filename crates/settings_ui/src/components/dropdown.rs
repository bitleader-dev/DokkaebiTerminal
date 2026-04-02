use std::rc::Rc;

use gpui::{App, ElementId, IntoElement, RenderOnce, SharedString};
use ui::{
    ButtonSize, ContextMenu, DropdownMenu, DropdownStyle, FluentBuilder as _, IconPosition, px,
};

#[derive(IntoElement)]
pub struct EnumVariantDropdown<T>
where
    T: strum::VariantArray + strum::VariantNames + Copy + PartialEq + Send + Sync + 'static,
{
    id: ElementId,
    current_value: T,
    variants: &'static [T],
    /// i18n 번역된 라벨 목록 (variants와 동일 순서)
    labels: Vec<SharedString>,
    should_do_title_case: bool,
    tab_index: Option<isize>,
    on_change: Rc<dyn Fn(T, &mut ui::Window, &mut App) + 'static>,
}

impl<T> EnumVariantDropdown<T>
where
    T: strum::VariantArray + strum::VariantNames + Copy + PartialEq + Send + Sync + 'static,
{
    pub fn new(
        id: impl Into<ElementId>,
        current_value: T,
        variants: &'static [T],
        labels: Vec<SharedString>,
        on_change: impl Fn(T, &mut ui::Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            current_value,
            variants,
            labels,
            should_do_title_case: true,
            tab_index: None,
            on_change: Rc::new(on_change),
        }
    }

    pub fn title_case(mut self, title_case: bool) -> Self {
        self.should_do_title_case = title_case;
        self
    }

    pub fn tab_index(mut self, tab_index: isize) -> Self {
        self.tab_index = Some(tab_index);
        self
    }
}

impl<T> RenderOnce for EnumVariantDropdown<T>
where
    T: strum::VariantArray + strum::VariantNames + Copy + PartialEq + Send + Sync + 'static,
{
    fn render(self, window: &mut ui::Window, cx: &mut ui::App) -> impl gpui::IntoElement {
        let current_idx = self
            .variants
            .iter()
            .position(|v| *v == self.current_value)
            .unwrap();
        let current_value_label = self.labels[current_idx].clone();

        let labels = self.labels.clone();
        let context_menu =
            window.use_keyed_state(current_value_label.clone(), cx, |window, cx| {
                let labels = labels.clone();
                ContextMenu::new(window, cx, move |mut menu, _, _| {
                    for (i, &value) in self.variants.iter().enumerate() {
                        let label = labels[i].to_string();
                        let on_change = self.on_change.clone();
                        let current_value = self.current_value;
                        menu = menu.toggleable_entry(
                            label,
                            value == current_value,
                            IconPosition::End,
                            None,
                            move |window, cx| {
                                on_change(value, window, cx);
                            },
                        );
                    }
                    menu
                })
            });

        DropdownMenu::new(self.id, current_value_label.to_string(), context_menu)
            .when_some(self.tab_index, |elem, tab_index| elem.tab_index(tab_index))
            .trigger_size(ButtonSize::Medium)
            .style(DropdownStyle::Outlined)
            .offset(gpui::Point {
                x: px(0.0),
                y: px(2.0),
            })
            .into_any_element()
    }
}
