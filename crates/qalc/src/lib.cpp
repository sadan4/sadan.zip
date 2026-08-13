#include "lib.hpp"
#include <libqalculate/util.h>
#include <memory>

std::unique_ptr<Qalculator> Qalculator::create() {
  auto ret = std::make_unique<Qalculator>();
  ret->m_calculator = std::make_unique<Calculator>();
  return ret;
}

bool Qalculator::load_exchange_rates() {
  auto ret = m_calculator->loadExchangeRates();
  return ret;
}

bool Qalculator::load_global_defs() {
  auto ret = m_calculator->loadGlobalDefinitions();
  return ret;
}

bool Qalculator::load_local_defs() {
  auto ret = m_calculator->loadLocalDefinitions();
  return ret;
}

void Qalculator::init_everything() {
  load_exchange_rates();
  load_global_defs();
  load_local_defs();
}

void Qalculator::use_twos_complement_for_bin(bool use) {
  m_eval_opts.parse_options.twos_complement = use;
}

bool Qalculator::get_twos_complement_for_bin() const {
  return m_eval_opts.parse_options.twos_complement;
}

void Qalculator::use_twos_complement_for_hex(bool use) {
  m_eval_opts.parse_options.hexadecimal_twos_complement = use;
}

bool Qalculator::get_twos_complement_for_hex() const {
  return m_eval_opts.parse_options.hexadecimal_twos_complement;
}

void Qalculator::allow_impure_expressions(bool allow) {
  m_allow_impure_expressions = allow;
}

bool Qalculator::get_allow_impure_expressions() const {
  return m_allow_impure_expressions;
}

void Qalculator::enable_sandboxing() {
  if (!m_disabled_export_function) {
    m_disabled_export_function =
        std::make_unique<DisabledFunction<ExportFunction>>();
    m_calculator->addFunction(m_disabled_export_function.get());
  }
  if (!m_disabled_load_function) {
    m_disabled_load_function =
        std::make_unique<DisabledFunction<LoadFunction>>();
    m_calculator->addFunction(m_disabled_load_function.get());
  }
  if (!m_disabled_command_function) {
    m_disabled_command_function =
        std::make_unique<DisabledFunction<CommandFunction>>();
    m_calculator->addFunction(m_disabled_command_function.get());
  }
  if (!m_disabled_plot_function) {
    m_disabled_plot_function =
        std::make_unique<DisabledFunction<PlotFunction>>();
    m_calculator->addFunction(m_disabled_plot_function.get());
  }
}

void Qalculator::set_timeout_ms(int ms) { m_timeout_ms = ms; }

int Qalculator::get_timeout_ms() const { return m_timeout_ms; }

rust::String Qalculator::calculate_and_print(rust::Str expr) {
  const std::string expr_str{expr};
  if (!m_allow_impure_expressions &&
      expression_contains_save_function(expr_str, m_eval_opts.parse_options)) {
    throw std::runtime_error("expression is not pure");
  }
  std::string result = m_calculator->calculateAndPrint(
      std::move(expr_str), m_timeout_ms, m_eval_opts, m_print_opts, nullptr);

  return rust::String(result);
}

rust::String Qalculator::get_package_data_dir() noexcept {
  return ::getPackageDataDir();
}