use std::str::FromStr;

use leptos::{
    component, create_signal, event_target_value, view, IntoAttribute, IntoProperty, IntoView, MaybeProp, ReadSignal, RwSignal
};

pub mod token_data;
pub mod tracker;
pub mod tx_output;

#[component]
pub fn ParsedInput<T: FromStr + Clone + 'static, I: IntoAttribute>(
    value: RwSignal<T>,
    #[prop(default = "")] placeholder: &'static str,
    #[prop(default = "")] class: &'static str,
    id: I,
    #[prop(into, default=None::<bool>.into())] disabled: MaybeProp<bool>,
) -> impl IntoView
where
    ReadSignal<T>: IntoProperty,
{
    let (parse_success, set_parse_success) = create_signal(true);
    let (thevalue, set_value) = value.split();

    view! {
        <input
            on:input=move |e| {
                let new_value = event_target_value(&e);
                match new_value.parse() {
                    Ok(v) => {
                        set_value(v);
                        set_parse_success(true);
                    }
                    Err(_) => {
                        set_parse_success(false);
                    }
                }
            }
            prop:value=thevalue
            class={move || format!("border border-solid rounded px-1 bg-stone-900 placeholder:text-stone-600 {}", class)}
            class=("border-stone-600", parse_success)
            class=("border-red-700", move || !parse_success())
            placeholder=placeholder
            disabled=disabled
            id=id
        />
    }
}
