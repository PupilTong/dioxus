use dioxus_core::{
    AttributeValue, ElementId, Template, TemplateAttribute, TemplateNode, VirtualDom,
    WriteMutations,
};
use dioxus_lynx_sys::raw;
use rustc_hash::FxHashMap;

use crate::events;

pub trait Host {
    fn page_root(&mut self) -> i32;
    fn create_element(&mut self, tag: &str) -> i32;
    fn create_text(&mut self, text: &str) -> i32;
    fn create_placeholder(&mut self) -> i32;
    fn append_child(&mut self, parent: i32, child: i32);
    fn remove_child(&mut self, parent: i32, child: i32);
    fn insert_before(&mut self, parent: i32, child: i32, before: Option<i32>);
    fn replace_element(&mut self, new: i32, old: i32);
    fn replace_elements(
        &mut self,
        parent: i32,
        inserted: &[i32],
        removed: &[i32],
        ref_id: Option<i32>,
    );
    fn parent(&self, node: i32) -> Option<i32>;
    fn next_sibling(&self, node: i32) -> Option<i32>;
    fn unique_id(&self, node: i32) -> i64;
    fn set_attribute(&mut self, node: i32, name: &str, value: &str);
    fn remove_attribute(&mut self, node: i32, name: &str);
    fn add_event_listener(&mut self, node: i32, name: &str, listener: i32);
    fn remove_event_listener(&mut self, node: i32, name: &str, listener: i32);
    fn commit(&mut self);
}

#[derive(Default)]
pub struct RealHost;

impl Host for RealHost {
    fn page_root(&mut self) -> i32 {
        raw::get_page_element().unwrap_or_else(raw::create_page)
    }

    fn create_element(&mut self, tag: &str) -> i32 {
        raw::create_element(tag)
    }

    fn create_text(&mut self, text: &str) -> i32 {
        raw::create_raw_text(text)
    }

    fn create_placeholder(&mut self) -> i32 {
        raw::create_non_element()
    }

    fn append_child(&mut self, parent: i32, child: i32) {
        raw::append_element(parent, child);
    }

    fn remove_child(&mut self, parent: i32, child: i32) {
        raw::remove_element(parent, child);
    }

    fn insert_before(&mut self, parent: i32, child: i32, before: Option<i32>) {
        raw::insert_element_before(parent, child, before);
    }

    fn replace_element(&mut self, new: i32, old: i32) {
        raw::replace_element(new, old);
    }

    fn replace_elements(
        &mut self,
        parent: i32,
        inserted: &[i32],
        removed: &[i32],
        ref_id: Option<i32>,
    ) {
        raw::replace_elements(parent, inserted, removed, ref_id);
    }

    fn parent(&self, node: i32) -> Option<i32> {
        raw::get_parent(node)
    }

    fn next_sibling(&self, node: i32) -> Option<i32> {
        raw::next_element(node)
    }

    fn unique_id(&self, node: i32) -> i64 {
        raw::get_element_unique_id(node)
    }

    fn set_attribute(&mut self, node: i32, name: &str, value: &str) {
        if name == "class" {
            raw::set_classes(node, value);
        } else {
            raw::set_string_attribute(node, name, value);
        }
    }

    fn remove_attribute(&mut self, node: i32, name: &str) {
        if name == "class" {
            raw::set_classes(node, "");
        } else {
            raw::remove_attribute(node, name);
        }
    }

    fn add_event_listener(&mut self, node: i32, name: &str, listener: i32) {
        raw::add_event_listener(node, name, listener, 0);
    }

    fn remove_event_listener(&mut self, node: i32, name: &str, listener: i32) {
        raw::remove_event_listener(node, name, listener, 0);
    }

    fn commit(&mut self) {
        let _ = dioxus_lynx_sys::commit();
    }
}

#[derive(Clone, Debug)]
struct StackNode {
    raw: i32,
    paths: FxHashMap<Vec<u8>, i32>,
}

impl StackNode {
    fn single(raw: i32) -> Self {
        let mut paths = FxHashMap::default();
        paths.insert(Vec::new(), raw);
        Self { raw, paths }
    }
}

pub struct LynxMutations<H: Host = RealHost> {
    host: H,
    nodes: FxHashMap<ElementId, i32>,
    unique_to_element: FxHashMap<i64, ElementId>,
    stack: Vec<StackNode>,
}

impl LynxMutations<RealHost> {
    pub fn new(root: i32) -> Self {
        Self::new_with_host(RealHost, root)
    }

    pub fn new_for_page() -> Self {
        let mut host = RealHost;
        let root = host.page_root();
        Self::new_with_host(host, root)
    }
}

impl<H: Host> LynxMutations<H> {
    pub fn new_with_host(host: H, root: i32) -> Self {
        let mut this = Self {
            host,
            nodes: FxHashMap::default(),
            unique_to_element: FxHashMap::default(),
            stack: Vec::new(),
        };
        this.bind_node(ElementId(0), root);
        this
    }

    pub fn host(&self) -> &H {
        &self.host
    }

    pub fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }

    pub fn raw_node(&self, id: ElementId) -> Option<i32> {
        self.nodes.get(&id).copied()
    }

    pub fn element_id_for_unique_id(&self, unique_id: i64) -> Option<ElementId> {
        self.unique_to_element.get(&unique_id).copied()
    }

    fn bind_node(&mut self, id: ElementId, raw_node: i32) {
        self.nodes.insert(id, raw_node);
        let unique_id = self.host.unique_id(raw_node);
        if unique_id >= 0 {
            self.unique_to_element.insert(unique_id, id);
        }
    }

    fn pop_nodes(&mut self, m: usize) -> Vec<StackNode> {
        let start = self
            .stack
            .len()
            .checked_sub(m)
            .expect("Dioxus Lynx renderer stack underflow");
        self.stack.split_off(start)
    }

    fn create_template_node(
        &mut self,
        node: &'static TemplateNode,
        path: &mut Vec<u8>,
        paths: &mut FxHashMap<Vec<u8>, i32>,
    ) -> i32 {
        let raw_node = match node {
            TemplateNode::Element {
                tag,
                namespace: _,
                attrs,
                children,
            } => {
                let raw_node = self.host.create_element(tag);
                for attr in *attrs {
                    if let TemplateAttribute::Static {
                        name,
                        value,
                        namespace: _,
                    } = attr
                    {
                        self.host.set_attribute(raw_node, name, value);
                    }
                }

                for (index, child) in children.iter().enumerate() {
                    path.push(index as u8);
                    let child_raw = self.create_template_node(child, path, paths);
                    path.pop();
                    self.host.append_child(raw_node, child_raw);
                }
                raw_node
            }
            TemplateNode::Text { text } => self.host.create_text(text),
            TemplateNode::Dynamic { .. } => self.host.create_placeholder(),
        };

        paths.insert(path.clone(), raw_node);
        raw_node
    }

    fn replace_raw_with(&mut self, old_raw: i32, replacements: &[StackNode]) {
        if replacements.is_empty() {
            if let Some(parent) = self.host.parent(old_raw) {
                self.host.remove_child(parent, old_raw);
            }
            return;
        }

        if replacements.len() == 1 {
            self.host.replace_element(replacements[0].raw, old_raw);
            return;
        }

        let Some(parent) = self.host.parent(old_raw) else {
            return;
        };
        let ref_id = self.host.next_sibling(old_raw);
        let inserted = replacements.iter().map(|node| node.raw).collect::<Vec<_>>();
        self.host
            .replace_elements(parent, &inserted, &[old_raw], ref_id);
    }
}

impl<H: Host> WriteMutations for LynxMutations<H> {
    fn append_children(&mut self, id: ElementId, m: usize) {
        let Some(parent) = self.raw_node(id) else {
            return;
        };
        for node in self.pop_nodes(m) {
            self.host.append_child(parent, node.raw);
        }
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        let Some(node) = self.stack.last() else {
            return;
        };
        let Some(raw_node) = node.paths.get(path).copied() else {
            return;
        };
        self.bind_node(id, raw_node);
    }

    fn create_placeholder(&mut self, id: ElementId) {
        let raw_node = self.host.create_placeholder();
        self.bind_node(id, raw_node);
        self.stack.push(StackNode::single(raw_node));
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        let raw_node = self.host.create_text(value);
        self.bind_node(id, raw_node);
        self.stack.push(StackNode::single(raw_node));
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        let Some(root) = template.roots().get(index) else {
            return;
        };
        let mut paths = FxHashMap::default();
        let raw_node = self.create_template_node(root, &mut Vec::new(), &mut paths);
        self.bind_node(id, raw_node);
        self.stack.push(StackNode {
            raw: raw_node,
            paths,
        });
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        let replacements = self.pop_nodes(m);
        let Some(old_raw) = self.raw_node(id) else {
            return;
        };
        self.replace_raw_with(old_raw, &replacements);
        if let Some(first) = replacements.first() {
            self.bind_node(id, first.raw);
        }
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        let replacements = self.pop_nodes(m);
        let old_raw = self
            .stack
            .last()
            .and_then(|template_root| template_root.paths.get(path).copied());
        let Some(old_raw) = old_raw else { return };
        self.replace_raw_with(old_raw, &replacements);
        if let Some(first) = replacements.first()
            && let Some(template_root) = self.stack.last_mut()
        {
            template_root.paths.insert(path.to_vec(), first.raw);
        }
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        let replacements = self.pop_nodes(m);
        let Some(target) = self.raw_node(id) else {
            return;
        };
        let Some(parent) = self.host.parent(target) else {
            return;
        };
        let ref_child = self.host.next_sibling(target);
        for node in replacements {
            self.host.insert_before(parent, node.raw, ref_child);
        }
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        let replacements = self.pop_nodes(m);
        let Some(target) = self.raw_node(id) else {
            return;
        };
        let Some(parent) = self.host.parent(target) else {
            return;
        };
        for node in replacements {
            self.host.insert_before(parent, node.raw, Some(target));
        }
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        _ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        let Some(raw_node) = self.raw_node(id) else {
            return;
        };

        match value {
            AttributeValue::Text(value) => self.host.set_attribute(raw_node, name, value),
            AttributeValue::Float(value) => {
                self.host.set_attribute(raw_node, name, &value.to_string())
            }
            AttributeValue::Int(value) => {
                self.host.set_attribute(raw_node, name, &value.to_string())
            }
            AttributeValue::Bool(value) => {
                self.host
                    .set_attribute(raw_node, name, if *value { "true" } else { "false" });
            }
            AttributeValue::None => self.host.remove_attribute(raw_node, name),
            AttributeValue::Any(_) | AttributeValue::Listener(_) => {
                #[cfg(debug_assertions)]
                eprintln!("dioxus_lynx: unsupported attribute value for `{name}`");
            }
        }
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        let Some(old_raw) = self.raw_node(id) else {
            return;
        };
        let raw_node = self.host.create_text(value);
        self.host.replace_element(raw_node, old_raw);
        self.bind_node(id, raw_node);
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        let Some(raw_node) = self.raw_node(id) else {
            return;
        };
        self.host
            .add_event_listener(raw_node, name, event_dispatcher_id());
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        let Some(raw_node) = self.raw_node(id) else {
            return;
        };
        self.host
            .remove_event_listener(raw_node, name, event_dispatcher_id());
    }

    fn remove_node(&mut self, id: ElementId) {
        let Some(raw_node) = self.raw_node(id) else {
            return;
        };
        if let Some(parent) = self.host.parent(raw_node) {
            self.host.remove_child(parent, raw_node);
        }
    }

    fn push_root(&mut self, id: ElementId) {
        if let Some(raw_node) = self.raw_node(id) {
            self.stack.push(StackNode::single(raw_node));
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn event_dispatcher_id() -> i32 {
    crate::__dioxus_lynx_event_dispatch as usize as i32
}

#[cfg(not(target_arch = "wasm32"))]
fn event_dispatcher_id() -> i32 {
    0
}

pub fn mount(root: fn() -> dioxus_core::Element) -> LynxApp {
    let mut app = LynxApp::new(root);
    app.rebuild();
    app
}

pub struct LynxApp {
    vdom: VirtualDom,
    mutations: LynxMutations,
}

impl LynxApp {
    pub fn new(root: fn() -> dioxus_core::Element) -> Self {
        let vdom = VirtualDom::new(root);
        let mutations = LynxMutations::new_for_page();
        Self { vdom, mutations }
    }

    pub fn rebuild(&mut self) {
        self.vdom.rebuild(&mut self.mutations);
        self.mutations.host_mut().commit();
    }

    pub fn render_immediate(&mut self) {
        self.vdom.render_immediate(&mut self.mutations);
        self.mutations.host_mut().commit();
    }

    pub fn dispatch_raw_event(&mut self, event_id: i32) {
        let Some(event) = dioxus_lynx_sys::Event::from_raw(event_id) else {
            return;
        };
        let target = dioxus_lynx_sys::event_current_target_unique_id(event.raw());
        let Some(element) = target.and_then(|id| self.mutations.element_id_for_unique_id(id))
        else {
            return;
        };
        let Some(event_type) = dioxus_lynx_sys::event_type(event.raw()).ok().flatten() else {
            return;
        };
        let event = events::event_for_runtime(event_type.as_str(), target);
        self.vdom
            .runtime()
            .handle_event(event_type.as_str(), event, element);
        self.render_immediate();
    }
}

pub(crate) fn install_app(app: LynxApp) {
    APP.with(|slot| {
        *slot.borrow_mut() = Some(app);
    });
}

thread_local! {
    static APP: std::cell::RefCell<Option<LynxApp>> = const { std::cell::RefCell::new(None) };
}

pub fn with_app<R>(f: impl FnOnce(Option<&mut LynxApp>) -> R) -> R {
    APP.with(|slot| f(slot.borrow_mut().as_mut()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[derive(Clone, Debug, Default)]
    struct RecordedNode {
        tag: String,
        text: Option<String>,
        parent: Option<i32>,
        children: Vec<i32>,
        attrs: FxHashMap<String, String>,
    }

    #[derive(Default)]
    struct RecordingHost {
        next: i32,
        nodes: FxHashMap<i32, RecordedNode>,
        listeners: Vec<(i32, String)>,
    }

    impl RecordingHost {
        fn alloc(&mut self, tag: &str, text: Option<&str>) -> i32 {
            let id = self.next;
            self.next += 1;
            self.nodes.insert(
                id,
                RecordedNode {
                    tag: tag.to_string(),
                    text: text.map(ToOwned::to_owned),
                    parent: None,
                    children: Vec::new(),
                    attrs: FxHashMap::default(),
                },
            );
            id
        }

        fn node(&self, id: i32) -> &RecordedNode {
            self.nodes.get(&id).unwrap()
        }

        fn detach(&mut self, child: i32) {
            let parent = self.nodes.get(&child).and_then(|node| node.parent);
            if let Some(parent) = parent
                && let Some(parent_node) = self.nodes.get_mut(&parent)
            {
                parent_node.children.retain(|id| *id != child);
            }
            if let Some(child_node) = self.nodes.get_mut(&child) {
                child_node.parent = None;
            }
        }
    }

    impl Host for RecordingHost {
        fn page_root(&mut self) -> i32 {
            self.alloc("page", None)
        }

        fn create_element(&mut self, tag: &str) -> i32 {
            self.alloc(tag, None)
        }

        fn create_text(&mut self, text: &str) -> i32 {
            self.alloc("#text", Some(text))
        }

        fn create_placeholder(&mut self) -> i32 {
            self.alloc("#placeholder", None)
        }

        fn append_child(&mut self, parent: i32, child: i32) {
            self.detach(child);
            self.nodes.get_mut(&parent).unwrap().children.push(child);
            self.nodes.get_mut(&child).unwrap().parent = Some(parent);
        }

        fn remove_child(&mut self, parent: i32, child: i32) {
            self.nodes
                .get_mut(&parent)
                .unwrap()
                .children
                .retain(|id| *id != child);
            self.nodes.get_mut(&child).unwrap().parent = None;
        }

        fn insert_before(&mut self, parent: i32, child: i32, before: Option<i32>) {
            self.detach(child);
            let children = &mut self.nodes.get_mut(&parent).unwrap().children;
            let index = before
                .and_then(|before| children.iter().position(|id| *id == before))
                .unwrap_or(children.len());
            children.insert(index, child);
            self.nodes.get_mut(&child).unwrap().parent = Some(parent);
        }

        fn replace_element(&mut self, new: i32, old: i32) {
            if let Some(parent) = self.parent(old) {
                let children = &mut self.nodes.get_mut(&parent).unwrap().children;
                if let Some(index) = children.iter().position(|id| *id == old) {
                    children[index] = new;
                }
                self.nodes.get_mut(&old).unwrap().parent = None;
                self.nodes.get_mut(&new).unwrap().parent = Some(parent);
            }
        }

        fn replace_elements(
            &mut self,
            parent: i32,
            inserted: &[i32],
            removed: &[i32],
            ref_id: Option<i32>,
        ) {
            for removed in removed {
                self.remove_child(parent, *removed);
            }
            for inserted in inserted.iter().rev() {
                self.insert_before(parent, *inserted, ref_id);
            }
        }

        fn parent(&self, node: i32) -> Option<i32> {
            self.nodes.get(&node).and_then(|node| node.parent)
        }

        fn next_sibling(&self, node: i32) -> Option<i32> {
            let parent = self.parent(node)?;
            let siblings = &self.nodes.get(&parent)?.children;
            let index = siblings.iter().position(|id| *id == node)?;
            siblings.get(index + 1).copied()
        }

        fn unique_id(&self, node: i32) -> i64 {
            node as i64
        }

        fn set_attribute(&mut self, node: i32, name: &str, value: &str) {
            self.nodes
                .get_mut(&node)
                .unwrap()
                .attrs
                .insert(name.to_string(), value.to_string());
        }

        fn remove_attribute(&mut self, node: i32, name: &str) {
            self.nodes.get_mut(&node).unwrap().attrs.remove(name);
        }

        fn add_event_listener(&mut self, node: i32, name: &str, _listener: i32) {
            self.listeners.push((node, name.to_string()));
        }

        fn remove_event_listener(&mut self, node: i32, name: &str, _listener: i32) {
            self.listeners
                .retain(|(seen_node, seen_name)| *seen_node != node || seen_name != name);
        }

        fn commit(&mut self) {}
    }

    #[allow(non_snake_case)]
    fn TemplateApp() -> Element {
        rsx! {
            view { class: "root",
                text { "hello" }
            }
        }
    }

    #[allow(non_snake_case)]
    fn EventApp() -> Element {
        rsx! {
            view { ontap: move |_| {} }
        }
    }

    #[test]
    fn rebuild_creates_template_tree() {
        let mut host = RecordingHost::default();
        let root = host.page_root();
        let mut mutations = LynxMutations::new_with_host(host, root);
        let mut dom = VirtualDom::new(TemplateApp);

        dom.rebuild(&mut mutations);

        let root_node = mutations.host().node(root);
        assert_eq!(root_node.children.len(), 1);
        let view = root_node.children[0];
        assert_eq!(mutations.host().node(view).tag, "view");
        assert_eq!(
            mutations
                .host()
                .node(view)
                .attrs
                .get("class")
                .map(String::as_str),
            Some("root")
        );
        let text_element = mutations.host().node(view).children[0];
        assert_eq!(mutations.host().node(text_element).tag, "text");
        let text = mutations.host().node(text_element).children[0];
        assert_eq!(mutations.host().node(text).text.as_deref(), Some("hello"));
    }

    #[test]
    fn set_node_text_replaces_text_node() {
        let mut host = RecordingHost::default();
        let root = host.page_root();
        let mut mutations = LynxMutations::new_with_host(host, root);

        mutations.create_text_node("before", ElementId(1));
        mutations.append_children(ElementId(0), 1);
        mutations.set_node_text("after", ElementId(1));

        let text = mutations.host().node(root).children[0];
        assert_eq!(mutations.host().node(text).text.as_deref(), Some("after"));
    }

    #[test]
    fn event_listener_is_registered_with_stripped_name() {
        let mut host = RecordingHost::default();
        let root = host.page_root();
        let mut mutations = LynxMutations::new_with_host(host, root);
        let mut dom = VirtualDom::new(EventApp);

        dom.rebuild(&mut mutations);

        assert_eq!(mutations.host().listeners.len(), 1);
        assert_eq!(mutations.host().listeners[0].1, "tap");
    }
}
