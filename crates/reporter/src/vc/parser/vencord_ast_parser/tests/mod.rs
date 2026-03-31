use crate::vc::parser::vencord_ast_parser::VencordAstParser;
use insta::assert_ron_snapshot;
use oxc::allocator::Allocator;
use crate::vc::parser::vencord_ast_parser::pass::dump_ast;

macro_rules! dump_patches {
    ($path:literal, $dump_code:literal) => {{
        let a = Allocator::new();
        let code = include_str!($path);
        let parser = VencordAstParser::try_new(&a, code).unwrap();
        if $dump_code {
            let code = dump_ast(&parser.prog);
            eprintln!("{code}");
        }
        parser.patches().unwrap()
    }};
    ($path:literal) => {
        dump_patches!($path, false)
    };
    ($path:literal, dbg_code) => {
        dump_patches!($path, true)
    };
}

#[test]
fn test_template_literal_replace() {
    let patches = dump_patches!("data/plugin3.tsx");
    assert_ron_snapshot!(patches);
}

#[test]
fn test_replace_simple_arrow_func() {
    let patches = dump_patches!("data/plugin4.tsx");
    assert_ron_snapshot!(patches);
}

#[test]
fn test_replace_concat_template() {
    let patches = dump_patches!("data/plugin1.tsx");
    assert_ron_snapshot!(patches);
}

#[test]
fn test_replace_concat_template_with_ident() {
    let patches = dump_patches!("data/plugin2.tsx");
    assert_ron_snapshot!(patches);
}

#[test]
fn test_inline_big_int_literal_expr() {
    let patches = dump_patches!("data/plugin5.tsx");
    assert_ron_snapshot!(patches);
}

#[test]
#[ignore = "todo"]
fn test_array_map_in_replace() {
    let patches = dump_patches!("data/plugin6.tsx");
    assert_ron_snapshot!(patches);
}

#[test]
fn test_inline_string_raw() {
    let patches = dump_patches!("data/plugin7.tsx");
    assert_ron_snapshot!(patches);
}

#[test]
#[ignore = "todo"]
fn test_inline_typescript_enums() {
    let patches = dump_patches!("data/plugin8.tsx");
    assert_ron_snapshot!(patches);
}