#pragma once
#include <libqalculate/BuiltinFunctions.h>
#include <libqalculate/Calculator.h>
#include <libqalculate/Function.h>
#include <memory>
#include <format>
#include "rust/cxx.h"

template <class Base> class DisabledFunction : public Base {
  std::string error_msg() const {
    const std::string name = this->name();
    return std::format("function {} is disabled", name);
  }

public:
  DisabledFunction() : Base() {}
  int calculate(MathStructure &, const MathStructure &,
                const EvaluationOptions &) override {
    const std::string msg = error_msg();
    throw std::runtime_error(msg);
  }
};
struct Qalculator {
  static std::unique_ptr<Qalculator> create();
  bool load_exchange_rates();
  bool load_global_defs();
  bool load_local_defs();
  void init_everything();
  void use_twos_complement_for_bin(bool use);
  bool get_twos_complement_for_bin() const;
  void use_twos_complement_for_hex(bool use);
  bool get_twos_complement_for_hex() const;
  void allow_impure_expressions(bool allow);
  bool get_allow_impure_expressions() const;
  void enable_sandboxing();
  void set_timeout_ms(int ms);
  int get_timeout_ms() const;
  rust::String calculate_and_print(rust::Str expr);

private:
  std::unique_ptr<Calculator> m_calculator;
  std::unique_ptr<DisabledFunction<ExportFunction>> m_disabled_export_function;
  std::unique_ptr<DisabledFunction<LoadFunction>> m_disabled_load_function;
  EvaluationOptions m_eval_opts = [] {
    EvaluationOptions eo = default_user_evaluation_options;
    eo.parse_options.twos_complement = true;
    return eo;
  }();
  PrintOptions m_print_opts = default_print_options;
  int m_timeout_ms = 10'000;
  bool m_allow_impure_expressions = false;
};


std::unique_ptr<Qalculator> create_qalculator();
