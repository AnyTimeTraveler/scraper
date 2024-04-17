//! Element references.

use std::fmt;
use std::iter::FusedIterator;
use std::ops::Deref;

use ego_tree::iter::{Edge, Traverse};
use ego_tree::NodeRef;
use html5ever::serialize::{serialize, SerializeOpts, TraversalScope};
use selectors::NthIndexCache;

use crate::node::Element;
use crate::{Node, Selector};

/// Wrapper around a reference to an element node.
///
/// This wrapper implements the `Element` trait from the `selectors` crate, which allows it to be
/// matched against CSS selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementRef<'a> {
    node: NodeRef<'a, Node>,
}

impl<'a> ElementRef<'a> {
    fn new(node: NodeRef<'a, Node>) -> Self {
        ElementRef { node }
    }

    /// Wraps a `NodeRef` only if it references a `Node::Element`.
    pub fn wrap(node: NodeRef<'a, Node>) -> Option<Self> {
        if node.value().is_element() {
            Some(ElementRef::new(node))
        } else {
            None
        }
    }

    /// Returns the `Element` referenced by `self`.
    pub fn value(&self) -> &'a Element {
        self.node.value().as_element().unwrap()
    }

    /// Returns an iterator over descendent elements matching a selector.
    pub fn select<'b>(&self, selector: &'b Selector) -> Select<'a, 'b> {
        let mut inner = self.traverse();
        inner.next(); // Skip Edge::Open(self).

        Select {
            scope: *self,
            inner,
            selector,
            nth_index_cache: NthIndexCache::default(),
        }
    }

    fn serialize(&self, traversal_scope: TraversalScope) -> String {
        let opts = SerializeOpts {
            scripting_enabled: false, // It's not clear what this does.
            traversal_scope,
            create_missing_parent: false,
        };
        let mut buf = Vec::new();
        serialize(&mut buf, self, opts).unwrap();
        String::from_utf8(buf).unwrap()
    }

    /// Returns the HTML of this element.
    pub fn html(&self) -> String {
        self.serialize(TraversalScope::IncludeNode)
    }

    /// Returns the inner HTML of this element.
    pub fn inner_html(&self) -> String {
        self.serialize(TraversalScope::ChildrenOnly(None))
    }

    /// Returns the value of an attribute.
    pub fn attr(&self, attr: &str) -> Option<&'a str> {
        self.value().attr(attr)
    }

    /// Returns an iterator over descendent text nodes.
    pub fn text(&self) -> Text<'a> {
        Text {
            inner: self.traverse(),
        }
    }

    /// Iterate over all child nodes which are elements
    ///
    /// # Example
    ///
    /// ```
    /// # use scraper::Html;
    /// let fragment = Html::parse_fragment("foo<span>bar</span><a>baz</a>qux");
    ///
    /// let children = fragment.root_element().child_elements().map(|element| element.value().name()).collect::<Vec<_>>();
    /// assert_eq!(children, ["span", "a"]);
    /// ```
    pub fn child_elements(&self) -> impl Iterator<Item = ElementRef<'a>> {
        self.children().filter_map(ElementRef::wrap)
    }

    /// Iterate over all descendent nodes which are elements
    ///
    /// # Example
    ///
    /// ```
    /// # use scraper::Html;
    /// let fragment = Html::parse_fragment("foo<span><b>bar</b></span><a><i>baz</i></a>qux");
    ///
    /// let descendants = fragment.root_element().descendent_elements().map(|element| element.value().name()).collect::<Vec<_>>();
    /// assert_eq!(descendants, ["html", "span", "b", "a", "i"]);
    /// ```
    pub fn descendent_elements(&self) -> impl Iterator<Item = ElementRef<'a>> {
        self.descendants().filter_map(ElementRef::wrap)
    }

    pub fn is_child_of(&self, parent: &ElementRef) -> bool {
        let mut maybe_parent = self.parent();
        while let Some(current_parent) = maybe_parent {
            if current_parent == **parent {
                return true;
            }
            maybe_parent = current_parent.parent();
        }
        false
    }
}

impl<'a> Deref for ElementRef<'a> {
    type Target = NodeRef<'a, Node>;
    fn deref(&self) -> &NodeRef<'a, Node> {
        &self.node
    }
}

/// Iterator over descendent elements matching a selector.
pub struct Select<'a, 'b> {
    scope: ElementRef<'a>,
    inner: Traverse<'a, Node>,
    selector: &'b Selector,
    nth_index_cache: NthIndexCache,
}

impl fmt::Debug for Select<'_, '_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Select")
            .field("scope", &self.scope)
            .field("inner", &self.inner)
            .field("selector", &self.selector)
            .field("nth_index_cache", &"..")
            .finish()
    }
}

impl Clone for Select<'_, '_> {
    fn clone(&self) -> Self {
        Self {
            scope: self.scope,
            inner: self.inner.clone(),
            selector: self.selector,
            nth_index_cache: NthIndexCache::default(),
        }
    }
}

impl<'a, 'b> Iterator for Select<'a, 'b> {
    type Item = ElementRef<'a>;

    fn next(&mut self) -> Option<ElementRef<'a>> {
        for edge in &mut self.inner {
            if let Edge::Open(node) = edge {
                if let Some(element) = ElementRef::wrap(node) {
                    if self.selector.matches_with_scope_and_cache(
                        &element,
                        Some(self.scope),
                        &mut self.nth_index_cache,
                    ) {
                        return Some(element);
                    }
                }
            }
        }
        None
    }
}

impl FusedIterator for Select<'_, '_> {}

/// Iterator over descendent text nodes.
#[derive(Debug, Clone)]
pub struct Text<'a> {
    inner: Traverse<'a, Node>,
}

impl<'a> Iterator for Text<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        for edge in &mut self.inner {
            if let Edge::Open(node) = edge {
                if let Node::Text(ref text) = node.value() {
                    return Some(&**text);
                }
            }
        }
        None
    }
}

impl FusedIterator for Text<'_> {}

mod element;
mod serializable;

#[cfg(test)]
mod tests {
    use crate::html::Html;
    use crate::selector::Selector;

    #[test]
    fn test_scope() {
        let html = r"
            <div>
                <b>1</b>
                <span>
                    <span><b>2</b></span>
                    <b>3</b>
                </span>
            </div>
        ";
        let fragment = Html::parse_fragment(html);
        let sel1 = Selector::parse("div > span").unwrap();
        let sel2 = Selector::parse(":scope > b").unwrap();

        let element1 = fragment.select(&sel1).next().unwrap();
        let element2 = element1.select(&sel2).next().unwrap();
        assert_eq!(element2.inner_html(), "3");
    }

    #[test]
    fn test_children(){
        let html = r"
            <form>
                <input>1</input>
            </form>
            <button>
                <span><b>2</b></span>
                <b>3</b>
            </button>
        ";

        let html = Html::parse_document(html);
        let forms = html.forms();
        let form = forms.get(0).unwrap();
        let input_sel = Selector::parse("input").unwrap();
        let input = html.select(&input_sel).next().unwrap();
        let button_sel = Selector::parse("button").unwrap();
        let button = html.select(&button_sel).next().unwrap();

        assert!(input.is_child_of(form));
        assert!(!button.is_child_of(form));
    }
}
