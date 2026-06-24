use dioxus_core::{
    AttributeValue, DynamicNode, ElementId, Template, TemplateAttribute, TemplateNode, VNode,
    VirtualDom, WriteMutations,
};
use dioxus_lynx_sys::raw;
use rustc_hash::FxHashMap;
use std::rc::Rc;

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
        match tag {
            "view" => raw::create_view(),
            "text" => raw::create_text(),
            "image" => raw::create_image(),
            "scroll-view" => raw::create_scroll_view(),
            "page" => raw::create_page(),
            "wrapper" => raw::create_wrapper_element(),
            _ => raw::create_element(tag),
        }
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
        } else if name == "style" {
            raw::set_inline_style_text(node, value);
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
enum TemplateOp {
    CreateElement {
        slot: usize,
        tag: &'static str,
    },
    CreateText {
        slot: usize,
        text: &'static str,
    },
    CreatePlaceholder,
    SetStaticAttr {
        slot: usize,
        name: &'static str,
        value: &'static str,
    },
    AppendChild {
        parent: usize,
        child: usize,
    },
}

#[derive(Clone, Debug)]
struct StaticAttr {
    name: &'static str,
    value: &'static str,
}

#[derive(Clone, Debug)]
struct SingleElementTemplate {
    tag: &'static str,
    static_attrs: Box<[StaticAttr]>,
    dynamic_child: bool,
}

#[derive(Clone, Debug)]
struct PathSlot {
    path: Box<[u8]>,
    slot: usize,
}

#[derive(Clone, Debug)]
struct PlaceholderSlot {
    slot: usize,
    parent: usize,
    next_sibling: Option<usize>,
}

#[derive(Clone, Debug)]
struct TemplateProgram {
    single_element: Option<SingleElementTemplate>,
    ops: Box<[TemplateOp]>,
    path_slots: Box<[PathSlot]>,
    placeholder_slots: Box<[PlaceholderSlot]>,
    slot_count: usize,
    root_slot: usize,
}

impl TemplateProgram {
    fn compile(root: &'static TemplateNode) -> Self {
        let single_element = Self::compile_single_element(root);
        let mut ops = Vec::new();
        let mut path_slots = Vec::new();
        let mut placeholder_slots = Vec::new();
        let mut next_slot = 0;
        let root_slot = Self::compile_node(
            root,
            &mut Vec::new(),
            &mut next_slot,
            &mut ops,
            &mut path_slots,
            &mut placeholder_slots,
        );
        Self {
            single_element,
            ops: ops.into_boxed_slice(),
            path_slots: path_slots.into_boxed_slice(),
            placeholder_slots: placeholder_slots.into_boxed_slice(),
            slot_count: next_slot,
            root_slot,
        }
    }

    fn compile_single_element(root: &'static TemplateNode) -> Option<SingleElementTemplate> {
        let TemplateNode::Element {
            tag,
            namespace: None,
            attrs,
            children,
        } = root
        else {
            return None;
        };

        let dynamic_child = match *children {
            [] => false,
            [TemplateNode::Dynamic { .. }] => true,
            _ => return None,
        };

        let mut static_attrs = Vec::new();
        for attr in *attrs {
            match attr {
                TemplateAttribute::Static {
                    name,
                    value,
                    namespace: None,
                } => static_attrs.push(StaticAttr { name, value }),
                TemplateAttribute::Dynamic { .. } => {}
                TemplateAttribute::Static { .. } => return None,
            }
        }

        Some(SingleElementTemplate {
            tag,
            static_attrs: static_attrs.into_boxed_slice(),
            dynamic_child,
        })
    }

    fn compile_node(
        node: &'static TemplateNode,
        path: &mut Vec<u8>,
        next_slot: &mut usize,
        ops: &mut Vec<TemplateOp>,
        path_slots: &mut Vec<PathSlot>,
        placeholder_slots: &mut Vec<PlaceholderSlot>,
    ) -> usize {
        let slot = *next_slot;
        *next_slot += 1;
        path_slots.push(PathSlot {
            path: path.clone().into_boxed_slice(),
            slot,
        });

        match node {
            TemplateNode::Element {
                tag,
                namespace: _,
                attrs,
                children,
            } => {
                ops.push(TemplateOp::CreateElement { slot, tag });
                for attr in *attrs {
                    if let TemplateAttribute::Static {
                        name,
                        value,
                        namespace: _,
                    } = attr
                    {
                        ops.push(TemplateOp::SetStaticAttr { slot, name, value });
                    }
                }

                let mut child_slots = Vec::with_capacity(children.len());
                for (index, child) in children.iter().enumerate() {
                    path.push(index as u8);
                    let child_slot = Self::compile_node(
                        child,
                        path,
                        next_slot,
                        ops,
                        path_slots,
                        placeholder_slots,
                    );
                    path.pop();
                    child_slots.push(child_slot);
                }

                for (index, child_slot) in child_slots.iter().copied().enumerate() {
                    if matches!(children[index], TemplateNode::Dynamic { .. }) {
                        placeholder_slots.push(PlaceholderSlot {
                            slot: child_slot,
                            parent: slot,
                            next_sibling: child_slots.get(index + 1).copied(),
                        });
                    }
                    ops.push(TemplateOp::AppendChild {
                        parent: slot,
                        child: child_slot,
                    });
                }
            }
            TemplateNode::Text { text } => ops.push(TemplateOp::CreateText { slot, text }),
            TemplateNode::Dynamic { .. } => ops.push(TemplateOp::CreatePlaceholder),
        }

        slot
    }

    fn slot_for_path(&self, path: &[u8]) -> Option<usize> {
        self.path_slots
            .iter()
            .find(|entry| entry.path.as_ref() == path)
            .map(|entry| entry.slot)
    }

    fn placeholder_for_slot(&self, slot: usize) -> Option<&PlaceholderSlot> {
        self.placeholder_slots
            .iter()
            .find(|entry| entry.slot == slot)
    }
}

#[derive(Debug)]
enum SlotStorage {
    Inline { slots: [i32; 2], len: usize },
    Heap(Vec<i32>),
}

impl SlotStorage {
    fn new(len: usize) -> Self {
        if len <= 2 {
            Self::Inline {
                slots: [raw::NULL_NODE; 2],
                len,
            }
        } else {
            Self::Heap(vec![raw::NULL_NODE; len])
        }
    }

    fn get(&self, index: usize) -> Option<i32> {
        match self {
            Self::Inline { slots, len } => (index < *len).then_some(slots[index]),
            Self::Heap(slots) => slots.get(index).copied(),
        }
    }

    fn set(&mut self, index: usize, value: i32) -> bool {
        match self {
            Self::Inline { slots, len } if index < *len => {
                slots[index] = value;
                true
            }
            Self::Heap(slots) => {
                if let Some(slot) = slots.get_mut(index) {
                    *slot = value;
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

#[derive(Debug)]
enum TemplateInstance {
    Program {
        program: Rc<TemplateProgram>,
        slots: SlotStorage,
    },
    SingleDynamicChild {
        parent: i32,
        child: i32,
    },
}

impl TemplateInstance {
    fn raw_for_path(&self, path: &[u8]) -> Option<i32> {
        match self {
            Self::Program { program, slots } => {
                let slot = program.slot_for_path(path)?;
                slots.get(slot).filter(|raw| *raw != raw::NULL_NODE)
            }
            Self::SingleDynamicChild { child, .. } => {
                (path == [0] && *child != raw::NULL_NODE).then_some(*child)
            }
        }
    }

    fn set_raw_for_path(&mut self, path: &[u8], raw_node: i32) {
        match self {
            Self::Program { program, slots } => {
                if let Some(slot) = program.slot_for_path(path) {
                    slots.set(slot, raw_node);
                }
            }
            Self::SingleDynamicChild { child, .. } if path == [0] => {
                *child = raw_node;
            }
            Self::SingleDynamicChild { .. } => {}
        }
    }

    fn insertion_point_for_path(&self, path: &[u8]) -> Option<(i32, Option<i32>)> {
        match self {
            Self::Program { program, slots } => {
                let slot = program.slot_for_path(path)?;
                let placeholder = program.placeholder_for_slot(slot)?;
                let parent = slots
                    .get(placeholder.parent)
                    .filter(|raw| *raw != raw::NULL_NODE)?;
                let next_sibling = placeholder
                    .next_sibling
                    .and_then(|slot| slots.get(slot))
                    .filter(|raw| *raw != raw::NULL_NODE);
                Some((parent, next_sibling))
            }
            Self::SingleDynamicChild { parent, .. } if path == [0] => Some((*parent, None)),
            Self::SingleDynamicChild { .. } => None,
        }
    }
}

#[derive(Debug)]
struct StackNode {
    raw: i32,
    template: Option<TemplateInstance>,
}

impl StackNode {
    fn single(raw: i32) -> Self {
        Self {
            raw,
            template: None,
        }
    }

    fn template(raw: i32, template: TemplateInstance) -> Self {
        Self {
            raw,
            template: Some(template),
        }
    }

    fn raw_for_path(&self, path: &[u8]) -> Option<i32> {
        if path.is_empty() {
            return Some(self.raw);
        }
        self.template.as_ref()?.raw_for_path(path)
    }

    fn set_raw_for_path(&mut self, path: &[u8], raw_node: i32) {
        if path.is_empty() {
            self.raw = raw_node;
            return;
        }
        if let Some(template) = self.template.as_mut() {
            template.set_raw_for_path(path, raw_node);
        }
    }
}

pub struct LynxMutations<H: Host = RealHost> {
    host: H,
    nodes: Vec<i32>,
    unique_to_element: FxHashMap<i64, ElementId>,
    listener_counts: Vec<usize>,
    templates: FxHashMap<Template, Box<[Rc<TemplateProgram>]>>,
    stack: Vec<StackNode>,
    mutated: bool,
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
            nodes: Vec::new(),
            unique_to_element: FxHashMap::default(),
            listener_counts: Vec::new(),
            templates: FxHashMap::default(),
            stack: Vec::new(),
            mutated: false,
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
        self.nodes
            .get(id.0)
            .copied()
            .filter(|raw| *raw != raw::NULL_NODE)
    }

    pub fn element_id_for_unique_id(&self, unique_id: i64) -> Option<ElementId> {
        self.unique_to_element.get(&unique_id).copied()
    }

    fn bind_node(&mut self, id: ElementId, raw_node: i32) {
        self.ensure_node_slot(id);
        self.nodes[id.0] = raw_node;
        if self.listener_count(id) > 0 {
            self.bind_event_target(id, raw_node);
        }
    }

    fn unbind_node(&mut self, id: ElementId) {
        if let Some(raw) = self.nodes.get_mut(id.0) {
            *raw = raw::NULL_NODE;
        }
        if let Some(count) = self.listener_counts.get_mut(id.0) {
            *count = 0;
        }
        self.unique_to_element.retain(|_, element| *element != id);
    }

    fn ensure_node_slot(&mut self, id: ElementId) {
        if self.nodes.len() <= id.0 {
            self.nodes.resize(id.0 + 1, raw::NULL_NODE);
        }
    }

    fn ensure_listener_slot(&mut self, id: ElementId) {
        if self.listener_counts.len() <= id.0 {
            self.listener_counts.resize(id.0 + 1, 0);
        }
    }

    fn listener_count(&self, id: ElementId) -> usize {
        self.listener_counts.get(id.0).copied().unwrap_or(0)
    }

    fn bind_event_target(&mut self, id: ElementId, raw_node: i32) {
        self.unique_to_element.retain(|_, element| *element != id);
        let unique_id = self.host.unique_id(raw_node);
        if unique_id >= 0 {
            self.unique_to_element.insert(unique_id, id);
        }
    }

    fn unbind_event_target(&mut self, id: ElementId) {
        self.unique_to_element.retain(|_, element| *element != id);
    }

    fn mark_mutated(&mut self) {
        self.mutated = true;
    }

    fn commit_if_mutated(&mut self) {
        if self.mutated {
            self.mutated = false;
            self.host.commit();
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

    fn template_program(
        &mut self,
        template: Template,
        index: usize,
    ) -> Option<Rc<TemplateProgram>> {
        if !self.templates.contains_key(&template) {
            let roots = template
                .roots()
                .iter()
                .map(|root| Rc::new(TemplateProgram::compile(root)))
                .collect::<Vec<_>>()
                .into_boxed_slice();
            self.templates.insert(template, roots);
        }
        self.templates
            .get(&template)
            .and_then(|roots| roots.get(index))
            .cloned()
    }

    fn replay_template(&mut self, program: Rc<TemplateProgram>) -> Option<StackNode> {
        if let Some(single) = &program.single_element {
            let raw_node = self.host.create_element(single.tag);
            self.mark_mutated();
            for attr in single.static_attrs.iter() {
                self.host.set_attribute(raw_node, attr.name, attr.value);
                self.mark_mutated();
            }

            return Some(if single.dynamic_child {
                StackNode::template(
                    raw_node,
                    TemplateInstance::SingleDynamicChild {
                        parent: raw_node,
                        child: raw::NULL_NODE,
                    },
                )
            } else {
                StackNode::single(raw_node)
            });
        }

        let mut slots = SlotStorage::new(program.slot_count);
        for op in program.ops.iter() {
            match *op {
                TemplateOp::CreateElement { slot, tag } => {
                    slots.set(slot, self.host.create_element(tag));
                    self.mark_mutated();
                }
                TemplateOp::CreateText { slot, text } => {
                    slots.set(slot, self.host.create_text(text));
                    self.mark_mutated();
                }
                TemplateOp::CreatePlaceholder => {}
                TemplateOp::SetStaticAttr { slot, name, value } => {
                    let raw_node = slots.get(slot).unwrap_or(raw::NULL_NODE);
                    if raw_node != raw::NULL_NODE {
                        self.host.set_attribute(raw_node, name, value);
                        self.mark_mutated();
                    }
                }
                TemplateOp::AppendChild { parent, child } => {
                    let parent = slots.get(parent).unwrap_or(raw::NULL_NODE);
                    let child = slots.get(child).unwrap_or(raw::NULL_NODE);
                    if parent != raw::NULL_NODE && child != raw::NULL_NODE {
                        self.host.append_child(parent, child);
                        self.mark_mutated();
                    }
                }
            }
        }

        let raw_node = slots.get(program.root_slot)?;
        (raw_node != raw::NULL_NODE).then(|| {
            StackNode::template(
                raw_node,
                TemplateInstance::Program {
                    program: program.clone(),
                    slots,
                },
            )
        })
    }

    fn insert_replacements_before(
        &mut self,
        parent: i32,
        replacements: &[StackNode],
        before: Option<i32>,
    ) {
        if replacements.is_empty() {
            return;
        }
        let inserted = replacements.iter().map(|node| node.raw).collect::<Vec<_>>();
        self.host.replace_elements(parent, &inserted, &[], before);
        self.mark_mutated();
    }

    fn replace_raw_with(&mut self, old_raw: i32, replacements: &[StackNode]) {
        if replacements.is_empty() {
            if let Some(parent) = self.host.parent(old_raw) {
                self.host.remove_child(parent, old_raw);
                self.mark_mutated();
            }
            return;
        }

        if replacements.len() == 1 {
            self.host.replace_element(replacements[0].raw, old_raw);
            self.mark_mutated();
            return;
        }

        let Some(parent) = self.host.parent(old_raw) else {
            return;
        };
        let ref_id = self.host.next_sibling(old_raw);
        let inserted = replacements.iter().map(|node| node.raw).collect::<Vec<_>>();
        self.host
            .replace_elements(parent, &inserted, &[old_raw], ref_id);
        self.mark_mutated();
    }
}

impl<H: Host> WriteMutations for LynxMutations<H> {
    fn append_children(&mut self, id: ElementId, m: usize) {
        let Some(parent) = self.raw_node(id) else {
            return;
        };
        for node in self.pop_nodes(m) {
            self.host.append_child(parent, node.raw);
            self.mark_mutated();
        }
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        let Some(node) = self.stack.last() else {
            return;
        };
        let Some(raw_node) = node.raw_for_path(path) else {
            return;
        };
        self.bind_node(id, raw_node);
    }

    fn create_placeholder(&mut self, id: ElementId) {
        let raw_node = self.host.create_placeholder();
        self.mark_mutated();
        self.bind_node(id, raw_node);
        self.stack.push(StackNode::single(raw_node));
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        let raw_node = self.host.create_text(value);
        self.mark_mutated();
        self.bind_node(id, raw_node);
        self.stack.push(StackNode::single(raw_node));
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        let Some(node) = self
            .template_program(template, index)
            .and_then(|program| self.replay_template(program))
        else {
            return;
        };
        self.bind_node(id, node.raw);
        self.stack.push(node);
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        let replacements = self.pop_nodes(m);
        let Some(old_raw) = self.raw_node(id) else {
            return;
        };
        self.replace_raw_with(old_raw, &replacements);
        if let Some(first) = replacements.first() {
            self.bind_node(id, first.raw);
        } else {
            self.unbind_node(id);
        }
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        let replacements = self.pop_nodes(m);
        let (old_raw, insertion_point) = self
            .stack
            .last()
            .map(|template_root| {
                (
                    template_root.raw_for_path(path),
                    template_root
                        .template
                        .as_ref()
                        .and_then(|template| template.insertion_point_for_path(path)),
                )
            })
            .unwrap_or((None, None));

        if let Some(old_raw) = old_raw {
            self.replace_raw_with(old_raw, &replacements);
        } else if let Some((parent, before)) = insertion_point {
            self.insert_replacements_before(parent, &replacements, before);
        } else {
            return;
        }

        let replacement_raw = replacements
            .first()
            .map(|first| first.raw)
            .unwrap_or(raw::NULL_NODE);
        if let Some(template_root) = self.stack.last_mut() {
            template_root.set_raw_for_path(path, replacement_raw);
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
            self.mark_mutated();
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
            self.mark_mutated();
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
            AttributeValue::Text(value) => {
                self.host.set_attribute(raw_node, name, value);
                self.mark_mutated();
            }
            AttributeValue::Float(value) => {
                self.host.set_attribute(raw_node, name, &value.to_string());
                self.mark_mutated();
            }
            AttributeValue::Int(value) => {
                self.host.set_attribute(raw_node, name, &value.to_string());
                self.mark_mutated();
            }
            AttributeValue::Bool(value) => {
                self.host
                    .set_attribute(raw_node, name, if *value { "true" } else { "false" });
                self.mark_mutated();
            }
            AttributeValue::None => {
                self.host.remove_attribute(raw_node, name);
                self.mark_mutated();
            }
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
        self.mark_mutated();
        self.host.replace_element(raw_node, old_raw);
        self.mark_mutated();
        self.bind_node(id, raw_node);
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        let Some(raw_node) = self.raw_node(id) else {
            return;
        };
        self.ensure_listener_slot(id);
        let old_count = self.listener_counts[id.0];
        if old_count == 0 {
            self.bind_event_target(id, raw_node);
        }
        self.listener_counts[id.0] = old_count + 1;
        self.host
            .add_event_listener(raw_node, name, event_dispatcher_id());
        self.mark_mutated();
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        let Some(raw_node) = self.raw_node(id) else {
            return;
        };
        self.host
            .remove_event_listener(raw_node, name, event_dispatcher_id());
        self.mark_mutated();
        if let Some(count) = self.listener_counts.get_mut(id.0) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.unbind_event_target(id);
            }
        }
    }

    fn remove_node(&mut self, id: ElementId) {
        let Some(raw_node) = self.raw_node(id) else {
            return;
        };
        if let Some(parent) = self.host.parent(raw_node) {
            self.host.remove_child(parent, raw_node);
            self.mark_mutated();
        }
        self.unbind_node(id);
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

pub fn mount_static(root: fn() -> dioxus_core::Element) {
    let mut host = RealHost;
    let root_raw = host.page_root();
    let mut mutated = false;
    if let Ok(vnode) = root() {
        mutated = render_static_vnode(&mut host, root_raw, &vnode) || mutated;
    }
    if mutated {
        host.commit();
    }
}

fn render_static_vnode<H: Host>(host: &mut H, parent: i32, vnode: &VNode) -> bool {
    let mut mutated = false;
    for root in vnode.template.roots() {
        mutated = render_static_template_node(host, parent, vnode, root) || mutated;
    }
    mutated
}

fn render_static_template_node<H: Host>(
    host: &mut H,
    parent: i32,
    vnode: &VNode,
    node: &TemplateNode,
) -> bool {
    match node {
        TemplateNode::Element {
            tag,
            namespace: _,
            attrs,
            children,
        } => {
            let raw = host.create_element(tag);
            for attr in *attrs {
                match attr {
                    TemplateAttribute::Static {
                        name,
                        value,
                        namespace: _,
                    } => host.set_attribute(raw, name, value),
                    TemplateAttribute::Dynamic { id } => {
                        for attr in &*vnode.dynamic_attrs[*id] {
                            set_static_attribute(host, raw, attr.name, &attr.value);
                        }
                    }
                }
            }
            for child in *children {
                render_static_template_node(host, raw, vnode, child);
            }
            host.append_child(parent, raw);
            true
        }
        TemplateNode::Text { text } => {
            let raw = host.create_text(text);
            host.append_child(parent, raw);
            true
        }
        TemplateNode::Dynamic { id } => {
            render_static_dynamic_node(host, parent, &vnode.dynamic_nodes[*id])
        }
    }
}

fn render_static_dynamic_node<H: Host>(host: &mut H, parent: i32, node: &DynamicNode) -> bool {
    match node {
        DynamicNode::Fragment(nodes) => {
            let mut mutated = false;
            for vnode in nodes {
                mutated = render_static_vnode(host, parent, vnode) || mutated;
            }
            mutated
        }
        DynamicNode::Text(text) => {
            let raw = host.create_text(&text.value);
            host.append_child(parent, raw);
            true
        }
        DynamicNode::Placeholder(_) => false,
        DynamicNode::Component(_) => {
            #[cfg(debug_assertions)]
            eprintln!("dioxus_lynx: static mount does not render child components");
            false
        }
    }
}

fn set_static_attribute<H: Host>(host: &mut H, raw_node: i32, name: &str, value: &AttributeValue) {
    match value {
        AttributeValue::Text(value) => host.set_attribute(raw_node, name, value),
        AttributeValue::Float(value) => host.set_attribute(raw_node, name, &value.to_string()),
        AttributeValue::Int(value) => host.set_attribute(raw_node, name, &value.to_string()),
        AttributeValue::Bool(value) => {
            host.set_attribute(raw_node, name, if *value { "true" } else { "false" })
        }
        AttributeValue::None => host.remove_attribute(raw_node, name),
        AttributeValue::Any(_) | AttributeValue::Listener(_) => {
            #[cfg(debug_assertions)]
            eprintln!("dioxus_lynx: unsupported static attribute value for `{name}`");
        }
    }
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
        self.mutations.commit_if_mutated();
    }

    pub fn render_immediate(&mut self) {
        self.vdom.render_immediate(&mut self.mutations);
        self.mutations.commit_if_mutated();
    }

    pub fn dispatch_external_callback(&mut self, callback: &mut dyn FnMut()) {
        self.vdom.in_runtime(|| callback());
        self.render_immediate();
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
    install_external_callback_hook();
    APP.with(|slot| {
        *slot.borrow_mut() = Some(app);
    });
}

fn install_external_callback_hook() {
    dioxus_lynx_sys::set_timer_dispatch_hook(Some(Box::new(|callback| {
        with_app(|app| {
            if let Some(app) = app {
                app.dispatch_external_callback(callback);
            } else {
                callback();
            }
        });
    })));
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
    use std::cell::Cell;

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
        unique_id_calls: Cell<usize>,
        replace_elements_calls: Cell<usize>,
        commits: usize,
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
            self.replace_elements_calls
                .set(self.replace_elements_calls.get() + 1);
            for removed in removed {
                self.remove_child(parent, *removed);
            }
            for inserted in inserted {
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
            self.unique_id_calls.set(self.unique_id_calls.get() + 1);
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

        fn commit(&mut self) {
            self.commits += 1;
        }
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

    #[allow(non_snake_case)]
    fn BubblingApp() -> Element {
        let mut jumped = use_signal(|| false);
        let label = if jumped() { "jumped" } else { "idle" };

        rsx! {
            view { ontap: move |_| jumped.set(true),
                view { ontap: move |_| {},
                    text { "{label}" }
                }
            }
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
        assert_eq!(mutations.host().unique_id_calls.get(), 0);
    }

    #[test]
    fn static_mount_renders_template_tree_without_vdom_runtime() {
        let mut host = RecordingHost::default();
        let root = host.page_root();
        let vnode = TemplateApp().unwrap();

        assert!(render_static_vnode(&mut host, root, &vnode));

        let root_node = host.node(root);
        assert_eq!(root_node.children.len(), 1);
        let view = root_node.children[0];
        assert_eq!(host.node(view).tag, "view");
        assert_eq!(
            host.node(view).attrs.get("class").map(String::as_str),
            Some("root")
        );
        let text_element = host.node(view).children[0];
        assert_eq!(host.node(text_element).tag, "text");
        let text = host.node(text_element).children[0];
        assert_eq!(host.node(text).text.as_deref(), Some("hello"));
        assert_eq!(host.unique_id_calls.get(), 0);
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
        let view = mutations.host().node(root).children[0];
        assert_eq!(mutations.host().unique_id_calls.get(), 1);
        assert_eq!(
            mutations.element_id_for_unique_id(view as i64),
            Some(ElementId(1))
        );
    }

    #[test]
    fn template_path_slot_updates_after_placeholder_replacement() {
        static CHILDREN: &[TemplateNode] = &[TemplateNode::Dynamic { id: 0 }];
        static ROOTS: &[TemplateNode] = &[TemplateNode::Element {
            tag: "view",
            namespace: None,
            attrs: &[],
            children: CHILDREN,
        }];
        static NODE_PATHS: &[&[u8]] = &[&[0, 0]];
        let template = Template::new(ROOTS, NODE_PATHS, &[]);

        let mut host = RecordingHost::default();
        let root = host.page_root();
        let mut mutations = LynxMutations::new_with_host(host, root);

        mutations.load_template(template, 0, ElementId(1));
        mutations.create_text_node("dynamic", ElementId(2));
        mutations.replace_placeholder_with_nodes(&[0], 1);
        mutations.assign_node_id(&[0], ElementId(3));

        let view = mutations.raw_node(ElementId(1)).unwrap();
        let dynamic = mutations.raw_node(ElementId(3)).unwrap();
        assert_eq!(mutations.host().node(view).children, vec![dynamic]);
        assert_eq!(
            mutations.host().node(dynamic).text.as_deref(),
            Some("dynamic")
        );
    }

    #[test]
    fn template_placeholder_replacement_preserves_static_sibling_order() {
        static CHILDREN: &[TemplateNode] = &[
            TemplateNode::Dynamic { id: 0 },
            TemplateNode::Text { text: "after" },
        ];
        static ROOTS: &[TemplateNode] = &[TemplateNode::Element {
            tag: "view",
            namespace: None,
            attrs: &[],
            children: CHILDREN,
        }];
        static NODE_PATHS: &[&[u8]] = &[&[0, 0]];
        let template = Template::new(ROOTS, NODE_PATHS, &[]);

        let mut host = RecordingHost::default();
        let root = host.page_root();
        let mut mutations = LynxMutations::new_with_host(host, root);

        mutations.load_template(template, 0, ElementId(1));
        mutations.create_text_node("dynamic", ElementId(2));
        mutations.replace_placeholder_with_nodes(&[0], 1);

        let view = mutations.raw_node(ElementId(1)).unwrap();
        let children = &mutations.host().node(view).children;
        assert_eq!(children.len(), 2);
        assert_eq!(
            mutations.host().node(children[0]).text.as_deref(),
            Some("dynamic")
        );
        assert_eq!(
            mutations.host().node(children[1]).text.as_deref(),
            Some("after")
        );
    }

    #[test]
    fn skipped_placeholder_multi_replacement_uses_batch_insert() {
        static CHILDREN: &[TemplateNode] = &[
            TemplateNode::Dynamic { id: 0 },
            TemplateNode::Text { text: "after" },
        ];
        static ROOTS: &[TemplateNode] = &[TemplateNode::Element {
            tag: "view",
            namespace: None,
            attrs: &[],
            children: CHILDREN,
        }];
        static NODE_PATHS: &[&[u8]] = &[&[0, 0]];
        let template = Template::new(ROOTS, NODE_PATHS, &[]);

        let mut host = RecordingHost::default();
        let root = host.page_root();
        let mut mutations = LynxMutations::new_with_host(host, root);

        mutations.load_template(template, 0, ElementId(1));
        mutations.create_text_node("first", ElementId(2));
        mutations.create_text_node("second", ElementId(3));
        mutations.replace_placeholder_with_nodes(&[0], 2);

        let view = mutations.raw_node(ElementId(1)).unwrap();
        let children = &mutations.host().node(view).children;
        assert_eq!(children.len(), 3);
        assert_eq!(
            mutations.host().node(children[0]).text.as_deref(),
            Some("first")
        );
        assert_eq!(
            mutations.host().node(children[1]).text.as_deref(),
            Some("second")
        );
        assert_eq!(
            mutations.host().node(children[2]).text.as_deref(),
            Some("after")
        );
        assert_eq!(mutations.host().replace_elements_calls.get(), 1);
    }

    #[test]
    fn commit_only_runs_when_mutations_are_dirty() {
        let mut host = RecordingHost::default();
        let root = host.page_root();
        let mut mutations = LynxMutations::new_with_host(host, root);
        let mut dom = VirtualDom::new(TemplateApp);

        dom.rebuild(&mut mutations);
        mutations.commit_if_mutated();
        assert_eq!(mutations.host().commits, 1);

        mutations.commit_if_mutated();
        assert_eq!(mutations.host().commits, 1);

        dom.render_immediate(&mut mutations);
        mutations.commit_if_mutated();
        assert_eq!(mutations.host().commits, 1);
    }

    #[test]
    fn child_event_bubbles_to_parent_and_commits_dirty_render() {
        let mut host = RecordingHost::default();
        let root = host.page_root();
        let mut mutations = LynxMutations::new_with_host(host, root);
        let mut dom = VirtualDom::new(BubblingApp);

        dom.rebuild(&mut mutations);
        mutations.commit_if_mutated();
        assert_eq!(mutations.host().commits, 1);

        let parent = mutations.host().node(root).children[0];
        let child = mutations.host().node(parent).children[0];
        let child_id = mutations.element_id_for_unique_id(child as i64).unwrap();

        dom.runtime().handle_event(
            "tap",
            crate::events::event_for_runtime("tap", Some(child as i64)),
            child_id,
        );
        dom.render_immediate(&mut mutations);
        mutations.commit_if_mutated();

        assert_eq!(mutations.host().commits, 2);
        let text_element = mutations.host().node(child).children[0];
        let text = mutations.host().node(text_element).children[0];
        assert_eq!(mutations.host().node(text).text.as_deref(), Some("jumped"));
    }
}
