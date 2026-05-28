use display_protocol::*;
use display_protocol_jsx::jsx;

#[test]
fn builds_nested_tree() {
    let node = jsx! {
        <Column gap={1}>
            <Text bold color={Color::YELLOW}>{"Title"}</Text>
            <Divider />
            <Row gap={2}>
                <Text>{"A"}</Text>
                <Text>{"B"}</Text>
            </Row>
        </Column>
    };

    match node {
        UiNode::Column(column) => {
            assert_eq!(column.gap, 1);
            assert_eq!(column.children.len(), 3);
        }
        _ => panic!("expected column"),
    }
}

#[test]
fn supports_input_and_expression_children() {
    let extra = UiNode::text("extra");
    let node = jsx! {
        <Column>
            <Input value={"hello"} placeholder="type" width={20} focused />
            {extra}
        </Column>
    };

    match node {
        UiNode::Column(column) => {
            assert_eq!(column.children.len(), 2);
            assert!(matches!(column.children[1], UiNode::Text(_)));
        }
        _ => panic!("expected column"),
    }
}

#[test]
fn supports_string_literal_children() {
    let node = jsx! {
        <Column>
            "plain"
            <Text>"styled"</Text>
        </Column>
    };

    match node {
        UiNode::Column(column) => {
            assert_eq!(column.children.len(), 2);
            assert_eq!(column.children[0].to_string(), "plain");
            assert_eq!(column.children[1].to_string(), "styled");
        }
        _ => panic!("expected column"),
    }
}

#[test]
fn supports_progress_props() {
    let node = jsx! {
        <Progress value={75.0} max={100.0} width={30} show_percent={false} />
    };

    match node {
        UiNode::ProgressBar(progress) => {
            assert_eq!(progress.value, 75.0);
            assert_eq!(progress.max, 100.0);
            assert_eq!(progress.width, 30);
            assert!(!progress.show_percent);
        }
        _ => panic!("expected progress bar"),
    }
}

#[test]
fn supports_divider_props() {
    let node = jsx! {
        <Divider char={'='} />
    };

    match node {
        UiNode::Divider(divider) => assert_eq!(divider.char, Some('=')),
        _ => panic!("expected divider"),
    }
}

#[test]
fn covers_all_ui_node_variants() {
    let cases = vec![
        jsx!(<Text style={Style::default().bold()} wrap={Wrap::NoWrap}>{"text"}</Text>),
        jsx!(<Box style={Style::default().reverse()} title="box" border width={10} height={3} min_width={5} min_height={2} max_width={20} max_height={6}><Text>{"child"}</Text></Box>),
        jsx!(<Row style={Style::default().dim()} gap={1} align={Align::Center} justify={Justify::End} flex_grow={1.0} flex_shrink={0.5}><Text>{"row"}</Text></Row>),
        jsx!(<Column gap={1}><Text>{"column"}</Text></Column>),
        jsx!(<Span style={Style::default().italic()}>{"span"}</Span>),
        jsx!(<List marker={ListMarker::Dash} style={Style::default().underline()}><ListItem><Text>{"item"}</Text></ListItem></List>),
        jsx!(<Divider char={'-'} style={Style::default().dim()} />),
        jsx!(<Progress value={1.0} max={2.0} width={8} />),
        jsx!(<Table headers={vec!["h".to_string()]} rows={vec![vec!["v".to_string()]]} border />),
        jsx!(<ScrollView scroll_top={1} height={4}><Text>{"scroll"}</Text></ScrollView>),
        jsx!(<Show when={true}><Text>{"show"}</Text></Show>),
        jsx!(<For><Text>{"for"}</Text></For>),
        jsx!(<Input value="input" placeholder="p" cursor={1} style={Style::default().bold()} cursor_style={Style::default().reverse()} focused />),
        jsx!(<TextArea lines={vec![StyledLine::plain("line")]} cursor_line={0} cursor_col={1} gutter focused />),
        jsx!(<TabBar items={vec![TabItem::new("tab").modified(true)]} active={0} />),
        jsx!(<TreeView items={vec![TreeItem::new("root").expanded(true)]} selected={0} indent={2} />),
        jsx!(<SplitPane orientation={Orientation::Vertical} ratio={0.25}><Text>{"a"}</Text><Text>{"b"}</Text></SplitPane>),
        jsx!(<StatusBar><Left><Text>{"left"}</Text></Left><Right><Text>{"right"}</Text></Right></StatusBar>),
        jsx!(<Canvas width={10} height={4} id="canvas" bg={Color::BLUE} />),
        jsx!(<Overlay x={1} y={2} z_index={3}><Text>{"overlay"}</Text></Overlay>),
        jsx!(<None />),
    ];

    assert!(matches!(cases[0], UiNode::Text(_)));
    assert!(matches!(cases[1], UiNode::Box(_)));
    assert!(matches!(cases[2], UiNode::Row(_)));
    assert!(matches!(cases[3], UiNode::Column(_)));
    assert!(matches!(cases[4], UiNode::Span(_)));
    assert!(matches!(cases[5], UiNode::List(_)));
    assert!(matches!(cases[6], UiNode::Divider(_)));
    assert!(matches!(cases[7], UiNode::ProgressBar(_)));
    assert!(matches!(cases[8], UiNode::Table(_)));
    assert!(matches!(cases[9], UiNode::ScrollView(_)));
    assert!(matches!(cases[10], UiNode::Show { .. }));
    assert!(matches!(cases[11], UiNode::For { .. }));
    assert!(matches!(cases[12], UiNode::Input(_)));
    assert!(matches!(cases[13], UiNode::TextArea(_)));
    assert!(matches!(cases[14], UiNode::TabBar(_)));
    assert!(matches!(cases[15], UiNode::TreeView(_)));
    assert!(matches!(cases[16], UiNode::SplitPane(_)));
    assert!(matches!(cases[17], UiNode::StatusBar(_)));
    assert!(matches!(cases[18], UiNode::Canvas(_)));
    assert!(matches!(cases[19], UiNode::Overlay(_)));
    assert!(matches!(cases[20], UiNode::None));
}
