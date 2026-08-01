#[cxx::bridge]
#[allow(clippy::semicolon_outside_block)]
pub mod ffi {

	unsafe extern "C++" {
		include!("qalc/src/lib.hpp");
		type Qalculator;

		#[Self = "Qalculator"]
		fn create() -> UniquePtr<Qalculator>;

		/// underlying api takes int
		fn set_timeout_ms(self: Pin<&mut Qalculator>, timeout_ms: i32);
		fn get_timeout_ms(self: &Qalculator) -> i32;
		fn load_exchange_rates(self: Pin<&mut Qalculator>) -> bool;
		fn load_global_defs(self: Pin<&mut Qalculator>) -> bool;
		fn load_local_defs(self: Pin<&mut Qalculator>) -> bool;
		fn init_everything(self: Pin<&mut Qalculator>);
		fn use_twos_complement_for_bin(self: Pin<&mut Qalculator>, use_: bool);
		fn get_twos_complement_for_bin(self: &Qalculator) -> bool;
		fn use_twos_complement_for_hex(self: Pin<&mut Qalculator>, use_: bool);
		fn get_twos_complement_for_hex(self: &Qalculator) -> bool;
		fn allow_impure_expressions(self: Pin<&mut Qalculator>, allow: bool);
		fn enable_sandboxing(self: Pin<&mut Qalculator>);
		fn calculate_and_print(
			self: Pin<&mut Qalculator>,
			expr: &str,
		) -> Result<String>;
	}
}
