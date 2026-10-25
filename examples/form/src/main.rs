use quinn::prelude::*;

#[component]
fn Form() -> impl IntoView {
    let (name, set_name) = signal(String::new());

    view! {
        <div class="flex flex-col gap-4 p-4">
            <h1>
                "Hello, "
                {move || if name.get().is_empty() { "there".to_string() } else { name.get() }}
            </h1>
            <input
                placeholder="Your name"
                value=move || name.get()
                on:input=move |value: String| { set_name.set(value); }
            />
            <button on:click=move |_| { set_name.set(String::new()); }>"Clear"</button>
        </div>
    }
}

quinn::run!("Form", 480, 320, view! { <Form /> });
