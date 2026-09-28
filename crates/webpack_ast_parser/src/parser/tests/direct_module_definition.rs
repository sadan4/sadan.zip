use super::*;
use async_trait::async_trait;
use std::collections::HashMap;
use url::Url;

struct TestModuleCache {
	paths: HashMap<ModuleId, Url>,
}

#[async_trait]
impl IModuleCache for TestModuleCache {
	async fn get_module_filepath(&self, id: ModuleId) -> Option<Url> {
		self.paths.get(&id).cloned()
	}
	async fn get_module_parser(
		&self,
		_requestor: &WebpackAstParser<'_>,
		_id: ModuleId,
		_latest: Option<bool>,
	) -> anyhow::Result<Arc<ThreadSafeParser>> {
		anyhow::bail!("test cache does not provide parsers")
	}
}

#[tokio::test]
async fn returns_definition_for_wreq_call_arg() {
	let alloc = Allocator::new();
	let source = include_str!("test_data/wp/module.js");
	let cache = TestModuleCache {
		paths: HashMap::from([(
			ModuleId(200651),
			Url::parse("file:///modules/200651.js").unwrap(),
		)]),
	};
	let mut p = WebpackAstParser::try_new(&alloc, source).unwrap();
	p.set_module_cache(Arc::new(cache));
	// pos 188 lies inside `200651` of `n(200651)` on line 11
	let defs = p
		.generate_definitions(188)
		.await
		.unwrap();
	assert_debug_snapshot!(defs, @r#"
	[
	    Definition {
	        location: Path(
	            Url {
	                scheme: "file",
	                cannot_be_a_base: false,
	                username: "",
	                password: None,
	                host: None,
	                port: None,
	                path: "/modules/200651.js",
	                query: None,
	                fragment: None,
	            },
	        ),
	        module_id: ModuleId(
	            200651,
	        ),
	        range: Span {
	            start: 0,
	            end: 0,
	        },
	    },
	]
	"#);
}

#[tokio::test]
async fn errors_when_module_cache_has_no_filepath() {
	let p = parse!("test_data/wp/module.js");
	let _ = p
		.generate_definitions(188)
		.await
		.unwrap_err();
}

#[tokio::test]
async fn errors_when_numeric_literal_parent_is_not_a_call() {
	let p = parse!("test_data/wp/module.js");
	let _ = p
		.generate_definitions(38)
		.await
		.unwrap_err();
}
