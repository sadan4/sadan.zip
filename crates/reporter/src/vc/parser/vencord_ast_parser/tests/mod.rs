use crate::vc::parser::vencord_ast_parser::VencordAstParser;
use insta::_macro_support::assert_snapshot;
use insta::{assert_ron_snapshot, assert_snapshot};
use oxc::allocator::Allocator;
use oxc::codegen::{Codegen, CodegenOptions, CommentOptions, IndentChar, LegalComment};

fn dump_ast(parser: &VencordAstParser<'_>) -> String {
    Codegen::new()
        .with_options(CodegenOptions {
            single_quote: true,
            minify: false,
            comments: CommentOptions {
                annotation: true,
                jsdoc: true,
                legal: LegalComment::Inline,
                normal: true,
            },
            indent_char: IndentChar::Tab,
            indent_width: 1,
            initial_indent: 0,
            source_map_path: None,
        })
        .build(&parser.prog)
        .code
}

#[test]
fn test_template_literal_replace() {
    let a = Allocator::new();
    let code = include_str!("data/imageZoom.tsx");
    let parser = VencordAstParser::try_new(&a, code).unwrap();
    let name = parser.plugin_name();
    assert_ron_snapshot!(name, @r#"Some("ImageZoom")"#);
    let patches = parser.patches().unwrap();
    assert_ron_snapshot!(patches);
}

#[test]
fn test_replace_simple_arrow_func() {
    let a = Allocator::new();
    let code = include_str!("data/commands.tsx");
    let parser = VencordAstParser::try_new(&a, code).unwrap();
    assert_ron_snapshot!(parser.plugin_name(), @r#"Some("CommandsAPI")"#);
    let patches = parser.patches().unwrap();
    assert_ron_snapshot!(patches);
}

#[test]
fn test_ignores_typeof() {
    let a = Allocator::new();
    let code = r#"
        const foo = {bar: "baz"};
        type TEST = typeof foo;
        console.log(foo);
    "#;
    let parser = VencordAstParser::try_new(&a, code).unwrap();
    assert_snapshot!(dump_ast(&parser), @"
        const foo = { bar: 'baz' };
        type TEST = typeof foo;
        console.log({ bar: 'baz' });
    ");
}

#[test]
fn test_inline_constants() {
    let a = Allocator::new();
    let code = r"
        const foo = 2;
        let bar = foo + 1;
        console.log(bar);
    ";
    let parser = VencordAstParser::try_new(&a, code).unwrap();
    assert_snapshot!(dump_ast(&parser), @"
        const foo = 2;
        let bar = 2 + 1;
        console.log(2 + 1);
    ");
}
