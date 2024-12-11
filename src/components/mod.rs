use std::str::FromStr;

use leptos::{
    component,
    prelude::{
        event_target_value, ClassAttribute, OnAttribute, PropAttribute, ReadSignal, RwSignal, Set,
    },
    tachys::html::property::IntoProperty,
    view, IntoView,
};

pub mod script_input;
pub mod token_data;
pub mod tracker;
pub mod tx_input;
pub mod tx_output;

#[component]
pub fn ParsedInput<T: FromStr + Clone + Send + Sync + 'static>(value: RwSignal<T>) -> impl IntoView
where
    ReadSignal<T>: IntoProperty,
{
    let parse_success = RwSignal::new(true);
    let (thevalue, set_value) = value.split();

    view! {
        <input
            on:input=move |e| {
                let new_value = event_target_value(&e);
                match new_value.parse() {
                    Ok(v) => {
                        set_value(v);
                        parse_success.set(true);
                    }
                    Err(_) => {
                        parse_success.set(false);
                    }
                }
            }
            prop:value=thevalue
            class="border border-solid rounded px-1 bg-stone-900 placeholder:text-stone-600"
            class=("border-stone-600", parse_success)
            class=("border-red-700", move || !parse_success())
        />
    }
}
