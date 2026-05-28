use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{braced, Ident, LitStr, Result, Token};

/// Build a `display_protocol::UiNode` tree from a small JSX-like syntax.
///
/// This macro lowers directly to `display_protocol` constructors; it does not
/// expand through `rsx!`.
#[proc_macro]
pub fn jsx(input: TokenStream) -> TokenStream {
    match syn::parse::<JsxElement>(input) {
        Ok(element) => element.expand().into(),
        Err(err) => err.to_compile_error().into(),
    }
}

struct JsxElement {
    tag: Ident,
    attrs: Vec<JsxAttr>,
    children: Vec<JsxChild>,
}

struct JsxAttr {
    name: Ident,
    value: Option<JsxAttrValue>,
}

enum JsxAttrValue {
    Expr(syn::Expr),
    Str(LitStr),
}

enum JsxChild {
    Element(JsxElement),
    Expr(syn::Expr),
    Str(LitStr),
}

impl Parse for JsxElement {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        input.parse::<Token![<]>()?;
        let tag: Ident = input.parse()?;
        let attrs = parse_attrs(input)?;

        if input.peek(Token![/]) {
            input.parse::<Token![/]>()?;
            input.parse::<Token![>]>()?;
            return Ok(Self {
                tag,
                attrs,
                children: Vec::new(),
            });
        }

        input.parse::<Token![>]>()?;
        let mut children = Vec::new();
        while !is_closing_tag(input) {
            children.push(input.parse()?);
        }

        input.parse::<Token![<]>()?;
        input.parse::<Token![/]>()?;
        let close_tag: Ident = input.parse()?;
        if close_tag != tag {
            return Err(syn::Error::new_spanned(
                close_tag,
                format!("expected closing tag </{}>", tag),
            ));
        }
        input.parse::<Token![>]>()?;

        Ok(Self {
            tag,
            attrs,
            children,
        })
    }
}

impl Parse for JsxChild {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(Token![<]) {
            return input.parse().map(Self::Element);
        }

        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            return content.parse().map(Self::Expr);
        }

        input.parse().map(Self::Str)
    }
}

fn parse_attrs(input: ParseStream<'_>) -> Result<Vec<JsxAttr>> {
    let mut attrs = Vec::new();
    while !input.peek(Token![>]) && !input.peek(Token![/]) {
        let name: Ident = input.parse()?;
        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        attrs.push(JsxAttr { name, value });
    }
    Ok(attrs)
}

impl Parse for JsxAttrValue {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            return content.parse().map(Self::Expr);
        }

        input.parse().map(Self::Str)
    }
}

fn is_closing_tag(input: ParseStream<'_>) -> bool {
    let fork = input.fork();
    fork.parse::<Token![<]>().is_ok() && fork.parse::<Token![/]>().is_ok()
}

impl JsxElement {
    fn expand(&self) -> TokenStream2 {
        match self.tag.to_string().as_str() {
            "Text" => self.expand_text(),
            "Box" => self.expand_box(),
            "Row" => self.expand_flex(quote!(Row)),
            "Column" => self.expand_flex(quote!(Column)),
            "List" => self.expand_list(),
            "Divider" => self.expand_divider(),
            "Progress" | "ProgressBar" => self.expand_progress(),
            "Input" => self.expand_input(),
            "Span" => self.expand_span(),
            "ListItem" => self.expand_list_item(),
            "Table" => self.expand_table(),
            "ScrollView" => self.expand_scroll_view(),
            "Show" => self.expand_show(),
            "For" => self.expand_for(),
            "TextArea" => self.expand_text_area(),
            "TabBar" => self.expand_tab_bar(),
            "TreeView" => self.expand_tree_view(),
            "SplitPane" | "Split" => self.expand_split_pane(),
            "StatusBar" => self.expand_status_bar(),
            "Canvas" => self.expand_canvas(),
            "Overlay" => self.expand_overlay(),
            "None" => self.expand_none(),
            _ => syn::Error::new_spanned(&self.tag, "unsupported display-protocol JSX tag")
                .to_compile_error(),
        }
    }

    fn expand_text(&self) -> TokenStream2 {
        let mut child_pieces = Vec::new();
        for child in &self.children {
            match child {
                JsxChild::Str(lit) => child_pieces.push(quote! {
                    __content.push_str(#lit);
                }),
                JsxChild::Expr(expr) => child_pieces.push(quote! {
                    __content.push_str(&(#expr).to_string());
                }),
                JsxChild::Element(element) => {
                    return syn::Error::new_spanned(
                        &element.tag,
                        "Text does not support element children",
                    )
                    .to_compile_error();
                }
            }
        }

        let mut pieces = Vec::new();
        for attr in &self.attrs {
            let name = attr.name.to_string();
            match name.as_str() {
                "bold" => pieces.push(attr.set_style_flag(quote!(bold))),
                "italic" => pieces.push(attr.set_style_flag(quote!(italic))),
                "underline" => pieces.push(attr.set_style_flag(quote!(underline))),
                "dim" => pieces.push(attr.set_style_flag(quote!(dim))),
                "reverse" => pieces.push(attr.set_style_flag(quote!(reverse))),
                "strikethrough" => pieces.push(attr.set_style_flag(quote!(strikethrough))),
                "color" => pieces.push(attr.set_text_field(quote!(style.fg), true)),
                "bg" => pieces.push(attr.set_text_field(quote!(style.bg), true)),
                "style" => pieces.push(attr.set_text_field(quote!(style), false)),
                "wrap" => pieces.push(attr.set_text_field(quote!(wrap), false)),
                _ => {
                    return syn::Error::new_spanned(
                        &attr.name,
                        format!("unsupported prop `{}` on <{}>", name, self.tag),
                    )
                    .to_compile_error();
                }
            }
        }

        collect_results(pieces).map_or_else(syn::Error::into_compile_error, |pieces| {
            quote! {{
                let mut __content = String::new();
                #(#child_pieces)*
                let mut __text = ::display_protocol::TextNode {
                    content: __content,
                    style: ::display_protocol::Style::default(),
                    wrap: ::display_protocol::Wrap::default(),
                };
                #(#pieces)*
                ::display_protocol::UiNode::Text(__text)
            }}
        })
    }

    fn expand_span(&self) -> TokenStream2 {
        let Some(content) = self.text_content() else {
            return syn::Error::new_spanned(
                &self.tag,
                "Span only supports text/expression children",
            )
            .to_compile_error();
        };
        let style = self
            .attr_expr("style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        if let Err(err) = self.reject_attrs(&["style"]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::Span(::display_protocol::SpanNode {
            content: #content,
            style: #style,
        }))
    }

    fn expand_box(&self) -> TokenStream2 {
        let children = self.children.iter().map(JsxChild::expand_node);
        let style = self
            .attr_expr("style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        let padding = self
            .attr_expr("padding")
            .unwrap_or_else(|| quote!(::display_protocol::Padding::ZERO));
        let border = self.border_attr();
        let title = self.string_option_attr("title");
        let width = optional_attr(self.attr_expr("width"));
        let height = optional_attr(self.attr_expr("height"));
        let min_width = optional_attr(self.attr_expr("min_width"));
        let min_height = optional_attr(self.attr_expr("min_height"));
        let max_width = optional_attr(self.attr_expr("max_width"));
        let max_height = optional_attr(self.attr_expr("max_height"));
        if let Err(err) = self.reject_attrs(&[
            "style",
            "padding",
            "border",
            "title",
            "width",
            "height",
            "min_width",
            "min_height",
            "max_width",
            "max_height",
        ]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::Box(::display_protocol::BoxNode {
            children: vec![#(#children),*],
            style: #style,
            padding: #padding,
            border: #border,
            title: #title,
            width: #width,
            height: #height,
            min_width: #min_width,
            min_height: #min_height,
            max_width: #max_width,
            max_height: #max_height,
        }))
    }

    fn expand_flex(&self, variant: TokenStream2) -> TokenStream2 {
        let children = self.children.iter().map(JsxChild::expand_node);
        let style = self
            .attr_expr("style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        let gap = self.attr_expr("gap").unwrap_or_else(|| quote!(0));
        let align = self
            .attr_expr("align")
            .unwrap_or_else(|| quote!(::display_protocol::Align::default()));
        let justify = self
            .attr_expr("justify")
            .unwrap_or_else(|| quote!(::display_protocol::Justify::default()));
        let padding = self
            .attr_expr("padding")
            .unwrap_or_else(|| quote!(::display_protocol::Padding::ZERO));
        let width = optional_attr(self.attr_expr("width"));
        let height = optional_attr(self.attr_expr("height"));
        let flex_grow = self.attr_expr("flex_grow").unwrap_or_else(|| quote!(0.0));
        let flex_shrink = self.attr_expr("flex_shrink").unwrap_or_else(|| quote!(1.0));
        if let Err(err) = self.reject_attrs(&[
            "style",
            "gap",
            "align",
            "justify",
            "padding",
            "width",
            "height",
            "flex_grow",
            "flex_shrink",
        ]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::#variant(::display_protocol::FlexNode {
            children: vec![#(#children),*],
            style: #style,
            gap: #gap,
            align: #align,
            justify: #justify,
            padding: #padding,
            width: #width,
            height: #height,
            flex_grow: #flex_grow,
            flex_shrink: #flex_shrink,
        }))
    }

    fn expand_list(&self) -> TokenStream2 {
        let items = self.children.iter().map(JsxChild::expand_node);
        let marker = self
            .attr_expr("marker")
            .unwrap_or_else(|| quote!(::display_protocol::ListMarker::Bullet));
        let style = self
            .attr_expr("style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        if let Err(err) = self.reject_attrs(&["marker", "style"]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::List(::display_protocol::ListNode {
            items: vec![#(#items),*],
            marker: #marker,
            style: #style,
        }))
    }

    fn expand_divider(&self) -> TokenStream2 {
        if let Some(child) = self.children.first() {
            return child.unsupported_here("self-closing tag does not support children");
        }

        let mut pieces = Vec::new();
        for attr in &self.attrs {
            match attr.name.to_string().as_str() {
                "char" => {
                    let Some(value) = attr.value_tokens() else {
                        return syn::Error::new_spanned(&attr.name, "prop requires a value")
                            .to_compile_error();
                    };
                    pieces.push(quote! {
                        if let ::display_protocol::UiNode::Divider(ref mut __divider) = __node {
                            __divider.char = Some(#value);
                        }
                    });
                }
                "style" => {
                    let Some(value) = attr.value_tokens() else {
                        return syn::Error::new_spanned(&attr.name, "prop requires a value")
                            .to_compile_error();
                    };
                    pieces.push(quote! {
                        if let ::display_protocol::UiNode::Divider(ref mut __divider) = __node {
                            __divider.style = #value;
                        }
                    });
                }
                name => {
                    return syn::Error::new_spanned(
                        &attr.name,
                        format!("unsupported prop `{}` on <{}>", name, self.tag),
                    )
                    .to_compile_error();
                }
            }
        }

        quote! {{
            let mut __node = ::display_protocol::UiNode::divider();
            #(#pieces)*
            __node
        }}
    }

    fn expand_progress(&self) -> TokenStream2 {
        if let Some(child) = self.children.first() {
            return child.unsupported_here("Progress does not support children");
        }

        let value = self.attr_expr("value").unwrap_or_else(|| quote!(0.0));
        let max = self.attr_expr("max").unwrap_or_else(|| quote!(100.0));
        let base = quote!(::display_protocol::UiNode::progress(#value, #max));
        self.apply_attrs(
            base,
            &["width", "filled_style", "empty_style", "show_percent"],
        )
    }

    fn expand_input(&self) -> TokenStream2 {
        if let Some(child) = self.children.first() {
            return child.unsupported_here("Input does not support children");
        }

        let value = self.attr_expr("value").unwrap_or_else(|| quote!(""));
        let placeholder = self.attr_expr("placeholder").unwrap_or_else(|| quote!(""));
        let cursor = self.attr_expr("cursor").unwrap_or_else(|| quote!(0));
        let style = self
            .attr_expr("style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        let cursor_style = self
            .attr_expr("cursor_style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        let width = optional_attr(self.attr_expr("width"));
        let focused = self.bool_attr("focused", false);
        if let Err(err) = self.reject_attrs(&[
            "value",
            "placeholder",
            "cursor",
            "style",
            "cursor_style",
            "width",
            "focused",
        ]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::Input(::display_protocol::InputNode {
            value: (#value).into(),
            placeholder: (#placeholder).into(),
            cursor: #cursor,
            style: #style,
            cursor_style: #cursor_style,
            width: #width,
            focused: #focused,
        }))
    }

    fn expand_list_item(&self) -> TokenStream2 {
        let child = match self.single_child("ListItem requires exactly one child") {
            Ok(child) => child,
            Err(err) => return err.to_compile_error(),
        };
        if let Err(err) = self.reject_attrs(&[]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::ListItem(Box::new(#child)))
    }

    fn expand_table(&self) -> TokenStream2 {
        if let Some(child) = self.children.first() {
            return child.unsupported_here("Table does not support children");
        }
        let headers = self
            .attr_expr("headers")
            .unwrap_or_else(|| quote!(Vec::new()));
        let rows = self.attr_expr("rows").unwrap_or_else(|| quote!(Vec::new()));
        let style = self
            .attr_expr("style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        let header_style = self
            .attr_expr("header_style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default().bold()));
        let border = self.bool_attr("border", false);
        if let Err(err) = self.reject_attrs(&["headers", "rows", "style", "header_style", "border"])
        {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::Table(::display_protocol::TableNode {
            headers: #headers,
            rows: #rows,
            style: #style,
            header_style: #header_style,
            border: #border,
        }))
    }

    fn expand_scroll_view(&self) -> TokenStream2 {
        let child = match self.single_child("ScrollView requires exactly one child") {
            Ok(child) => child,
            Err(err) => return err.to_compile_error(),
        };
        let scroll_top = self.attr_expr("scroll_top").unwrap_or_else(|| quote!(0));
        let height = self.attr_expr("height").unwrap_or_else(|| quote!(10));
        if let Err(err) = self.reject_attrs(&["scroll_top", "height"]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::ScrollView(::display_protocol::ScrollNode {
            child: Box::new(#child),
            scroll_top: #scroll_top,
            height: #height,
        }))
    }

    fn expand_show(&self) -> TokenStream2 {
        let child = match self.single_child("Show requires exactly one child") {
            Ok(child) => child,
            Err(err) => return err.to_compile_error(),
        };
        let when = self.bool_attr("when", true);
        if let Err(err) = self.reject_attrs(&["when"]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::show(#when, #child))
    }

    fn expand_for(&self) -> TokenStream2 {
        if let Some(children) = self.attr_expr("children") {
            if let Err(err) = self.reject_attrs(&["children"]) {
                return err.to_compile_error();
            }
            return quote!(::display_protocol::UiNode::For { children: #children });
        }

        let children = self.children.iter().map(JsxChild::expand_node);
        if let Err(err) = self.reject_attrs(&[]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::For {
            children: vec![#(#children),*]
        })
    }

    fn expand_text_area(&self) -> TokenStream2 {
        if let Some(child) = self.children.first() {
            return child.unsupported_here("TextArea does not support children");
        }
        let lines = self
            .attr_expr("lines")
            .unwrap_or_else(|| quote!(Vec::new()));
        let cursor_line = self.attr_expr("cursor_line").unwrap_or_else(|| quote!(0));
        let cursor_col = self.attr_expr("cursor_col").unwrap_or_else(|| quote!(0));
        let selection = optional_attr(self.attr_expr("selection"));
        let scroll_top = self.attr_expr("scroll_top").unwrap_or_else(|| quote!(0));
        let scroll_left = self.attr_expr("scroll_left").unwrap_or_else(|| quote!(0));
        let height = self.attr_expr("height").unwrap_or_else(|| quote!(10));
        let style = self
            .attr_expr("style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        let cursor_style = self
            .attr_expr("cursor_style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        let selection_style = self.attr_expr("selection_style").unwrap_or_else(|| {
            quote!(::display_protocol::Style::default()
                .bg(::display_protocol::Color::new(50, 100, 180)))
        });
        let gutter = self.bool_attr("gutter", false);
        let focused = self.bool_attr("focused", false);
        if let Err(err) = self.reject_attrs(&[
            "lines",
            "cursor_line",
            "cursor_col",
            "selection",
            "scroll_top",
            "scroll_left",
            "height",
            "style",
            "cursor_style",
            "selection_style",
            "gutter",
            "focused",
        ]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::TextArea(::display_protocol::TextAreaNode {
            lines: #lines,
            cursor_line: #cursor_line,
            cursor_col: #cursor_col,
            selection: #selection,
            scroll_top: #scroll_top,
            scroll_left: #scroll_left,
            height: #height,
            style: #style,
            cursor_style: #cursor_style,
            selection_style: #selection_style,
            gutter: #gutter,
            focused: #focused,
        }))
    }

    fn expand_tab_bar(&self) -> TokenStream2 {
        if let Some(child) = self.children.first() {
            return child.unsupported_here("TabBar does not support children");
        }
        let items = self
            .attr_expr("items")
            .unwrap_or_else(|| quote!(Vec::new()));
        let active = self.attr_expr("active").unwrap_or_else(|| quote!(0));
        let style = self
            .attr_expr("style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        let active_style = self
            .attr_expr("active_style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default().underline()));
        if let Err(err) = self.reject_attrs(&["items", "active", "style", "active_style"]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::TabBar(::display_protocol::TabBarNode {
            items: #items,
            active: #active,
            style: #style,
            active_style: #active_style,
        }))
    }

    fn expand_tree_view(&self) -> TokenStream2 {
        if let Some(child) = self.children.first() {
            return child.unsupported_here("TreeView does not support children");
        }
        let items = self
            .attr_expr("items")
            .unwrap_or_else(|| quote!(Vec::new()));
        let selected = optional_attr(self.attr_expr("selected"));
        let style = self
            .attr_expr("style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        let selected_style = self
            .attr_expr("selected_style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default().reverse()));
        let indent = self.attr_expr("indent").unwrap_or_else(|| quote!(2));
        if let Err(err) =
            self.reject_attrs(&["items", "selected", "style", "selected_style", "indent"])
        {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::TreeView(::display_protocol::TreeViewNode {
            items: #items,
            selected: #selected,
            style: #style,
            selected_style: #selected_style,
            indent: #indent,
        }))
    }

    fn expand_split_pane(&self) -> TokenStream2 {
        if self.children.len() != 2 {
            return syn::Error::new_spanned(&self.tag, "SplitPane requires exactly two children")
                .to_compile_error();
        }
        let first = self.children[0].expand_node();
        let second = self.children[1].expand_node();
        let orientation = self
            .attr_expr("orientation")
            .unwrap_or_else(|| quote!(::display_protocol::Orientation::Horizontal));
        let ratio = self.attr_expr("ratio").unwrap_or_else(|| quote!(0.5));
        let divider_style = self
            .attr_expr("divider_style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        if let Err(err) = self.reject_attrs(&["orientation", "ratio", "divider_style"]) {
            return err.to_compile_error();
        }
        quote! {{
            let __ratio: f32 = #ratio;
            ::display_protocol::UiNode::SplitPane(::display_protocol::SplitPaneNode {
                orientation: #orientation,
                ratio: __ratio.clamp(0.0, 1.0),
                first: Box::new(#first),
                second: Box::new(#second),
                divider_style: #divider_style,
            })
        }}
    }

    fn expand_status_bar(&self) -> TokenStream2 {
        let mut left = None;
        let mut right = None;
        for child in &self.children {
            match child {
                JsxChild::Element(element) if element.tag == "Left" => {
                    left = Some(
                        element
                            .children
                            .iter()
                            .map(JsxChild::expand_node)
                            .collect::<Vec<_>>(),
                    );
                    if let Err(err) = element.reject_attrs(&[]) {
                        return err.to_compile_error();
                    }
                }
                JsxChild::Element(element) if element.tag == "Right" => {
                    right = Some(
                        element
                            .children
                            .iter()
                            .map(JsxChild::expand_node)
                            .collect::<Vec<_>>(),
                    );
                    if let Err(err) = element.reject_attrs(&[]) {
                        return err.to_compile_error();
                    }
                }
                _ => {
                    return child
                        .unsupported_here("StatusBar only supports <Left> and <Right> children")
                }
            }
        }
        let left = left.unwrap_or_default();
        let right = right.unwrap_or_default();
        let style = self
            .attr_expr("style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        if let Err(err) = self.reject_attrs(&["style"]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::StatusBar(::display_protocol::StatusBarNode {
            left: vec![#(#left),*],
            right: vec![#(#right),*],
            style: #style,
        }))
    }

    fn expand_canvas(&self) -> TokenStream2 {
        if let Some(child) = self.children.first() {
            return child.unsupported_here("Canvas does not support children");
        }
        let width = match self.attr_expr("width") {
            Some(value) => value,
            None => return self.missing_attr("width"),
        };
        let height = match self.attr_expr("height") {
            Some(value) => value,
            None => return self.missing_attr("height"),
        };
        let frame_id = match self.attr_expr("id").or_else(|| self.attr_expr("frame_id")) {
            Some(value) => value,
            None => return self.missing_attr("id"),
        };
        let bg = self
            .attr_expr("bg")
            .unwrap_or_else(|| quote!(::display_protocol::Color::BLACK));
        if let Err(err) = self.reject_attrs(&["width", "height", "id", "frame_id", "bg"]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::Canvas(::display_protocol::CanvasNode {
            width: #width,
            height: #height,
            frame_id: (#frame_id).into(),
            bg: #bg,
        }))
    }

    fn expand_overlay(&self) -> TokenStream2 {
        let child = match self.single_child("Overlay requires exactly one child") {
            Ok(child) => child,
            Err(err) => return err.to_compile_error(),
        };
        let x = self.attr_expr("x").unwrap_or_else(|| quote!(0));
        let y = self.attr_expr("y").unwrap_or_else(|| quote!(0));
        let z_index = self.attr_expr("z_index").unwrap_or_else(|| quote!(0));
        let style = self
            .attr_expr("style")
            .unwrap_or_else(|| quote!(::display_protocol::Style::default()));
        if let Err(err) = self.reject_attrs(&["x", "y", "z_index", "style"]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::Overlay(::display_protocol::OverlayNode {
            child: Box::new(#child),
            x: #x,
            y: #y,
            z_index: #z_index,
            style: #style,
        }))
    }

    fn expand_none(&self) -> TokenStream2 {
        if let Some(child) = self.children.first() {
            return child.unsupported_here("None does not support children");
        }
        if let Err(err) = self.reject_attrs(&[]) {
            return err.to_compile_error();
        }
        quote!(::display_protocol::UiNode::None)
    }

    fn apply_attrs(&self, mut base: TokenStream2, allowed: &[&str]) -> TokenStream2 {
        for attr in &self.attrs {
            let name = attr.name.to_string();
            if !allowed.contains(&name.as_str()) {
                if name == "value" || name == "max" {
                    continue;
                }
                return syn::Error::new_spanned(
                    &attr.name,
                    format!("unsupported prop `{}` on <{}>", name, self.tag),
                )
                .to_compile_error();
            }

            let next = match name.as_str() {
                "bold" => attr.boolean_method(base, quote!(bold)),
                "italic" => attr.boolean_method(base, quote!(italic)),
                "underline" => attr.boolean_method(base, quote!(underline)),
                "dim" => attr.boolean_method(base, quote!(dim)),
                "reverse" => attr.boolean_method(base, quote!(reverse)),
                "strikethrough" => attr.boolean_method(base, quote!(strikethrough)),
                "focused" => attr.bool_arg_method(base, quote!(focused)),
                "border" => attr.border_method(base),
                "cursor" => attr.expr_method(base, quote!(cursor_pos)),
                _ => attr.expr_method(base, Ident::new(&name, Span::call_site())),
            };

            match next {
                Ok(expanded) => base = expanded,
                Err(err) => return err.to_compile_error(),
            }
        }
        base
    }

    fn attr_expr(&self, name: &str) -> Option<TokenStream2> {
        self.attrs
            .iter()
            .find(|attr| attr.name == name)
            .and_then(JsxAttr::value_tokens)
    }

    fn bool_attr(&self, name: &str, default: bool) -> TokenStream2 {
        self.attrs
            .iter()
            .find(|attr| attr.name == name)
            .map(|attr| attr.value_tokens().unwrap_or_else(|| quote!(true)))
            .unwrap_or_else(|| if default { quote!(true) } else { quote!(false) })
    }

    fn border_attr(&self) -> TokenStream2 {
        self.attrs
            .iter()
            .find(|attr| attr.name == "border")
            .map(|attr| {
                attr.value_tokens()
                    .unwrap_or_else(|| quote!(::display_protocol::Border::all(None)))
            })
            .unwrap_or_else(|| quote!(::display_protocol::Border::NONE))
    }

    fn string_option_attr(&self, name: &str) -> TokenStream2 {
        self.attr_expr(name)
            .map(|value| quote!(Some((#value).into())))
            .unwrap_or_else(|| quote!(None))
    }

    fn reject_attrs(&self, allowed: &[&str]) -> Result<()> {
        for attr in &self.attrs {
            let name = attr.name.to_string();
            if !allowed.contains(&name.as_str()) {
                return Err(syn::Error::new_spanned(
                    &attr.name,
                    format!("unsupported prop `{}` on <{}>", name, self.tag),
                ));
            }
        }
        Ok(())
    }

    fn single_child(&self, message: &str) -> Result<TokenStream2> {
        if self.children.len() != 1 {
            return Err(syn::Error::new_spanned(&self.tag, message));
        }
        Ok(self.children[0].expand_node())
    }

    fn text_content(&self) -> Option<TokenStream2> {
        let mut child_pieces = Vec::new();
        for child in &self.children {
            match child {
                JsxChild::Str(lit) => child_pieces.push(quote! {
                    __content.push_str(#lit);
                }),
                JsxChild::Expr(expr) => child_pieces.push(quote! {
                    __content.push_str(&(#expr).to_string());
                }),
                JsxChild::Element(_) => return None,
            }
        }
        Some(quote! {{
            let mut __content = String::new();
            #(#child_pieces)*
            __content
        }})
    }

    fn missing_attr(&self, name: &str) -> TokenStream2 {
        syn::Error::new_spanned(
            &self.tag,
            format!("<{}> requires `{}` prop", self.tag, name),
        )
        .to_compile_error()
    }
}

impl JsxChild {
    fn expand_node(&self) -> TokenStream2 {
        match self {
            JsxChild::Element(element) => element.expand(),
            JsxChild::Expr(expr) => quote!(#expr),
            JsxChild::Str(lit) => quote!(::display_protocol::UiNode::text(#lit)),
        }
    }

    fn unsupported_here(&self, message: &str) -> TokenStream2 {
        match self {
            JsxChild::Element(element) => syn::Error::new_spanned(&element.tag, message),
            JsxChild::Expr(expr) => syn::Error::new_spanned(expr, message),
            JsxChild::Str(lit) => syn::Error::new_spanned(lit, message),
        }
        .to_compile_error()
    }
}

impl JsxAttr {
    fn value_tokens(&self) -> Option<TokenStream2> {
        self.value.as_ref().map(|value| match value {
            JsxAttrValue::Expr(expr) => quote!(#expr),
            JsxAttrValue::Str(lit) => quote!(#lit),
        })
    }

    fn expr_method(
        &self,
        base: TokenStream2,
        method: impl quote::ToTokens,
    ) -> Result<TokenStream2> {
        let value = self
            .value_tokens()
            .ok_or_else(|| syn::Error::new_spanned(&self.name, "prop requires a value"))?;
        Ok(quote!(#base.#method(#value)))
    }

    fn boolean_method(
        &self,
        base: TokenStream2,
        method: impl quote::ToTokens,
    ) -> Result<TokenStream2> {
        if self.value.is_some() {
            return Err(syn::Error::new_spanned(
                &self.name,
                "style boolean prop does not accept a value",
            ));
        }
        Ok(quote!(#base.#method()))
    }

    fn bool_arg_method(
        &self,
        base: TokenStream2,
        method: impl quote::ToTokens,
    ) -> Result<TokenStream2> {
        let value = self.value_tokens().unwrap_or_else(|| quote!(true));
        Ok(quote!(#base.#method(#value)))
    }

    fn border_method(&self, base: TokenStream2) -> Result<TokenStream2> {
        let value = self
            .value_tokens()
            .unwrap_or_else(|| quote!(::display_protocol::Border::all(None)));
        Ok(quote!(#base.border(#value)))
    }

    fn set_style_flag(&self, flag: impl quote::ToTokens) -> Result<TokenStream2> {
        if self.value.is_some() {
            return Err(syn::Error::new_spanned(
                &self.name,
                "style boolean prop does not accept a value",
            ));
        }
        Ok(quote! {
            __text.style = __text.style.#flag();
        })
    }

    fn set_text_field(&self, field: impl quote::ToTokens, wrap_some: bool) -> Result<TokenStream2> {
        let value = self
            .value_tokens()
            .ok_or_else(|| syn::Error::new_spanned(&self.name, "prop requires a value"))?;
        if wrap_some {
            Ok(quote! {
                __text.#field = Some(#value);
            })
        } else {
            Ok(quote! {
                __text.#field = #value;
            })
        }
    }
}

fn optional_attr(value: Option<TokenStream2>) -> TokenStream2 {
    value
        .map(|value| quote!(Some(#value)))
        .unwrap_or_else(|| quote!(None))
}

fn collect_results(values: Vec<Result<TokenStream2>>) -> Result<Vec<TokenStream2>> {
    values.into_iter().collect()
}
