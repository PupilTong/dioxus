use std::rc::Rc;

use dioxus_core::{Attribute, AttributeValue, Event, ListenerCallback, SpawnIfAsync, SuperInto};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LynxEventData {
    event_type: String,
    current_target_unique_id: Option<i64>,
}

impl LynxEventData {
    pub fn new(event_type: impl Into<String>, current_target_unique_id: Option<i64>) -> Self {
        Self {
            event_type: event_type.into(),
            current_target_unique_id,
        }
    }

    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    pub fn current_target_unique_id(&self) -> Option<i64> {
        self.current_target_unique_id
    }
}

pub type LynxEvent = Event<LynxEventData>;

macro_rules! impl_event {
    ($($name:ident => $event_type:literal;)*) => {
        $(
            pub fn $name<__Marker>(
                f: impl SuperInto<ListenerCallback<LynxEventData>, __Marker>,
            ) -> Attribute {
                let event_handler = f.super_into();
                Attribute::new(
                    stringify!($name),
                    AttributeValue::listener(move |event: LynxEvent| {
                        event_handler.call(event.into_any());
                    }),
                    None,
                    false,
                )
            }

            #[doc(hidden)]
            pub mod $name {
                use super::*;

                pub fn call_with_explicit_closure<
                    __Marker,
                    Return: SpawnIfAsync<__Marker> + 'static,
                >(
                    event_handler: impl FnMut(LynxEvent) -> Return + 'static,
                ) -> Attribute {
                    super::$name(event_handler)
                }
            }
        )*
    };
}

impl_event! {
    onabort => "abort";
    onauxclick => "auxclick";
    onblur => "blur";
    oncancel => "cancel";
    oncanplay => "canplay";
    oncanplaythrough => "canplaythrough";
    onchange => "change";
    onclick => "click";
    onclose => "close";
    oncontextmenu => "contextmenu";
    oncopy => "copy";
    oncut => "cut";
    oncuechange => "cuechange";
    ondblclick => "dblclick";
    ondrag => "drag";
    ondragend => "dragend";
    ondragenter => "dragenter";
    ondragexit => "dragexit";
    ondragleave => "dragleave";
    ondragover => "dragover";
    ondragstart => "dragstart";
    ondrop => "drop";
    ondurationchange => "durationchange";
    onemptied => "emptied";
    onended => "ended";
    onerror => "error";
    onfocus => "focus";
    onfocusin => "focusin";
    onfocusout => "focusout";
    onformdata => "formdata";
    oninput => "input";
    oninvalid => "invalid";
    onkeydown => "keydown";
    onkeypress => "keypress";
    onkeyup => "keyup";
    onload => "load";
    onloadeddata => "loadeddata";
    onloadedmetadata => "loadedmetadata";
    onloadend => "loadend";
    onloadstart => "loadstart";
    onmousedown => "mousedown";
    onmouseenter => "mouseenter";
    onmouseleave => "mouseleave";
    onmousemove => "mousemove";
    onmouseout => "mouseout";
    onmouseover => "mouseover";
    onmouseup => "mouseup";
    onpaste => "paste";
    onpause => "pause";
    onplay => "play";
    onplaying => "playing";
    onpointerlockchange => "pointerlockchange";
    onpointerlockerror => "pointerlockerror";
    onprogress => "progress";
    onratechange => "ratechange";
    onreset => "reset";
    onresize => "resize";
    onsecuritypolicyviolation => "securitypolicyviolation";
    onseeked => "seeked";
    onseeking => "seeking";
    onselect => "select";
    onselectionchange => "selectionchange";
    onselectstart => "selectstart";
    onshow => "show";
    onslotchange => "slotchange";
    onstalled => "stalled";
    onsubmit => "submit";
    onsuspend => "suspend";
    ontap => "tap";
    ontimeupdate => "timeupdate";
    ontoggle => "toggle";
    onvolumechange => "volumechange";
    onwaiting => "waiting";
    onwheel => "wheel";
    onanimationcancel => "animationcancel";
    onanimationend => "animationend";
    onanimationiteration => "animationiteration";
    onanimationstart => "animationstart";
    ongotpointercapture => "gotpointercapture";
    onlostpointercapture => "lostpointercapture";
    onpointercancel => "pointercancel";
    onpointerdown => "pointerdown";
    onpointerenter => "pointerenter";
    onpointerleave => "pointerleave";
    onpointermove => "pointermove";
    onpointerout => "pointerout";
    onpointerover => "pointerover";
    onpointerup => "pointerup";
    onscroll => "scroll";
    ontouchcancel => "touchcancel";
    ontouchend => "touchend";
    ontouchmove => "touchmove";
    ontouchstart => "touchstart";
    ontransitioncancel => "transitioncancel";
    ontransitionend => "transitionend";
    ontransitionrun => "transitionrun";
    ontransitionstart => "transitionstart";
}

pub(crate) fn event_for_runtime(
    event_type: impl Into<String>,
    current_target_unique_id: Option<i64>,
) -> Event<dyn std::any::Any> {
    let data = LynxEventData::new(event_type, current_target_unique_id);
    Event::new(Rc::new(data) as Rc<dyn std::any::Any>, false)
}
