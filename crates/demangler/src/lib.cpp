#include "lib.hpp"
#include <llvm/Demangle/Demangle.h>
#include <memory>
#include <string>

std::unique_ptr<std::string> demangle_all(rust::Str input) {
  if (input.empty()) {
    return nullptr;
  }

  std::string result{};
  // there are linker errors with the conversion function
  auto input_view = std::string_view{input.begin(), input.end()};

  if (llvm::nonMicrosoftDemangle(input_view, result)) {
    return std::make_unique<std::string>(result);
  }

  if (input_view[0] == '_' &&
      llvm::nonMicrosoftDemangle(input_view.substr(1), result, false)) {
    return std::make_unique<std::string>(result);
  }

  if (char *demangled = llvm::microsoftDemangle(input_view, nullptr, nullptr)) {
    auto *ret = new std::string(demangled);
    std::free(demangled);
    return std::unique_ptr<std::string>{ret};
  }

  return nullptr;
}
