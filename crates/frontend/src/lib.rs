use bonsai_types::{CfsResponse, DiffResponse, LeafResponse, NodeResponse, ProofResponse, RootResponse, TrieKind};
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::prelude::IntoAny;
use leptos::task::spawn_local;
use leptos::prelude::Callback;
use wasm_bindgen::prelude::wasm_bindgen;

const API_BASE: &str = "http://127.0.0.1:4010";

#[component]
pub fn App() -> impl IntoView {
    let (active_tab, set_active_tab) = signal(Tab::Tree);
    let (db_path, set_db_path) = signal(String::new());
    let (db_status, set_db_status) = signal(String::new());

    let (trie_kind, set_trie_kind) = signal(TrieKind::Contract);
    let (identifier, set_identifier) = signal(String::new());
    let (key_input, set_key_input) = signal(String::new());

    let (root, set_root) = signal::<Option<RootResponse>>(None);
    let (nodes, set_nodes) = signal(std::collections::HashMap::<String, NodeResponse>::new());
    let (loading_paths, set_loading_paths) = signal(std::collections::HashSet::<String>::new());
    let (search_input, set_search_input) = signal(String::new());
    let (search_target, set_search_target) = signal::<Option<String>>(None);

    let (diff_block, set_diff_block) = signal(String::new());
    let (diff_resp, set_diff_resp) = signal::<Option<DiffResponse>>(None);

    let (proof_resp, set_proof_resp) = signal::<Option<ProofResponse>>(None);
    let (leaf_resp, set_leaf_resp) = signal::<Option<LeafResponse>>(None);
    let (trace_bits, set_trace_bits) = signal::<Option<String>>(None);

    let (cfs_resp, set_cfs_resp) = signal::<Option<CfsResponse>>(None);

    let open_db = move || {
        let path = db_path.get();
        if path.is_empty() {
            set_db_status.set("DB path missing".to_string());
            return;
        }
        spawn_local(async move {
            let path = if path.ends_with("/db") { path } else { format!("{path}/db") };
            let url = format!("{API_BASE}/api/open?db_path={}", urlencoding::encode(&path));
            let _ = Request::post(&url).send().await;
            set_db_status.set(format!("DB open requested: {path}"));
        });
    };

    let on_node = Callback::new(move |path_hex: String| {
        let trie = trie_kind.get();
        let ident = identifier.get();
        set_loading_paths.update(|set| {
            set.insert(path_hex.clone());
        });
        spawn_local(async move {
            let mut url = format!("{API_BASE}/api/trie/node?trie={}&path={}", format_trie(trie), urlencoding::encode(&path_hex));
            if trie == TrieKind::Storage && !ident.is_empty() {
                url.push_str(&format!("&identifier={}", urlencoding::encode(&ident)));
            }
            let Ok(resp) = Request::get(&url).send().await else { return; };
            let Ok(data) = resp.json::<NodeResponse>().await else { return; };
            let path_key = path_hex.clone();
            set_nodes.update(|map| {
                map.insert(path_key, data);
            });
            set_loading_paths.update(|set| {
                set.remove(&path_hex);
            });
        });
    });

    let fetch_root = {
        let on_node = on_node.clone();
        move || {
            let trie = trie_kind.get();
            let ident = identifier.get();
            spawn_local(async move {
                let mut url = format!("{API_BASE}/api/trie/root?trie={}", format_trie(trie));
                if trie == TrieKind::Storage && !ident.is_empty() {
                    url.push_str(&format!("&identifier={}", urlencoding::encode(&ident)));
                }
                let Ok(resp) = Request::get(&url).send().await else { return; };
                let Ok(data) = resp.json::<RootResponse>().await else { return; };
                if let Some(node) = data.node.clone() {
                    for (child, _) in child_paths(&data.path_hex, &node) {
                        on_node.run(child);
                    }
                }
                set_root.set(Some(data));
            });
        }
    };

    let fetch_leaf = move || {
        let trie = trie_kind.get();
        let ident = identifier.get();
        let key = key_input.get();
        if key.is_empty() {
            return;
        }
        spawn_local(async move {
            let mut url = format!("{API_BASE}/api/trie/leaf?trie={}&key={}", format_trie(trie), urlencoding::encode(&key));
            if trie == TrieKind::Storage && !ident.is_empty() {
                url.push_str(&format!("&identifier={}", urlencoding::encode(&ident)));
            }
            let Ok(resp) = Request::get(&url).send().await else { return; };
            let Ok(data) = resp.json::<LeafResponse>().await else { return; };
            set_leaf_resp.set(Some(data));
        });
    };

    let fetch_diff = move || {
        let trie = trie_kind.get();
        let block = diff_block.get();
        if block.is_empty() {
            return;
        }
        spawn_local(async move {
            let url = format!("{API_BASE}/api/diff?trie={}&block={}", format_trie(trie), block);
            let Ok(resp) = Request::get(&url).send().await else { return; };
            let Ok(data) = resp.json::<DiffResponse>().await else { return; };
            set_diff_resp.set(Some(data));
        });
    };

    let fetch_proof = move || {
        let trie = trie_kind.get();
        let ident = identifier.get();
        let key = key_input.get();
        if key.is_empty() {
            return;
        }
        spawn_local(async move {
            let mut url = format!("{API_BASE}/api/proof?trie={}&key={}", format_trie(trie), urlencoding::encode(&key));
            if trie == TrieKind::Storage && !ident.is_empty() {
                url.push_str(&format!("&identifier={}", urlencoding::encode(&ident)));
            }
            let Ok(resp) = Request::get(&url).send().await else { return; };
            let Ok(data) = resp.json::<ProofResponse>().await else { return; };
            set_proof_resp.set(Some(data));
        });
    };

    let fetch_trace = {
        let fetch_leaf = fetch_leaf;
        let fetch_proof = fetch_proof;
        move || {
            let key = key_input.get();
            if key.is_empty() {
                return;
            }
            let bits = felt_hex_to_bits(&key);
            set_trace_bits.set(Some(format_bits_preview(&bits)));
            fetch_leaf();
            fetch_proof();
        }
    };

    let on_search = {
        let on_node = on_node.clone();
        move || {
            let query = search_input.get();
            if query.is_empty() {
                return;
            }
            let nodes_map = nodes.get();
            let target = if let Some(found) = nodes_map.iter().find_map(|(path, node)| {
                node.node
                    .as_ref()
                    .and_then(|n| n.hash.as_ref())
                    .filter(|h| h.eq_ignore_ascii_case(&query))
                    .map(|_| path.clone())
            }) {
                found
            } else {
                query.clone()
            };
            set_search_target.set(Some(target.clone()));
            on_node.run(target);
        }
    };

    let fetch_cfs = move || {
        spawn_local(async move {
            let Ok(resp) = Request::get(&format!("{API_BASE}/api/cfs")).send().await else { return; };
            let Ok(data) = resp.json::<CfsResponse>().await else { return; };
            set_cfs_resp.set(Some(data));
        });
    };

    view! {
        <div class="app">
            <aside class="sidebar">
                <h1>"Bonsai Trie Visualizer"</h1>
                <nav>
                    <button class=tab_class(active_tab, Tab::Tree) on:click=move |_| set_active_tab.set(Tab::Tree)>"Tree"</button>
                    <button class=tab_class(active_tab, Tab::Path) on:click=move |_| set_active_tab.set(Tab::Path)>"Path Trace"</button>
                    <button class=tab_class(active_tab, Tab::Diff) on:click=move |_| set_active_tab.set(Tab::Diff)>"Diff"</button>
                    <button class=tab_class(active_tab, Tab::Proof) on:click=move |_| set_active_tab.set(Tab::Proof)>"Proof"</button>
                    <button class=tab_class(active_tab, Tab::Stats) on:click=move |_| set_active_tab.set(Tab::Stats)>"Stats"</button>
                </nav>
                <div class="panel">
                    <label>"DB Path"</label>
                    <input type="text" value=db_path on:input=move |ev| set_db_path.set(event_target_value(&ev)) />
                    <button on:click=move |_| open_db()>"Open"</button>
                    <p class="muted">{move || db_status.get()}</p>
                </div>
                <div class="panel">
                    <label>"Trie"</label>
                    <select on:change=move |ev| {
                        let v = event_target_value(&ev);
                        let kind = match v.as_str() { "contract" => TrieKind::Contract, "storage" => TrieKind::Storage, _ => TrieKind::Class };
                        set_trie_kind.set(kind);
                    }>
                        <option value="contract">"Contract"</option>
                        <option value="storage">"Storage"</option>
                        <option value="class">"Class"</option>
                    </select>
                    <label>"Storage Identifier"</label>
                    <input type="text" value=identifier on:input=move |ev| set_identifier.set(event_target_value(&ev)) />
                </div>
                <div class="panel">
                    <label>"Key"</label>
                    <input type="text" value=key_input on:input=move |ev| set_key_input.set(event_target_value(&ev)) />
                    <div class="row">
                        <button on:click=move |_| fetch_leaf()>"Leaf"</button>
                        <button on:click=move |_| fetch_proof()>"Proof"</button>
                        <button on:click=move |_| fetch_trace()>"Trace"</button>
                    </div>
                </div>
                <div class="panel">
                    <label>"Search (Path/Hash)"</label>
                    <input type="text" value=search_input on:input=move |ev| set_search_input.set(event_target_value(&ev)) />
                    <button on:click=move |_| on_search()>"Go"</button>
                </div>
            </aside>

            <main class="content">
                <Show when=move || active_tab.get() == Tab::Tree fallback=|| ()>
                    <TreeView root=root nodes=nodes loading=loading_paths search=search_target on_root=fetch_root on_node=on_node />
                </Show>
                <Show when=move || active_tab.get() == Tab::Path fallback=|| ()>
                    <PathView leaf=leaf_resp proof=proof_resp trace=trace_bits />
                </Show>
                <Show when=move || active_tab.get() == Tab::Diff fallback=|| ()>
                    <DiffView block=diff_block diff=diff_resp on_block=set_diff_block on_fetch=fetch_diff />
                </Show>
                <Show when=move || active_tab.get() == Tab::Proof fallback=|| ()>
                    <ProofView proof=proof_resp />
                </Show>
                <Show when=move || active_tab.get() == Tab::Stats fallback=|| ()>
                    <StatsView cfs=cfs_resp on_fetch=fetch_cfs />
                </Show>
            </main>
        </div>
    }
}

#[component]
fn TreeView(
    root: ReadSignal<Option<RootResponse>>,
    nodes: ReadSignal<std::collections::HashMap<String, NodeResponse>>,
    loading: ReadSignal<std::collections::HashSet<String>>,
    search: ReadSignal<Option<String>>,
    on_root: impl Fn() + 'static + Copy,
    on_node: Callback<String>,
) -> impl IntoView {
    view! {
        <section>
            <div class="header-row">
                <h2>"Trie Tree"</h2>
                <button on:click=move |_| on_root()>"Load Root"</button>
            </div>
            <Show when=move || root.get().is_some() fallback=|| view! { <p class="muted">"No root loaded."</p> }>
                {move || {
                    let root = root.get().unwrap();
                    view! { <GraphView root=root nodes=nodes loading=loading search=search on_node=on_node /> }
                }}
            </Show>
        </section>
    }
}

#[component]
fn GraphView(
    root: RootResponse,
    nodes: ReadSignal<std::collections::HashMap<String, NodeResponse>>,
    loading: ReadSignal<std::collections::HashSet<String>>,
    search: ReadSignal<Option<String>>,
    on_node: Callback<String>,
) -> impl IntoView {
    let root_path = root.path_hex.clone();
    let root_node = root.node.clone();
    let root_for_action = StoredValue::new(root_path.clone());
    let root_for_graph = root.clone();
    let (selected, set_selected) = signal::<Option<String>>(None);
    let (hovered, set_hovered) = signal::<Option<String>>(None);
    let (scale, set_scale) = signal(1.0_f32);
    let (offset, set_offset) = signal((0.0_f32, 0.0_f32));
    let (dragging, set_dragging) = signal(false);
    let (last_pos, set_last_pos) = signal((0.0_f32, 0.0_f32));
    let graph = Memo::new(move |_| {
        let nodes_map = nodes.get();
        let node_count = nodes_map.len();
        let data = build_graph(&root_for_graph, &nodes_map);
        (node_count, data)
    });
    Effect::new(move |_| {
        if let Some(path) = search.get() {
            set_selected.set(Some(path));
        }
    });

    view! {
        <div class="graph">
            {move || {
                let (node_count, data) = graph.get();
                let (width, height) = graph_bounds(&data);
                let (x_gap, y_gap, padding) = graph_metrics();
                let pad = padding * 0.5;
                view! {
                    <>
                        <div class="graph-header">
                            <div class="graph-help">
                                <p class="muted">"Loaded nodes: " {node_count + 1} ". Click a node to load its children."</p>
                            </div>
                            <button class="graph-action" on:click=move |_| on_node.run(root_for_action.get_value())>"Load Root Node"</button>
                        </div>
                        <div class="graph-stage">
                            <div class="graph-detail">
                                {render_selection(selected, hovered, nodes, root_path.clone(), root_node.clone(), loading)}
                            </div>
                        <svg
                            class="graph-svg"
                            viewBox=format!("0 0 {} {}", width, height)
                            width="100%"
                            height="100%"
                            on:wheel=move |ev| {
                                ev.prevent_default();
                                let delta = ev.delta_y() as f32;
                                let next = (scale.get() - delta * 0.001).clamp(0.3, 2.5);
                                set_scale.set(next);
                            }
                            on:pointerdown=move |ev| {
                                set_dragging.set(true);
                                set_last_pos.set((ev.client_x() as f32, ev.client_y() as f32));
                            }
                            on:pointerup=move |_| set_dragging.set(false)
                            on:pointerleave=move |_| set_dragging.set(false)
                            on:pointermove=move |ev| {
                                if dragging.get() {
                                    let (lx, ly) = last_pos.get();
                                    let (ox, oy) = offset.get();
                                    let nx = ev.client_x() as f32;
                                    let ny = ev.client_y() as f32;
                                    set_offset.set((ox + (nx - lx), oy + (ny - ly)));
                                    set_last_pos.set((nx, ny));
                                }
                            }
                        >
                            <g transform=move || {
                                let (ox, oy) = offset.get();
                                format!("translate({} {}) scale({})", ox, oy, scale.get())
                            }>
                            {data.edges.into_iter().map(|edge| {
                                let x1 = edge.from_x * x_gap + pad;
                                let y1 = edge.from_y * y_gap + pad;
                                let x2 = edge.to_x * x_gap + pad;
                                let y2 = edge.to_y * y_gap + pad;
                                let label_x = (x1 + x2) * 0.5;
                                let label_y = (y1 + y2) * 0.5;
                                view! {
                                    <>
                                        <line
                                            class="graph-edge"
                                            x1=x1
                                            y1=y1
                                            x2=x2
                                            y2=y2
                                        />
                                        <text class="graph-edge-label" x=label_x y=label_y>{edge.label}</text>
                                    </>
                                }
                            }).collect_view()}
                            {data.nodes.into_iter().map(|node| {
                                let on_node = on_node.clone();
                                let set_selected = set_selected;
                                let set_hovered = set_hovered;
                                let node_path = node.path.clone();
                                let node_path_click = node_path.clone();
                                let node_path_hover_in = node_path.clone();
                                let node_data = node.node.clone();
                                let kind = node.kind.clone();
                                let x = node.x * x_gap + pad;
                                let y = node.y * y_gap + pad;
                                let is_selected = selected.get().as_ref().map(|p| p == &node_path).unwrap_or(false);
                                let is_loading = loading.get().contains(&node_path);
                                view! {
                                    <g class="graph-node" on:click=move |_| {
                                        set_selected.set(Some(node_path_click.clone()));
                                        on_node.run(node_path_click.clone());
                                        if let Some(node_data) = node_data.clone() {
                                            for (child, _) in child_paths(&node_path_click, &node_data) {
                                                on_node.run(child);
                                            }
                                        }
                                    }>
                                        <circle
                                            class=format!(
                                                "graph-dot {} {} {}",
                                                kind,
                                                if is_selected { "selected" } else { "" },
                                                if is_loading { "loading" } else { "" }
                                            )
                                            cx=x
                                            cy=y
                                            r="18"
                                            on:mouseenter=move |_| set_hovered.set(Some(node_path_hover_in.clone()))
                                            on:mouseleave=move |_| set_hovered.set(None)
                                        />
                                        <text class="graph-label" x=x y=y>{short_kind(&kind)}</text>
                                    </g>
                                }
                            }).collect_view()}
                            </g>
                        </svg>
                        </div>
                    </>
                }
            }}
        </div>
    }
}

#[derive(Clone, PartialEq)]
struct GraphNode {
    path: String,
    kind: String,
    node: Option<bonsai_types::NodeView>,
    x: f32,
    y: f32,
}

#[derive(Clone, PartialEq)]
struct GraphEdge {
    from_x: f32,
    from_y: f32,
    to_x: f32,
    to_y: f32,
    label: String,
}

#[derive(Clone, PartialEq)]
struct GraphData {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

fn build_graph(root: &RootResponse, nodes: &std::collections::HashMap<String, NodeResponse>) -> GraphData {
    let mut data = GraphData { nodes: Vec::new(), edges: Vec::new() };
    let mut next_x = 0.0_f32;
    let root_path = root.path_hex.clone();
    let root_node = root
        .node
        .clone()
        .or_else(|| nodes.get(&root_path).and_then(|n| n.node.clone()));
    walk_graph(&root_path, root_node, nodes, 0, &mut next_x, &mut data);
    data
}

fn walk_graph(
    path: &str,
    node: Option<bonsai_types::NodeView>,
    nodes: &std::collections::HashMap<String, NodeResponse>,
    depth: usize,
    next_x: &mut f32,
    data: &mut GraphData,
) -> f32 {
    let mut child_centers: Vec<(f32, String)> = Vec::new();
    if let Some(node_data) = node.clone() {
        for (child_path, label) in child_paths(path, &node_data) {
            if let Some(child_node) = nodes.get(&child_path).and_then(|n| n.node.clone()) {
                let child_x = walk_graph(&child_path, Some(child_node), nodes, depth + 1, next_x, data);
                child_centers.push((child_x, label));
            }
        }
    }

    let x = if child_centers.is_empty() {
        let x = *next_x;
        *next_x += 1.0;
        x
    } else {
        let first = child_centers.first().map(|(x, _)| *x).unwrap_or(0.0);
        let last = child_centers.last().map(|(x, _)| *x).unwrap_or(first);
        (first + last) * 0.5
    };

    let y = depth as f32;
    data.nodes.push(GraphNode {
        path: path.to_string(),
        kind: node.as_ref().map(|n| n.kind.clone()).unwrap_or_else(|| "missing".to_string()),
        node,
        x,
        y,
    });

    for (child_x, label) in child_centers {
        data.edges.push(GraphEdge {
            from_x: x,
            from_y: y,
            to_x: child_x,
            to_y: (depth + 1) as f32,
            label,
        });
    }

    x
}

fn child_paths(path: &str, node: &bonsai_types::NodeView) -> Vec<(String, String)> {
    if node.kind == "binary" {
        vec![
            (append_bit_to_path(path, false), "L".to_string()),
            (append_bit_to_path(path, true), "R".to_string()),
        ]
    } else if let Some(edge_hex) = node.path_hex.clone() {
        vec![(concat_paths(path, &edge_hex), "C".to_string())]
    } else {
        Vec::new()
    }
}

fn graph_bounds(data: &GraphData) -> (f32, f32) {
    let (x_gap, y_gap, padding) = graph_metrics();
    let mut max_x = 0.0;
    let mut max_y = 0.0;
    for node in &data.nodes {
        if node.x > max_x {
            max_x = node.x;
        }
        if node.y > max_y {
            max_y = node.y;
        }
    }
    let width = (max_x + 1.0) * x_gap + padding;
    let height = (max_y + 1.0) * y_gap + padding;
    (width, height)
}

fn graph_metrics() -> (f32, f32, f32) {
    (140.0, 120.0, 80.0)
}

fn short_kind(kind: &str) -> String {
    match kind {
        "binary" => "B".to_string(),
        "edge" => "E".to_string(),
        other => other.chars().next().map(|c| c.to_string()).unwrap_or_else(|| "?".to_string()),
    }
}

fn render_selection(
    selected: ReadSignal<Option<String>>,
    hovered: ReadSignal<Option<String>>,
    nodes: ReadSignal<std::collections::HashMap<String, NodeResponse>>,
    root_path: String,
    root_node: Option<bonsai_types::NodeView>,
    loading: ReadSignal<std::collections::HashSet<String>>,
) -> impl IntoView {
    let root_path = StoredValue::new(root_path);
    let root_node = StoredValue::new(root_node);
    view! {
        <div class="detail-card">
            <h3>"Node Details"</h3>
            <Show
                when=move || !loading.get().is_empty()
                fallback=|| ()
            >
                {move || {
                    let count = loading.get().len();
                    view! { <p class="muted">"Loading nodes: " {count}</p> }
                }}
            </Show>
            <Show
                when=move || selected.get().is_some() || hovered.get().is_some()
                fallback=|| view! { <p class="muted">"Select or hover a node to see details."</p> }
            >
                {move || {
                    let path = selected.get().or_else(|| hovered.get()).unwrap_or_default();
                    let node = if path == root_path.get_value() {
                        root_node.get_value()
                    } else {
                        nodes.get().get(&path).and_then(|n| n.node.clone())
                    };
                    match node {
                        Some(node) => view! {
                            <div class="detail-grid">
                                <div>
                                    <span class="label">"Path"</span>
                                    <span>{path}</span>
                                </div>
                                <div>
                                    <span class="label">"Kind"</span>
                                    <span>{node.kind}</span>
                                </div>
                                <div>
                                    <span class="label">"Height"</span>
                                    <span>{node.height}</span>
                                </div>
                                <div>
                                    <span class="label">"Hash"</span>
                                    <span class="mono">{node.hash.unwrap_or_else(|| "none".to_string())}</span>
                                </div>
                            </div>
                        }.into_any(),
                        None => view! { <p class="muted">"Node not loaded."</p> }.into_any(),
                    }
                }}
            </Show>
        </div>
    }
}

#[component]
fn PathView(
    leaf: ReadSignal<Option<LeafResponse>>,
    proof: ReadSignal<Option<ProofResponse>>,
    trace: ReadSignal<Option<String>>,
) -> impl IntoView {
    view! {
        <section>
            <h2>"Path Trace"</h2>
            <Show
                when=move || trace.get().is_some()
                fallback=|| view! { <p class="muted">"Click Trace to compute a key path."</p> }
            >
                {move || {
                    let bits = trace.get().unwrap_or_default();
                    view! {
                        <div class="detail-card">
                            <h3>"Path Bits"</h3>
                            <p class="mono">{bits}</p>
                        </div>
                    }
                }}
            </Show>
            <Show when=move || leaf.get().is_some() fallback=|| ()>
                {move || {
                    let resp = leaf.get().unwrap();
                    view! { <p>"Value: " {resp.value.unwrap_or_else(|| "None".to_string())}</p> }
                }}
            </Show>
            <Show when=move || proof.get().is_some() fallback=|| ()>
                {move || {
                    let resp = proof.get().unwrap();
                    view! {
                        <div class="detail-card">
                            <h3>"Proof Trace"</h3>
                            <p class="muted">{if resp.verified { "Verified" } else { "Not verified" }}</p>
                            <ul class="list">
                                {resp.nodes.into_iter().map(|n| view!{
                                    <li>{n.kind} " path_len=" {n.path_len.unwrap_or_default()}</li>
                                }).collect_view()}
                            </ul>
                        </div>
                    }
                }}
            </Show>
        </section>
    }
}

#[component]
fn DiffView(
    block: ReadSignal<String>,
    diff: ReadSignal<Option<DiffResponse>>,
    on_block: WriteSignal<String>,
    on_fetch: impl Fn() + 'static + Copy,
) -> impl IntoView {
    view! {
        <section>
            <div class="header-row">
                <h2>"Diff"</h2>
                <div class="row">
                    <input type="text" placeholder="Block number" value=block on:input=move |ev| on_block.set(event_target_value(&ev)) />
                    <button on:click=move |_| on_fetch()>"Load"</button>
                </div>
            </div>
            <Show when=move || diff.get().is_some() fallback=|| view! { <p class="muted">"No diff loaded."</p> }>
                {move || {
                    let resp = diff.get().unwrap();
                    view! {
                        <div class="detail-card">
                            <h3>"Changes"</h3>
                            <p class="muted">"Total entries: " {resp.entries.len()}</p>
                            <table class="diff-table">
                                <thead>
                                    <tr>
                                        <th>"Key Type"</th>
                                        <th>"Change"</th>
                                        <th>"Key Len"</th>
                                        <th>"Value"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {resp.entries.into_iter().map(|e| view!{
                                        <tr>
                                            <td>{e.key_type}</td>
                                            <td>{e.change_type}</td>
                                            <td>{e.key_len.unwrap_or_default()}</td>
                                            <td class="mono">{e.value}</td>
                                        </tr>
                                    }).collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }
                }}
            </Show>
        </section>
    }
}

#[component]
fn ProofView(proof: ReadSignal<Option<ProofResponse>>) -> impl IntoView {
    let (show_json, set_show_json) = signal(false);
    view! {
        <section>
            <h2>"Proof"</h2>
            <Show when=move || proof.get().is_some() fallback=|| view! { <div><p class="muted">"No proof loaded."</p></div> }>
                {move || {
                    let resp = proof.get().unwrap();
                    let json = StoredValue::new(serde_json::to_string_pretty(&resp).unwrap_or_default());
                    view! {
                        <div>
                            <div class="header-row">
                                <p class="muted">{if resp.verified { "Verified" } else { "Not verified" }}</p>
                                <button on:click=move |_| set_show_json.set(!show_json.get())>
                                    {if show_json.get() { "Hide JSON" } else { "Show JSON" }}
                                </button>
                            </div>
                            <div class="detail-card">
                                <h3>"Proof Nodes"</h3>
                                <p class="muted">"Total nodes: " {resp.nodes.len()}</p>
                                <ul class="list">
                                    {resp.nodes.into_iter().map(|n| view!{
                                        <li>{n.kind} " path_len=" {n.path_len.unwrap_or_default()}</li>
                                    }).collect_view()}
                                </ul>
                            </div>
                            <Show when=move || show_json.get() fallback=|| ()>
                                {move || view! { <pre class="code-block">{json.get_value()}</pre> }}
                            </Show>
                        </div>
                    }
                }}
            </Show>
        </section>
    }
}

#[component]
fn StatsView(cfs: ReadSignal<Option<CfsResponse>>, on_fetch: impl Fn() + 'static + Copy) -> impl IntoView {
    view! {
        <section>
            <div class="header-row">
                <h2>"Stats"</h2>
                <button on:click=move |_| on_fetch()>"Load CFs"</button>
            </div>
            <Show when=move || cfs.get().is_some() fallback=|| view! { <div><p class="muted">"No CFs loaded."</p></div> }>
                {move || {
                    let resp = cfs.get().unwrap();
                    view! {
                        <div>
                            <ul class="list">
                                {resp.names.into_iter().map(|name| view!{ <li>{name}</li> }).collect_view()}
                            </ul>
                        </div>
                    }
                }}
            </Show>
        </section>
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Tree,
    Path,
    Diff,
    Proof,
    Stats,
}

fn tab_class(active: ReadSignal<Tab>, tab: Tab) -> String {
    if active.get() == tab {
        "nav-btn active".to_string()
    } else {
        "nav-btn".to_string()
    }
}

fn format_trie(kind: TrieKind) -> &'static str {
    match kind {
        TrieKind::Contract => "contract",
        TrieKind::Storage => "storage",
        TrieKind::Class => "class",
    }
}

fn felt_hex_to_bits(hex: &str) -> Vec<bool> {
    let mut bytes = hex_to_bytes(hex).unwrap_or_default();
    if bytes.len() < 32 {
        let mut padded = vec![0u8; 32 - bytes.len()];
        padded.append(&mut bytes);
        bytes = padded;
    } else if bytes.len() > 32 {
        bytes = bytes[bytes.len() - 32..].to_vec();
    }
    let mut bits = Vec::with_capacity(256);
    for b in bytes {
        for i in 0..8 {
            bits.push((b >> (7 - i)) & 1 == 1);
        }
    }
    if bits.len() > 5 {
        bits.split_off(5)
    } else {
        Vec::new()
    }
}

fn format_bits_preview(bits: &[bool]) -> String {
    let len = bits.len();
    let to_str = |slice: &[bool]| slice.iter().map(|b| if *b { '1' } else { '0' }).collect::<String>();
    if len <= 128 {
        format!("{} (len={})", to_str(bits), len)
    } else {
        let start = to_str(&bits[..64]);
        let end = to_str(&bits[len - 64..]);
        format!("{}...{} (len={})", start, end, len)
    }
}

fn decode_path_bits(encoded: &[u8]) -> Vec<bool> {
    if encoded.is_empty() {
        return Vec::new();
    }
    let len = encoded[0] as usize;
    let mut bits = Vec::with_capacity(len);
    let mut remaining = len;
    for byte in encoded.iter().skip(1) {
        for i in 0..8 {
            if remaining == 0 {
                break;
            }
            let bit = (byte >> (7 - i)) & 1 == 1;
            bits.push(bit);
            remaining -= 1;
        }
        if remaining == 0 {
            break;
        }
    }
    bits
}

fn encode_path_bits(bits: &[bool]) -> Vec<u8> {
    let len = bits.len();
    let mut out = Vec::with_capacity(1 + (len + 7) / 8);
    out.push(len as u8);
    let mut current = 0u8;
    for (i, bit) in bits.iter().enumerate() {
        let idx = i % 8;
        if *bit {
            current |= 1 << (7 - idx);
        }
        if idx == 7 {
            out.push(current);
            current = 0;
        }
    }
    if len % 8 != 0 {
        out.push(current);
    }
    out
}

fn append_bit_to_path(path_hex: &str, bit: bool) -> String {
    let bytes = hex_to_bytes(path_hex).unwrap_or_else(|| vec![0u8]);
    let mut bits = decode_path_bits(&bytes);
    bits.push(bit);
    bytes_to_hex(&encode_path_bits(&bits))
}

fn concat_paths(left_hex: &str, right_hex: &str) -> String {
    let left = hex_to_bytes(left_hex).unwrap_or_else(|| vec![0u8]);
    let right = hex_to_bytes(right_hex).unwrap_or_else(|| vec![0u8]);
    let mut bits = decode_path_bits(&left);
    bits.extend(decode_path_bits(&right));
    bytes_to_hex(&encode_path_bits(&bits))
}

fn hex_to_bytes(hex: &str) -> Option<Vec<u8>> {
    let s = hex.trim().strip_prefix("0x").unwrap_or(hex.trim());
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16).ok()?;
        out.push(byte);
    }
    Some(out)
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2 + 2);
    s.push_str("0x");
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[wasm_bindgen(start)]
pub fn main() {
    leptos::mount::mount_to_body(App);
}
