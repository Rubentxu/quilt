use crate::bridge;
use crate::components::block::Block;
use crate::components::loading::Loading;
use crate::outliner::history::OutlinerCommand;
use crate::outliner::page::PageOutliner;
use crate::outliner::tree::apply_structural_mutation;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

#[component]
pub fn PageView() -> impl IntoView {
    let params = use_params_map();
    let page_name = move || {
        params
            .get()
            .get("name")
            .map(|s| s.to_string())
            .unwrap_or_default()
    };
    let (blocks, set_blocks) = signal(Vec::<crate::bridge::BlockDto>::new());
    let (loading, set_loading) = signal(true);

    // Create the PageOutliner coordinator with both a content-applier callback
    // and a structural-applier callback. Both update the blocks signal.
    // This makes undo/redo work for both content and structural operations.
    let page_outliner = {
        let set_blocks_a = set_blocks.clone();
        let set_blocks_b = set_blocks.clone();
        let apply = move |block_id: &str, content: &str| {
            let id = block_id.to_string();
            let c = content.to_string();
            set_blocks_a.update(|blocks_mut| {
                if let Some(idx) = blocks_mut.iter().position(|b| b.id == id) {
                    blocks_mut[idx].content = c;
                }
            });
        };
        let structural_apply = move |cmd: &OutlinerCommand| {
            set_blocks_b.update(|blocks_mut| {
                apply_structural_mutation(blocks_mut, cmd);
            });
        };
        PageOutliner::new_with_structural(100, apply, structural_apply)
    };
    provide_context(page_outliner);

    Effect::new(move || {
        let name = page_name();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            match bridge::get_page_blocks(&name).await {
                Ok(b) => set_blocks.set(b),
                Err(_) => set_blocks.set(vec![]),
            }
            set_loading.set(false);
        });
    });

    view! {
        <div class="page-view">
            <h1 class="text-2xl font-bold mb-6">
                {move || page_name()}
            </h1>

            <Show when=move || loading.get()>
                <Loading />
            </Show>

            <Show
                when=move || !loading.get() && !blocks.get().is_empty()
                fallback=move || view! {
                    <Show when=move || !loading.get()>
                        <div class="text-text-muted text-sm py-4">
                            "This page is empty. Start writing..."
                        </div>
                    </Show>
                }
            >
                <div class="outliner">
                    <For each=move || blocks.get() key=|b| b.id.clone() let:block>
                        <Block block=Signal::derive(move || block.clone()) blocks=blocks set_blocks=set_blocks children=vec![] />
                    </For>
                </div>
            </Show>
        </div>
    }
}
