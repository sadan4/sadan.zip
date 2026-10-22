use insta::assert_debug_snapshot;

use super::test;

#[test]
async fn handles_mangled_css_module() {
	let p = parse!("test_data/wp/e.exports/classNames.js");
	let css = p.as_css_module().await.unwrap();
	assert_debug_snapshot!(css, @r#"
	CssModule {
	    class_names: [
	        ClassName {
	            module: ModuleId(
	                295575,
	            ),
	            exported_as: "XX",
	            span: Span {
	                start: 109,
	                end: 138,
	            },
	            value: "appAsidePanelWrapper_a3002d",
	        },
	        ClassName {
	            module: ModuleId(
	                295575,
	            ),
	            exported_as: "Xq",
	            span: Span {
	                start: 152,
	                end: 187,
	            },
	            value: "mobileAppAsidePanelWrapper_a3002d",
	        },
	        ClassName {
	            module: ModuleId(
	                295575,
	            ),
	            exported_as: "x$",
	            span: Span {
	                start: 201,
	                end: 226,
	            },
	            value: "notAppAsidePanel_a3002d",
	        },
	        ClassName {
	            module: ModuleId(
	                295575,
	            ),
	            exported_as: "yA",
	            span: Span {
	                start: 240,
	                end: 252,
	            },
	            value: "app_a3002d",
	        },
	        ClassName {
	            module: ModuleId(
	                295575,
	            ),
	            exported_as: "C$",
	            span: Span {
	                start: 266,
	                end: 284,
	            },
	            value: "mobileApp_a3002d",
	        },
	    ],
	}
	"#);
}

#[test]
async fn handles_unmangled_css_module() {
	let p = parse!("test_data/wp/e.exports/classNames2.js");
	let css = p.as_css_module().await.unwrap();
	assert_debug_snapshot!(css, @r#"
	CssModule {
	    class_names: [
	        ClassName {
	            module: ModuleId(
	                2753,
	            ),
	            exported_as: "container",
	            span: Span {
	                start: 119,
	                end: 137,
	            },
	            value: "container__29209",
	        },
	        ClassName {
	            module: ModuleId(
	                2753,
	            ),
	            exported_as: "content",
	            span: Span {
	                start: 156,
	                end: 172,
	            },
	            value: "content__29209",
	        },
	        ClassName {
	            module: ModuleId(
	                2753,
	            ),
	            exported_as: "cta",
	            span: Span {
	                start: 187,
	                end: 199,
	            },
	            value: "cta__29209",
	        },
	        ClassName {
	            module: ModuleId(
	                2753,
	            ),
	            exported_as: "artContainer",
	            span: Span {
	                start: 223,
	                end: 244,
	            },
	            value: "artContainer__29209",
	        },
	        ClassName {
	            module: ModuleId(
	                2753,
	            ),
	            exported_as: "small",
	            span: Span {
	                start: 261,
	                end: 275,
	            },
	            value: "small__29209",
	        },
	        ClassName {
	            module: ModuleId(
	                2753,
	            ),
	            exported_as: "art",
	            span: Span {
	                start: 290,
	                end: 302,
	            },
	            value: "art__29209",
	        },
	        ClassName {
	            module: ModuleId(
	                2753,
	            ),
	            exported_as: "medium",
	            span: Span {
	                start: 320,
	                end: 335,
	            },
	            value: "medium__29209",
	        },
	        ClassName {
	            module: ModuleId(
	                2753,
	            ),
	            exported_as: "large",
	            span: Span {
	                start: 352,
	                end: 366,
	            },
	            value: "large__29209",
	        },
	        ClassName {
	            module: ModuleId(
	                2753,
	            ),
	            exported_as: "header",
	            span: Span {
	                start: 384,
	                end: 399,
	            },
	            value: "header__29209",
	        },
	    ],
	}
	"#);
}
