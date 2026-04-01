use std::collections::HashMap;

use oxc::ast::ast::ObjectExpression;

use crate::{Sealed, types::ModuleEntry};

pub(crate) trait WebpackChunkParserImpl<'ast>: Sealed {
	/// the object with each module defined, should conform to `Record<PropertyKey, (e, t, n) => void)>`
	fn get_module_object(&self) -> Option<&'ast ObjectExpression<'ast>>;
	/// ```js
	///  let __webpack_modules__ = {
	///      123: function(module, exports, require) {
	///          // module
	///      }
	///  };
	/// ```
	fn try_parse_chunk_entry_property_assignment(&self) -> Option<ModuleEntry> {
		todo!()
	}
	/// ```js
	/// let __webpack_modules__ = {
	///     123(module, exports, require) {
	///         // module
	///     }
	/// };
	/// ```
	fn try_parse_chunk_entry_method_decl(&self) -> Option<ModuleEntry> {
		todo!()
	}
	fn try_parse_chunk_entry(&self) -> Option<ModuleEntry> {
		todo!()
	}
}

pub trait WebpackChunkParser<'ast> {
	fn get_defined_modules(&self) -> Option<HashMap<u32, String>>;
}

impl<'ast, T: WebpackChunkParserImpl<'ast>> WebpackChunkParser<'ast> for T {
	fn get_defined_modules(&self) -> Option<HashMap<u32, String>> {
		todo!()
	}
}
