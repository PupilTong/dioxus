use crate::properties::RootProps;
use crate::{DynamicNode, Element, Template, TemplateNode, VComponent, VNode};
#[cfg(not(feature = "minimal-root"))]
use crate::{ErrorBoundary, Properties, SuspenseBoundary, fc_to_builder};

// We wrap the root scope in a component that renders it inside a default ErrorBoundary and SuspenseBoundary
#[allow(non_snake_case)]
#[allow(clippy::let_and_return)]
pub(crate) fn RootScopeWrapper(props: RootProps<VComponent>) -> Element {
    static TEMPLATE: Template =
        Template::new(&[TemplateNode::Dynamic { id: 0usize }], &[&[0u8]], &[]);

    #[cfg(feature = "minimal-root")]
    {
        return Element::Ok(VNode::new(
            None,
            TEMPLATE,
            Box::new([DynamicNode::Component(props.0)]),
            Box::new([]),
        ));
    }

    #[cfg(not(feature = "minimal-root"))]
    Element::Ok(VNode::new(
        None,
        TEMPLATE,
        Box::new([DynamicNode::Component(
            fc_to_builder(SuspenseBoundary)
                .fallback(|_| Element::Ok(VNode::placeholder()))
                .children(Ok(VNode::new(
                    None,
                    TEMPLATE,
                    Box::new([DynamicNode::Component({
                        fc_to_builder(ErrorBoundary)
                            .children(Element::Ok(VNode::new(
                                None,
                                TEMPLATE,
                                Box::new([DynamicNode::Component(props.0)]),
                                Box::new([]),
                            )))
                            .build()
                            .into_vcomponent(ErrorBoundary)
                    })]),
                    Box::new([]),
                )))
                .build()
                .into_vcomponent(SuspenseBoundary),
        )]),
        Box::new([]),
    ))
}
