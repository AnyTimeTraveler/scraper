use std::ops::Deref;

use ahash::{HashMap, HashMapExt};

use crate::{ElementRef, Selector};

pub struct Form<'a> {
    root: ElementRef<'a>,
    form_element: ElementRef<'a>,
    values: HashMap<ElementRef<'a>, String>,
}

impl<'a> Form<'a> {
    pub(crate) fn wrap(root: ElementRef<'a>, form_element: ElementRef<'a>) -> Form<'a> {
        let mut map = HashMap::new();

        for input in root.select(&Selector::parse("button, fieldset, input, keygen, object, output, select, textarea").unwrap())
            .filter(|element| element.is_child_of(&form_element) || belongs_to_form(element, &form_element))
        {
            map.insert(input, get_value(input))
        }

        Form {
            root,
            form_element,
            values: map,
        }
    }

    pub(crate) fn inputs(&self) -> Vec<ElementRef> {
        self.root.select(&Selector::parse("button, fieldset, input, keygen, object, output, select, textarea").unwrap())
            .filter(|element| element.is_child_of(&self.form_element) || belongs_to_form(element, &self.form_element))
            .collect()
    }
}

fn get_value(element: ElementRef) -> Option<String> {
    match element.value().name() {
        "input" => {
            match element.attr("type") {
                None => None,
                Some("checkbox") | Some("radio") => element.attr("checked").map(str::to_string),
                Some("color") | Some("date") | Some("datetime-local") | Some("email") | Some("hidden") | Some("month") | Some("number") | Some("password") | Some("range") | Some("")=> element.attr("value").map(str::to_string),
                Some(_) => None,
            }
        }
        // "button" => element.attr("value").unwrap_or("").to_string(),
        "select" => find_selected_child(element).map(str::to_string),
        "datalist" => find_selected_child(element).map(str::to_string),
        "textarea" => Some(element.inner_html()),
        _ => unimplemented!("Tag not known"),
    }
}

fn find_selected_child(element: ElementRef) -> Option<&str> {
    for child in element.child_elements() {
        if child.attr("selected").is_some() {
            return child.attr("value");
        }
    }
    None
}

fn get_ids<'a>(element: &'a ElementRef, form: &'a ElementRef) -> Option<(&'a str, &'a str)> {
    let form_ref = element.attr("form")?;
    let id_ref = form.attr("id")?;
    Some((id_ref, form_ref))
}

fn belongs_to_form(element: &ElementRef, form: &ElementRef) -> bool {
    if let Some((id_ref, form_ref)) = get_ids(element, form) {
        id_ref == form_ref
    } else {
        false
    }
}

impl<'a> Deref for Form<'a> {
    type Target = ElementRef<'a>;
    fn deref(&self) -> &ElementRef<'a> {
        &self.form_element
    }
}

#[cfg(test)]
mod test {
    use crate::{Html, Selector};
    use crate::html::form::belongs_to_form;

    #[test]
    fn abc() {
        let html = r"
            <form>
            </form>
            <input>1</input>
            <button>
                <span><b>2</b></span>
                <b>3</b>
            </button>
        ";
        let html = Html::parse_document(html);
        let vec = html.forms();

        let i = vec.len();
        assert_eq!(i, 1, "expected to see one form");

        let form = vec.get(0).unwrap();
        for input in form.inputs() {
            println!("{:?}", input);
        }

        // println!("{:?}",form.inputs());
    }


    #[test]
    fn test_belongs_to_form() {
        let html = r#"
            <form id="a">
                <input>1</input>
            </form>
            <button>
                <span><b>2</b></span>
                <b>3</b>
            </button>
            <select name="abc" form="a">
                <option value="def">DEF</option>
            </select>
        "#;

        let html = Html::parse_document(html);
        let forms = html.forms();
        let form = forms.get(0).unwrap();
        let input_sel = Selector::parse("input").unwrap();
        let input = html.select(&input_sel).next().unwrap();
        let button_sel = Selector::parse("button").unwrap();
        let button = html.select(&button_sel).next().unwrap();
        let select_sel = Selector::parse("select").unwrap();
        let select = html.select(&select_sel).next().unwrap();

        assert!(input.is_child_of(form));
        assert!(!button.is_child_of(form));
        assert!(belongs_to_form(&select, form));

        let form_inputs = form.inputs();

        assert!(form_inputs.contains(&input));
        assert!(form_inputs.contains(&select));
    }
}
